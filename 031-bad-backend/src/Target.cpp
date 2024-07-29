#include "llvm/ADT/StringRef.h"
#include "llvm/Config/llvm-config.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/raw_ostream.h"
#include <cassert>
#include <cstdint>
#include <llvm/ADT/ArrayRef.h>
#include <llvm/ADT/DenseMap.h>
#include <llvm/ADT/STLExtras.h>
#include <llvm/ADT/SmallVector.h>
#include <llvm/ADT/Twine.h>
#include <llvm/IR/Argument.h>
#include <llvm/IR/BasicBlock.h>
#include <llvm/IR/CFG.h>
#include <llvm/IR/Constants.h>
#include <llvm/IR/DataLayout.h>
#include <llvm/IR/DerivedTypes.h>
#include <llvm/IR/InstrTypes.h>
#include <llvm/IR/Instruction.h>
#include <llvm/IR/Instructions.h>
#include <llvm/IR/IntrinsicInst.h>
#include <llvm/IR/PassManager.h>
#include <llvm/Support/Casting.h>
#include <llvm/Support/ErrorHandling.h>
#include <optional>

using namespace llvm;

namespace {

struct Block;

struct Inst {
  enum OpCodeTy {
    Invalid,
    Add,
    Arg,
    Br,
    BrCond,
    Constant,
    ICmpEQ,
    Load,
    Move,
    Mul,
    Phi,
    Ret,
  } OpCode;
  Type *Ty = nullptr;
  const Value *IRVal = nullptr;
  std::optional<unsigned> VReg = std::nullopt;
  std::optional<unsigned> PReg = std::nullopt;
  SmallVector<unsigned> VOperands;
  Block *BB = nullptr;

  Inst(OpCodeTy OpCode, Type *Ty, const Value *IRVal,
       std::optional<unsigned> VReg, ArrayRef<unsigned> VOperands)
      : OpCode(OpCode), Ty(Ty), IRVal(IRVal), VReg(VReg), VOperands(VOperands) {
    assert(!Ty || Ty->isPointerTy() || Ty->isIntegerTy());
  }

  void print(raw_ostream &O) const;

  bool isTerminator() const {
    return OpCode == Br || OpCode == BrCond || OpCode == Ret;
  }

  void removeFromBB(bool Del);
};

struct Block {
  std::string Name;
  SmallVector<Inst *> Instrs;
  SmallVector<Block *> Predecessors;
  SmallVector<Block *> Successors;

  Block(StringRef Name) : Name(Name) {}

  void append(Inst *I) {
    assert(I && !I->BB);
    I->BB = this;
    Instrs.push_back(I);
  }

  void insertBeforeTerminator(Inst *I) {
    assert(I && !I->BB);
    I->BB = this;
    if (Instrs.empty() || !Instrs.back()->isTerminator())
      Instrs.push_back(I);
    else
      Instrs.insert(Instrs.end() - 1, I);
  }

  void addSucc(Block *B) {
    assert(!is_contained(Successors, B));
    assert(!is_contained(B->Predecessors, this));
    Successors.push_back(B);
    B->Predecessors.push_back(this);
  }

  void print(raw_ostream &O) const {
    O << "." << Name << ":\n";
    if (!Predecessors.empty()) {
      O << "  # preds: ";
      for (Block *Pred : Predecessors)
        O << (Pred == Predecessors[0] ? "." : ", .") << Pred->Name;
      O << "\n";
    }
    for (Inst *I : Instrs)
      I->print(O);
    if (!Successors.empty()) {
      O << "  # succs: ";
      for (Block *Succ : Successors)
        O << (Succ == Successors[0] ? "." : ", .") << Succ->Name;
      O << "\n";
    }
    O << "\n";
  }
};

void Inst::print(raw_ostream &O) const {
  O << "  ";
  if (VReg)
    O << "%v" << VReg.value() << " = ";
  switch (OpCode) {
  case Invalid:
    O << "!INVALID!";
    break;
  case Add:
    assert(VOperands.size() == 2);
    O << "add";
    break;
  case Arg: {
    assert(VOperands.size() == 0 && BB->Predecessors.empty());
    const Argument *A = cast<Argument>(IRVal);
    O << "argument#" << A->getArgNo() << " (" << A->getName() << ")";
    break;
  }
  case Br:
    assert(VOperands.size() == 0 && BB->Successors.size() == 1 &&
           !VReg.has_value() && BB->Instrs.back() == this);
    O << "br";
    break;
  case BrCond:
    assert(VOperands.size() == 1 && BB->Successors.size() == 2 &&
           !VReg.has_value() && BB->Instrs.back() == this);
    O << "br.cond";
    break;
  case Constant:
    assert(VOperands.size() == 0);
    O << "constant " << *cast<ConstantInt>(IRVal);
    break;
  case ICmpEQ:
    assert(VOperands.size() == 2);
    O << "icmp eq";
    break;
  case Load:
    assert(VOperands.size() == 1);
    O << "load";
    break;
  case Move:
    assert(VOperands.size() == 1);
    O << "move";
    break;
  case Mul:
    assert(VOperands.size() == 2);
    O << "mul";
    break;
  case Phi:
    assert(VOperands.size() == BB->Predecessors.size());
    O << "phi";
    for (auto [Pred, VReg] : zip(BB->Predecessors, VOperands))
      O << " [ " << Pred->Name << ": %v" << VReg << " ]";
    O << "\n";
    return;
  case Ret:
    assert(VOperands.size() == 1 && BB->Successors.size() == 0 &&
           BB->Instrs.back() == this);
    O << "ret";
    break;
  default:
    llvm_unreachable("unhandled opcode!");
  }
  for (unsigned VOp : VOperands)
    O << ", %v" << VOp;
  O << "\n";
}

void Inst::removeFromBB(bool Del) {
  assert(BB && is_contained(BB->Instrs, this));
  BB->Instrs.erase(std::find(BB->Instrs.begin(), BB->Instrs.end(), this));
  BB = nullptr;
  if (Del)
    delete this;
}

static const DenseMap<unsigned, Inst::OpCodeTy> LLVMOpcode2InstOpcode = {
    {Instruction::Add, Inst::Add},
    {Instruction::Mul, Inst::Mul},
};

struct BadCodeGenPass : PassInfoMixin<BadCodeGenPass> {
  raw_ostream &outs;

