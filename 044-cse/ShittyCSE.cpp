#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"
#include <llvm/ADT/DenseMap.h>
#include <llvm/ADT/Hashing.h>
#include <llvm/ADT/STLExtras.h>
#include <llvm/ADT/bit.h>
#include <llvm/IR/BasicBlock.h>
#include <llvm/IR/CFG.h>
#include <llvm/IR/Dominators.h>
#include <llvm/IR/Function.h>
#include <llvm/IR/Instruction.h>
#include <memory>

using namespace llvm;

static constexpr bool DoDebug = true;
#ifdef LLVM_DEBUG
#undef LLVM_DEBUG
#endif
#define LLVM_DEBUG(stmt) do { stmt; } while (0)

struct Expr {
  Instruction &Def;
  std::shared_ptr<Expr> Next;

  Expr(Instruction &Def): Def(Def) {}
  Expr(Instruction &Def, std::shared_ptr<Expr> &&Next): Def(Def), Next(Next) {}

  bool canReplace(Instruction &I) {
    if (!Def.isSameOperationAs(&I) ||
        Def.getNumOperands() != I.getNumOperands())
      return false;

    if (I.isCommutative() && I.getNumOperands() == 2 &&
        (I.getOperand(0) == Def.getOperand(1) &&
         I.getOperand(1) == Def.getOperand(0)))
      return true;

    return all_of_zip(I.operands(), Def.operands(), [&](const Value *Op1, const Value *Op2) {
      return Op1 == Op2;
    });
  }
};

struct ShittyCSE {
  Function &F;
  DominatorTree &DT;

  ShittyCSE(Function &F, DominatorTree &DT)
    : F(F), DT(DT) {}

  BasicBlock *getIDom(BasicBlock &BB) const {
    if (BB.hasNPredecessors(0))
      return nullptr;

    auto Preds = predecessors(&BB);
    BasicBlock *IDom = *Preds.begin();
    for (BasicBlock *BB : drop_begin(Preds))
      IDom = DT.findNearestCommonDominator(IDom, BB);
    return IDom;
  }

  unsigned run() {
    unsigned NumReplaced = 0;
    DenseMap<const BasicBlock *, std::shared_ptr<Expr>> ExprsPerBB;
    ReversePostOrderTraversal<Function *> RPOT(&F);
    for (BasicBlock *BB : RPOT) {
      std::shared_ptr<Expr> Exprs = ExprsPerBB[getIDom(*BB)];
      for (BasicBlock::iterator Iter = BB->begin(); Iter != BB->end(); ++Iter) {
        Instruction &I = *Iter;
        LLVM_DEBUG(dbgs() << "Visiting: " << I << "\n");
        if (!I.willReturn() || I.mayHaveSideEffects() || I.mayReadOrWriteMemory() || I.isTerminator())
          continue;

        bool Replaced = false;
        for (Expr *E = Exprs.get(); E != nullptr; E = E->Next.get()) {
          assert(DT.dominates(&E->Def, &I));
          if (!E->canReplace(I))
            continue;

          LLVM_DEBUG(dbgs() << "Replaced: " << I << "\n"
                            << "    with: " << E->Def << "\n");
          Replaced = true;
          NumReplaced += 1;
          I.replaceAllUsesWith(&E->Def);
          Iter = I.eraseFromParent();
          --Iter;
          break;
        }
        if (Replaced)
          continue;

        Exprs = std::make_shared<Expr>(I, std::move(Exprs));
      }
      ExprsPerBB[BB] = Exprs;
    }

    return NumReplaced;
  }
};

struct ShittyCSEPass: PassInfoMixin<ShittyCSEPass> {
	PreservedAnalyses run(Function &F, FunctionAnalysisManager &FAM) {
    auto &DT = FAM.getResult<DominatorTreeAnalysis>(F);
    ShittyCSE CSE(F, DT);
    unsigned NumReplaced = CSE.run();
    LLVM_DEBUG(assert(CSE.run() == 0 && "Re-run should not have found new CSE opts."));
		return NumReplaced > 0 ? PreservedAnalyses::none() : PreservedAnalyses::all();
	}
};

PassPluginLibraryInfo getPluginInfo() {
	return {
		LLVM_PLUGIN_API_VERSION,
		"shitty-cse",
		LLVM_VERSION_STRING,
		[](PassBuilder &PB) {
			PB.registerPipelineParsingCallback([](StringRef Name, FunctionPassManager &FPM, ArrayRef<PassBuilder::PipelineElement>) {
				if (Name == "shitty-cse") {
					FPM.addPass(ShittyCSEPass());
					return true;
				}
				return false;
			});
		}
	};
}

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
	return getPluginInfo();
}

