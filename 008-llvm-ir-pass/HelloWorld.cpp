#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"

#include <vector>
#include <tuple>

static bool isPowerOfTwo(int64_t x) {
	return x > 0 && (x & (x - 1)) == 0;
}

static int64_t log2(int64_t x) {
	llvm::outs() << "-------------> x = " << x << "\n";
	int64_t y = sizeof(int64_t) * 8 - __builtin_clzl(x) - 1;
	llvm::outs() << "-------------> y = " << y << "\n";
	return y;
}

struct HelloWorld: llvm::PassInfoMixin<HelloWorld> {
	llvm::PreservedAnalyses run(llvm::Function &F, llvm::FunctionAnalysisManager &FAM) {
		llvm::errs() << "[hello-world]: entering function '" << F.getName() << "'\n";

		bool change = false;
		for (llvm::BasicBlock &bb: F) {
			for (auto inst = bb.begin(); inst != bb.end(); ++inst) {
				if (!inst->isBinaryOp() || inst->getOpcode() != llvm::Instruction::BinaryOps::SDiv)
					continue;

				llvm::ConstantInt *rhs = llvm::dyn_cast<llvm::ConstantInt>(inst->getOperand(1));
				if (!rhs || rhs->getBitWidth() >= 64 || !isPowerOfTwo(rhs->getSExtValue()))
					continue;

				llvm::ConstantInt *shiftby = llvm::ConstantInt::get(rhs->getType(), uint64_t(log2(rhs->getSExtValue())));
				llvm::Instruction *shift = llvm::BinaryOperator::CreateShl(inst->getOperand(0), shiftby);
				llvm::ReplaceInstWithInst(bb.getInstList(), inst, shift); // <- one must pass the iterator and bbs here or the iterator goes invalid!
				change = true;
			}
		}

		return change ? llvm::PreservedAnalyses::none() : llvm::PreservedAnalyses::all();
	}

	static bool isRequired() { return true; }
};

struct ReverseGauss: llvm::PassInfoMixin<ReverseGauss> {
	llvm::PreservedAnalyses run(llvm::Function &F, llvm::FunctionAnalysisManager &FAM) {
		bool change = false;

		/* find the following pattern: `(x * (x + 1)) / 2` */
		std::vector<std::tuple<llvm::Instruction*, llvm::Value*>> worklist;
		for (auto bb = F.begin(); bb != F.end(); ++bb) {
			for (auto inst = bb->begin(); inst != bb->end(); ++inst) {
				if (inst->getOpcode() != llvm::Instruction::BinaryOps::SDiv)
					continue;

				llvm::ConstantInt *two = llvm::dyn_cast<llvm::ConstantInt>(inst->getOperand(1));
				if (!two || two->getSExtValue() != 2)
					continue;

				llvm::BinaryOperator *mul = llvm::dyn_cast<llvm::BinaryOperator>(inst->getOperand(0));
				if (!mul || mul->getOpcode() != llvm::Instruction::BinaryOps::Mul)
					continue;

				llvm::Value *n = mul->getOperand(0);
				llvm::BinaryOperator *add = llvm::dyn_cast<llvm::BinaryOperator>(mul->getOperand(1));
				if (!add || add->getOpcode() != llvm::Instruction::BinaryOps::Add || add->getOperand(0) != n)
					continue;

				llvm::ConstantInt *one = llvm::dyn_cast<llvm::ConstantInt>(add->getOperand(1));
				if (!one || one->getSExtValue() != 1)
					continue;

				change = true;
				worklist.push_back(std::make_tuple(&*inst, n));
			}
		}

		/* transform to the sum of all numbers from 0 to x: */
		for (auto workitem: worklist) {
			llvm::Instruction *inst = std::get<0>(workitem);
			llvm::Value *n = std::get<1>(workitem);

			llvm::BasicBlock *prevBB = inst->getParent()->splitBasicBlockBefore(inst);
			llvm::BasicBlock *nextBB = inst->getParent(); // <- also includes the division by 2

			llvm::BasicBlock *condBB = llvm::BasicBlock::Create(F.getContext(), "gauss_cond", &F, nextBB);
			llvm::BasicBlock *loopBB = llvm::BasicBlock::Create(F.getContext(), "gauss_loop", &F, condBB);

			// inserted loop starts after first halve of split block:
			prevBB->back().eraseFromParent();
			llvm::IRBuilder<> prevBuilder(prevBB);
			llvm::Value *sum0 = llvm::ConstantInt::get(n->getType(), 0);
			llvm::Value *i0 = llvm::ConstantInt::get(n->getType(), 0);
			prevBuilder.CreateBr(condBB);

			// building condition of the loop:
			llvm::IRBuilder<> condBuilder(condBB);
			llvm::PHINode *sum1 = condBuilder.CreatePHI(n->getType(), 2, "cond_sum");
			sum1->addIncoming(sum0, prevBB);
			llvm::PHINode *i1 = condBuilder.CreatePHI(n->getType(), 2, "cond_i");
			i1->addIncoming(i0, prevBB);
			llvm::Value *cmp = condBuilder.CreateCmp(llvm::CmpInst::ICMP_EQ, i1, n);
			condBuilder.CreateCondBr(cmp, loopBB, nextBB);

			// building the body of the loop:
			llvm::IRBuilder<> loopBuilder(loopBB);
			llvm::Value *sum2 = loopBuilder.CreateAdd(sum1, i1, "loop_sum");
			llvm::Value *i2 = loopBuilder.CreateAdd(i1, llvm::ConstantInt::get(n->getType(), 1), "loop_i");
			sum1->addIncoming(sum2, loopBB);
			i1->addIncoming(i2, loopBB);
			loopBuilder.CreateBr(condBB);

			// almost done:
			inst->replaceAllUsesWith(sum1);
			// TODO: Remove the now dead code...
		}

		return change ? llvm::PreservedAnalyses::none() : llvm::PreservedAnalyses::all();
	}

	static bool isRequired() { return true; }
};

llvm::PassPluginLibraryInfo getHelloWorldPluginInfo() {
	return {
		LLVM_PLUGIN_API_VERSION,
		"HelloWorld",
		LLVM_VERSION_STRING,
		[](llvm::PassBuilder &PB) {
			PB.registerPipelineParsingCallback([](llvm::StringRef Name, llvm::FunctionPassManager &FPM, llvm::ArrayRef<llvm::PassBuilder::PipelineElement>) {
				if (Name == "hello-world") {
					// FPM.addPass(HelloWorld());
					FPM.addPass(ReverseGauss());
					return true;
				}
				return false;
			});
		}
	};
}

// This is the core interface for pass plugins. It guarantees that 'opt' will
// be able to recognize HelloWorld when added to the pass pipeline on the
// command line, i.e. via '-passes=hello-world'
extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
	return getHelloWorldPluginInfo();
}