  BadCodeGenPass(raw_ostream &outs) : outs(outs) {}

  PreservedAnalyses run(Function &F, FunctionAnalysisManager &FAM) {
    unsigned MaxVReg = 1;
    SmallVector<Block *> BBs;
    SmallVector<std::tuple<const PHINode *, Block *, Inst *>> PHIsToFix;
    DenseMap<const Value *, unsigned> IRVal2VReg;
    DenseMap<const BasicBlock *, Block *> IRBB2BB;
    DenseMap<Block *, const BasicBlock *> BB2IRBB;
    for (const BasicBlock &IRBB : F) {
      Block *BB = new Block(IRBB.getName());
      BBs.push_back(BB);
      IRBB2BB[&IRBB] = BB;
      BB2IRBB[BB] = &IRBB;
    }

    auto GetOpVReg = [&](const Value *Op, Block *BB) -> unsigned {
      if (const auto *IRC = dyn_cast<ConstantInt>(Op)) {
        Inst *C = new Inst(Inst::Constant, Op->getType(), IRC, MaxVReg++, {});
        BB->insertBeforeTerminator(C);
        return C->VReg.value();
      }
      return IRVal2VReg.at(Op);
    };

    const DataLayout &DL = F.getParent()->getDataLayout();
    IntegerType *PtrIntTy =
        IntegerType::get(F.getContext(), DL.getPointerSizeInBits());
    for (const BasicBlock &IRBB : F) {
      Block *BB = IRBB2BB[&IRBB];
      for (const BasicBlock *IRSucc : successors(&IRBB))
        BB->addSucc(IRBB2BB[IRSucc]);

      if (IRBB.isEntryBlock())
        for (const Argument &IRArg : F.args()) {
          Inst *Arg =
              new Inst(Inst::Arg, IRArg.getType(), &IRArg, MaxVReg++, {});
          IRVal2VReg[&IRArg] = Arg->VReg.value();
          BB->append(Arg);
        }

      for (const Instruction &IRI : IRBB) {
        if (const auto *IRPhi = dyn_cast<PHINode>(&IRI)) {
          Inst *Phi =
              new Inst(Inst::Phi, IRPhi->getType(), IRPhi, MaxVReg++, {});
          PHIsToFix.push_back({IRPhi, BB, Phi});
          IRVal2VReg[IRPhi] = Phi->VReg.value();
          BB->append(Phi);
          continue;
        }

        if (const auto *IRCmp = dyn_cast<ICmpInst>(&IRI)) {
          assert(IRCmp->getPredicate() == ICmpInst::ICMP_EQ);
          Inst *Cmp =
              new Inst(Inst::ICmpEQ, IRCmp->getType(), nullptr, MaxVReg++,
                       {GetOpVReg(IRCmp->getOperand(0), BB),
                        GetOpVReg(IRCmp->getOperand(1), BB)});
          IRVal2VReg[IRCmp] = Cmp->VReg.value();
          BB->append(Cmp);
          continue;
        }

        if (const auto *IRBr = dyn_cast<BranchInst>(&IRI)) {
          Inst *Br = new Inst(
              IRBr->isConditional() ? Inst::BrCond : Inst::Br, nullptr, nullptr,
              std::nullopt,
              IRBr->isConditional()
                  ? ArrayRef<unsigned>({GetOpVReg(IRBr->getCondition(), BB)})
                  : ArrayRef<unsigned>());
          BB->append(Br);
          continue;
        }

        if (const auto *IRGEP = dyn_cast<GetElementPtrInst>(&IRI)) {
          unsigned BaseVReg = GetOpVReg(IRGEP->getPointerOperand(), BB);
          Type *Ty = IRGEP->getSourceElementType();
          unsigned OpVReg = GetOpVReg(IRGEP->getOperand(1), BB);
          unsigned Size = DL.getTypeAllocSize(Ty);
          if (Size != 1) {
            unsigned SizeInReg =
                GetOpVReg(ConstantInt::get(PtrIntTy, Size), BB);
            Inst *Mul = new Inst(Inst::Mul, PtrIntTy, nullptr, MaxVReg++,
                                 {SizeInReg, OpVReg});
            BB->append(Mul);
            OpVReg = Mul->VReg.value();
          }

          Inst *Add = new Inst(Inst::Add, PtrIntTy, nullptr, MaxVReg++,
                               {BaseVReg, OpVReg});
          BB->append(Add);
          IRVal2VReg[IRGEP] = Add->VReg.value();
          continue;
        }

        if (const auto *IRLI = dyn_cast<LoadInst>(&IRI)) {
          Inst *LI = new Inst(Inst::Load, IRLI->getType(), nullptr, MaxVReg++,
                              {GetOpVReg(IRLI->getPointerOperand(), BB)});
          IRVal2VReg[IRLI] = LI->VReg.value();
          BB->append(LI);
          continue;
        }

        if (const auto *IRBinOp = dyn_cast<BinaryOperator>(&IRI)) {
          Inst *BinOp = new Inst(LLVMOpcode2InstOpcode.at(IRBinOp->getOpcode()),
                                 IRBinOp->getType(), nullptr, MaxVReg++,
                                 {GetOpVReg(IRBinOp->getOperand(0), BB),
                                  GetOpVReg(IRBinOp->getOperand(1), BB)});
          IRVal2VReg[IRBinOp] = BinOp->VReg.value();
          BB->append(BinOp);
          continue;
        }

        if (const auto *IRRet = dyn_cast<ReturnInst>(&IRI)) {
          bool IsRetVoid = IRRet->getNumOperands() == 0;
          Inst *Ret = new Inst(
              Inst::Ret, nullptr, nullptr, std::nullopt,
              IsRetVoid
                  ? ArrayRef<unsigned>()
                  : ArrayRef<unsigned>({GetOpVReg(IRRet->getOperand(0), BB)}));
          BB->append(Ret);
          continue;
        }

        llvm_unreachable("unimplemeted!");
      }
    }

    for (auto [IRPhi, BB, Phi] : PHIsToFix)
      for (Block *Pred : BB->Predecessors) {
        const Value *IncIRVal =
            IRPhi->getIncomingValueForBlock(BB2IRBB.at(Pred));
        Phi->VOperands.push_back(GetOpVReg(IncIRVal, Pred));
      }

    // Initial construction is done!
    dbgs() << "----- " << F.getName() << " -----\n";
    for (const Block *BB : BBs)
      BB->print(dbgs());

    replacePHIs(BBs);

    // PHI removal:
    dbgs() << "----- " << F.getName() << " -----\n";
    for (const Block *BB : BBs)
      BB->print(dbgs());
    return PreservedAnalyses::all();
  }

  void replacePHIs(ArrayRef<Block *> BBs) {
    for (Block *BB : BBs) {
      for (Inst *Phi : BB->Instrs) {
        if (Phi->OpCode != Inst::Phi)
          break;

        unsigned DstVReg = Phi->VReg.value();
        for (unsigned Idx = 0; Idx < BB->Predecessors.size(); ++Idx) {
          Inst *Mov = new Inst(Inst::Move, Phi->Ty, nullptr, DstVReg,
                               {Phi->VOperands[Idx]});
          BB->Predecessors[Idx]->insertBeforeTerminator(Mov);
        }

        Phi->removeFromBB(true);
      }
    }
  }

  static bool isRequired() { return true; }
};

}; // namespace

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "BadCodeGen", LLVM_VERSION_STRING,
          [](PassBuilder &PB) {
            PB.registerPipelineParsingCallback(
                [](StringRef Name, FunctionPassManager &FPM,
                   ArrayRef<PassBuilder::PipelineElement>) {
                  if (Name == "bad-codegen") {
                    FPM.addPass(BadCodeGenPass(outs()));
                    return true;
                  }
                  return false;
                });
          }};
}
