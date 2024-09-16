#include <llvm/Pass.h>
#include <llvm/IR/Function.h>
#include <llvm/IR/PassManager.h>
#include <llvm/IR/Dominators.h>
#include <llvm/IR/BasicBlock.h>
#include <llvm/IR/CFG.h>
#include <llvm/IR/Intrinsics.h>
#include <llvm/IR/IntrinsicsAArch64.h>
#include <llvm/Passes/PassBuilder.h>
#include <llvm/Passes/PassPlugin.h>
#include <llvm/Support/Debug.h>
#include <llvm/Support/Casting.h>
#include <llvm/Support/raw_ostream.h>
#include <llvm/Support/CommandLine.h>
#include <llvm/Support/WithColor.h>
#include <llvm/Analysis/TargetTransformInfo.h>
#include <llvm/Analysis/AliasAnalysis.h>
#include <llvm/Analysis/DependenceAnalysis.h>
#include <llvm/Analysis/LoopAnalysisManager.h>
#include <llvm/Analysis/LoopInfo.h>
#include <llvm/Transforms/IPO/PassManagerBuilder.h>
#include <llvm/ADT/Statistic.h>
#include <llvm/ADT/SmallVector.h>
#include <llvm/ADT/SetVector.h>

#include <string>

using namespace llvm;

// There is definitely a better way of doing this, using the dominator tree
// or so, but if a value created inside the vectorized loop is used after
// the loop was left (other than the induction variable), that is a problem!
static bool valueUsedOutside(const BasicBlock *BB, const Value *Val) {
	for (auto I = Val->user_begin(), IEnd = Val->user_end(); I != IEnd; ++I)
		if (const Instruction *Inst = dyn_cast<Instruction>(*I))
			if (Inst->getParent() != BB)
				return true;
	return false;
}

static bool instructionsCanBeVectorized(const BasicBlock *BB) {
	for (auto I = BB->begin(); I != BB->end(); ++I) {
		if (!I->getType()->isFloatTy() && !I->getType()->isIntegerTy(64)
				&& !I->getType()->isPointerTy())
			return false;

		switch (I->getOpcode()) {
		case Instruction::GetElementPtr:
		case Instruction::Load:
		case Instruction::FAdd:
		case Instruction::FSub:
		case Instruction::FMul:
		case Instruction::ICmp:
			return !valueUsedOutside(BB, &*I);
		case Instruction::Store:
		case Instruction::Add:
		case Instruction::PHI:
		case Instruction::Br:
			return true;
		default:
			WithColor::warning() << "instruction cannot be vectorized: \n";
			I->dump();
			return false;
		}
	}
	return true;
}

// Very very very loosely inspired by VPlan.
struct SVNode {
	// LLVM uses it's own RTTI, so play by its rules:
	enum SVNodeKind {
		Load,
		Store,
		FloatBinOp
	} Kind;
	SVNodeKind getKind() const { return Kind; }

	SVNode(SVNodeKind Kind): Kind(Kind) {}
	virtual ~SVNode() {}

	Value *ReturnValue = nullptr;
	SmallVector<SVNode*, 2> Operands;
	SmallSetVector<SVNode*, 2> UsedBy;

	void AddOperand(SVNode *Node) {
		this->Operands.push_back(Node);
		Node->UsedBy.insert(this);
	}
};

struct SVLoad: public SVNode {
	SVLoad(LoadInst *LI):
		SVNode(SVNode::Load), OrigInst(LI),
		BasePtr(cast<GetElementPtrInst>(LI->getOperand(0))->getOperand(0)) {}
	static bool classof(const SVNode *N) {
		return N->getKind() == SVNode::Load; }

	LoadInst *OrigInst = nullptr;
	Value *BasePtr = nullptr;
};

struct SVStore: public SVNode {
	SVStore(StoreInst *SI):
		SVNode(SVLoad::Store), OrigInst(SI),
		BasePtr(cast<GetElementPtrInst>(SI->getOperand(1))->getOperand(0)) {}
	static bool classof(const SVNode *N) {
		return N->getKind() == SVNode::Store; }

	StoreInst *OrigInst = nullptr;
	Value *BasePtr = nullptr;
};

struct SVFloatBinOpt: public SVNode {
	SVFloatBinOpt(BinaryOperator *BI, SVNode *LHS, SVNode *RHS):
		SVNode(SVNode::FloatBinOp), OrigInst(BI),
		BinaryOp(BI->getOpcode()) {
		this->Operands.push_back(LHS);
		this->Operands.push_back(RHS);
		LHS->UsedBy.insert(this);
		RHS->UsedBy.insert(this);
	}
	static bool classof(const SVNode *N) {
		return N->getKind() == SVNode::FloatBinOp; }

