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

llvm::PassPluginLibraryInfo getHelloWorldPluginInfo() {
	return {
		LLVM_PLUGIN_API_VERSION,
		"HelloWorld",
		LLVM_VERSION_STRING,
		[](llvm::PassBuilder &PB) {
			PB.registerPipelineParsingCallback([](llvm::StringRef Name, llvm::FunctionPassManager &FPM, llvm::ArrayRef<llvm::PassBuilder::PipelineElement>) {
				if (Name == "hello-world") {
					FPM.addPass(HelloWorld());
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