	BinaryOperator *OrigInst = nullptr;
	Instruction::BinaryOps BinaryOp;

	Intrinsic::ID getIntrinsic() const {
		switch (BinaryOp) {
		case Instruction::BinaryOps::FAdd: return Intrinsic::aarch64_sve_fadd;
		case Instruction::BinaryOps::FSub: return Intrinsic::aarch64_sve_fsub;
		case Instruction::BinaryOps::FMul: return Intrinsic::aarch64_sve_fmul;
		default:
			llvm_unreachable("no Intrinsic for this operation");
		}
	}
};

static bool vectorize(BasicBlock *BB, const Loop *Loop,
		ScalarEvolution &SE, TargetLibraryInfo &TLI, TargetTransformInfo &TTI) {
	BasicBlock *PredBB = nullptr, *NextBB = nullptr;
	for (auto It = pred_begin(BB), End = pred_end(BB); It != End; ++It)
		if (*It != BB) PredBB = *It;
	for (auto It = succ_begin(BB), End = succ_end(BB); It != End; ++It)
		if (*It != BB) NextBB = *It;
	assert(PredBB && NextBB);

	PHINode *InductionVar = Loop->getCanonicalInductionVariable();

	// This pass only works for float32 operations:
	Type *FloatType = Type::getFloatTy(BB->getContext());
	IRBuilder<> PredBBBuilder(PredBB, --PredBB->getTerminator()->getIterator());
	Value *VScale = PredBBBuilder.CreateIntrinsic(
			InductionVar->getType(), Intrinsic::vscale, {}, nullptr, "vscale");
	// TODO: Get the min. vector size in a more flexible manner?
	// But fuck it, SVE is the only thing that works for now anyways.
	unsigned MinElms = 128 / FloatType->getPrimitiveSizeInBits();
	Value *VL = PredBBBuilder.CreateMul(VScale,
			ConstantInt::get(VScale->getType(), MinElms), "vl");

	DenseSet<Instruction*> VisitedInstructions(BB->size());
	DenseMap<Instruction*, SVNode*> VFNodeByInst(BB->size());
	SmallVector<SVNode*, 0> VFGraphNodes;

	BinaryOperator *IncInst = nullptr;
	ICmpInst *CmpInst = nullptr;
	BranchInst *BrInst = nullptr;

	/*
	 * TODO: For more advanced stuff, one should probably
	 * work backwards from the stores and then handle the induction
	 * variable's PHI, ADD and CMP separately. All instructions not
	 * vectorizable or not reached from the stores or mentioned before
	 * are dead or block vectorization!
	 */
	for (auto I = BB->begin(); I != BB->end(); ++I) {
		switch (I->getOpcode()) {
		case Instruction::PHI:
		{
			// Nothing but the induction variable is allowed to
			// enter or leave this loop!
			if (dyn_cast<PHINode>(I) != InductionVar)
				return false;
			break;
		}
		case Instruction::GetElementPtr:
		{
			// GEPs themselves are not of interest, but once we know
			// the connected load/store, we can create a graph node for it.
			GetElementPtrInst *GEP = cast<GetElementPtrInst>(&*I);
			if (GEP->getNumOperands() != 2
					|| GEP->getOperand(1) != cast<Value>(InductionVar))
				return false;
			VisitedInstructions.insert(&*I);
			break;
		}
		case Instruction::Load:
		{
			LoadInst *LI = cast<LoadInst>(&*I);
			if (LI->getType() != FloatType || LI->getNumOperands() != 1
					|| !VisitedInstructions.contains(
						dyn_cast<Instruction>(LI->getOperand(0)))
					|| !isa<GetElementPtrInst>(LI->getOperand(0)))
				return false;

			// A load! This means we have to create a graph node
			// which other nodes can then use. The GEP is connected
			// to the node.
			SVLoad *Node = new SVLoad(LI);
			VFGraphNodes.push_back(Node);
			VFNodeByInst[&*I] = Node;
			VisitedInstructions.insert(&*I);
			break;
		}
		case Instruction::Store:
		{
			StoreInst *SI = cast<StoreInst>(&*I);
			Instruction *OP0 = dyn_cast<Instruction>(SI->getOperand(0));
			GetElementPtrInst *OP1 = dyn_cast<GetElementPtrInst>(
					SI->getOperand(1));
			if (!OP0 || !OP1 || OP0->getType() != FloatType
					|| !VisitedInstructions.contains(OP0)
					|| !VisitedInstructions.contains(OP1))
				return false;

			// A store! Let's track the nodes this depends on.
			SVNode *Operand = VFNodeByInst[OP0];
			assert(Operand);
			SVStore *Node = new SVStore(SI);
			VFGraphNodes.push_back(Node);
			Node->AddOperand(Operand);
			break;
		}
		case Instruction::FAdd:
		case Instruction::FSub:
		case Instruction::FMul:
		{
			BinaryOperator *BinOp = cast<BinaryOperator>(&*I);
			Instruction *LHS = dyn_cast<Instruction>(BinOp->getOperand(0));
			Instruction *RHS = dyn_cast<Instruction>(BinOp->getOperand(1));
			if (!LHS || !RHS || BinOp->getType() != FloatType
					|| !VisitedInstructions.contains(LHS)
					|| !VisitedInstructions.contains(RHS))
				return false;

			// A binary operation! Let's track the two operands
			// and wait for a store to use the created node.
			SVNode *SVLHS = VFNodeByInst[LHS];
			SVNode *SVRHS = VFNodeByInst[RHS];
			assert(SVLHS && SVRHS);
			SVFloatBinOpt *Node = new SVFloatBinOpt(BinOp, SVLHS, SVRHS);
			VFGraphNodes.push_back(Node);
			VFNodeByInst[&*I] = Node;
			VisitedInstructions.insert(&*I);
			break;
		}
		case Instruction::Add:
		{
			// The only integer add allowed for now is the 
			BinaryOperator *Add = cast<BinaryOperator>(&*I);
			if (IncInst || Add->getOperand(0) != cast<Value>(InductionVar)
					|| Add->getOperand(1) != cast<Value>(
						ConstantInt::get(InductionVar->getType(), 1)))
				return false;
			IncInst = Add;
			break;
		}
		case Instruction::ICmp:
		{
			ICmpInst *Cmp = cast<ICmpInst>(&*I);
			if (CmpInst || !IncInst || Cmp->getOperand(0) != cast<Value>(IncInst)
					|| !(Cmp->getPredicate() == ICmpInst::ICMP_EQ
						|| Cmp->getPredicate() == ICmpInst::ICMP_SLT
						|| Cmp->getPredicate() == ICmpInst::ICMP_SGT))
				return false;
			CmpInst = Cmp;
			break;
		}
		case Instruction::Br:
		{
			BranchInst *Branch = cast<BranchInst>(&*I);
			if (BrInst || !CmpInst || Branch->getNumSuccessors() != 2
					|| Branch->getSuccessor(0) != NextBB
					|| Branch->getSuccessor(1) != BB
					|| Branch->getOperand(0) != cast<Value>(CmpInst))
				return false;
			BrInst = Branch;
			break;
		}
		default:
			WithColor::warning() << "cannot vectorize instruction:\n";
			I->dump();
			return false;
		}
	}

	WithColor::note() << "vecfun: loop vectorization in function "
					  << BB->getParent()->getName()
					  << " of loop with header " << Loop->getHeader()->getName()
					  << "\n";

	// Increase by VL instead of one after every run:
	BinaryOperator *NewInc = BinaryOperator::CreateAdd(InductionVar, VL,
			"vl_" + Twine(IncInst->getName()), IncInst);
	IncInst->replaceAllUsesWith(NewInc);
	IncInst->eraseFromParent();

	// A comparison against N directly would not work anymore if N
	// is not a multiple of VL.
	if (CmpInst->getPredicate() == ICmpInst::ICMP_EQ)
		CmpInst->setPredicate(BrInst->getSuccessor(0) == BB
				? ICmpInst::ICMP_SLT : ICmpInst::ICMP_SGT);

	ScalableVectorType *PredType = ScalableVectorType::get(
			IntegerType::get(BB->getContext(), 1), MinElms);
	ScalableVectorType *VecType = ScalableVectorType::get(FloatType, MinElms);

	// Start every loop by creating a mask in which all lanes smaller
	// than N are active.
	IRBuilder<> BBBuilder(BB, BB->getFirstInsertionPt());
	Instruction *Mask = BBBuilder.CreateIntrinsic(
			PredType, Intrinsic::aarch64_sve_whilelt,
			{ InductionVar, CmpInst->getOperand(1) }, nullptr, "vl_mask");
	for (auto Iter = VFGraphNodes.begin(),
			IEnd = VFGraphNodes.end(); Iter != IEnd; ++Iter) {
		SVNode *Node = *Iter;
		switch (Node->Kind) {
		case SVNode::Load:
		{
			SVLoad *Load = cast<SVLoad>(Node);
			Value *Ptr = BBBuilder.CreateGEP(FloatType, Load->BasePtr,
					{ InductionVar }, "vl_load_gep");
			Instruction *VLoad = BBBuilder.CreateMaskedLoad(
					VecType, Ptr, Align(4), Mask,
					ConstantAggregateZero::get(VecType), "vl_load");
			Load->ReturnValue = VLoad;
			break;
		}
		case SVNode::Store:
		{
			SVStore *Store = cast<SVStore>(Node);
			Value *Ptr = BBBuilder.CreateGEP(FloatType, Store->BasePtr,
					{ InductionVar }, "vl_store_gep");
			assert(Store->Operands.size() == 1
					&& Store->Operands[0]->ReturnValue);
			Value *Val = Store->Operands[0]->ReturnValue;
			Instruction *VStore = BBBuilder.CreateMaskedStore(
					Val, Ptr, Align(4), Mask);

			// By erasing the old store, a dead-code elimination can later
			// delete all the other non-vectorized/replaced instructions
			// still in the basic block:
			Store->OrigInst->eraseFromParent();
			break;
		}
		case SVNode::FloatBinOp:
		{
			SVFloatBinOpt *BinOp = cast<SVFloatBinOpt>(Node);
			assert(BinOp->Operands[0]->ReturnValue
					&& BinOp->Operands[1]->ReturnValue);
			Instruction *VOp = BBBuilder.CreateIntrinsic(
					VecType, BinOp->getIntrinsic(),
					{ Mask, BinOp->Operands[0]->ReturnValue,
						BinOp->Operands[1]->ReturnValue },
					nullptr, "vl_" + Twine(BinOp->OrigInst->getName()));
			BinOp->ReturnValue = VOp;
			break;
		}
		default:
			llvm_unreachable("whoops, forgotten something?");
			return false;
		}
	}

	return true;
}

namespace {
	struct VeryBadVectorizerPass: public PassInfoMixin<VeryBadVectorizerPass> {
		PreservedAnalyses run(Function &F, FunctionAnalysisManager &FAM) {
			// WithColor::note() << "vecfun: visiting function '" << F.getName() << "'...\n";

			// TODO: Use AliasAnalysis! For the moment, semantically wrong
			// code could be generated!
			AliasAnalysis &AA = FAM.getResult<AAManager>(F);
			LoopInfo &LI = FAM.getResult<LoopAnalysis>(F);
			DominatorTree &DT = FAM.getResult<DominatorTreeAnalysis>(F);
			TargetLibraryInfo &TLI = FAM.getResult<TargetLibraryAnalysis>(F);
			TargetTransformInfo &TTI = FAM.getResult<TargetIRAnalysis>(F);
			ScalarEvolution &SE = FAM.getResult<ScalarEvolutionAnalysis>(F);

			// TODO: At the moment, DependenceInfo is irrelevant because only use of the
			// induction variable directly is allowed, and no expressions of it.
			// In order to support more flexible addressing, this will be needed:
			DependenceInfo DI(&F, &AA, &SE, &LI);

			bool Change = false;
			for (const Loop *Loop: LI) {
				// No predication and no nested loops for now:
				if (Loop->getBlocks().size() != 1)
					continue;

				// Let's keep it simple:
				if (!Loop->getCanonicalInductionVariable())
					continue;

				// Might be redundant?
				BasicBlock *BB = Loop->getBlocks()[0];
				if (!BB->hasNPredecessors(2))
					continue;

				if (!instructionsCanBeVectorized(BB))
					continue;

				Change |= vectorize(BB, Loop, SE, TLI, TTI);
			}

			// Maybe be a bit nicer in case of changes?
			// The loop stays a loop etc, so CFG analysis should
			// be allowed to live on?
			return Change
				? PreservedAnalyses::none()
				: PreservedAnalyses::all();
		}
	};
}

extern "C" ::llvm::PassPluginLibraryInfo LLVM_ATTRIBUTE_WEAK llvmGetPassPluginInfo() {
	return {
		LLVM_PLUGIN_API_VERSION,
		"VeryBadVectorizerPass",
		"v0.1",
		[](PassBuilder &PB) {
			PB.registerPipelineParsingCallback([](
						StringRef PassName, FunctionPassManager &FPM,
						ArrayRef<PassBuilder::PipelineElement> Pipeline) -> bool {
				if (PassName == "vbv") {
					FPM.addPass(VeryBadVectorizerPass());
					return true;
				}
				return false;
			});
		}
	};
}

