#include "clang/AST/ASTConsumer.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/FrontendPluginRegistry.h"
#include <clang/AST/ASTContext.h>
#include <clang/AST/Attrs.inc>
#include <clang/AST/CharUnits.h>
#include <clang/AST/Decl.h>
#include <clang/AST/Type.h>
#include <llvm/Support/Casting.h>

using namespace clang;

class Consumer : public clang::ASTConsumer {
  clang::CompilerInstance &CI;

  void visitTypeDecl(const clang::TypeDecl &TD) {
    const clang::RecordDecl *RD = dyn_cast<clang::RecordDecl>(&TD);
    if (!RD || !RD->isCompleteDefinition() || !(RD->isStruct() || RD->isUnion()))
      return;

    for (const clang::Decl *D : RD->decls())
      if (const auto *TD = dyn_cast<clang::TypeDecl>(D))
        visitTypeDecl(*TD);

    const clang::ASTContext &Ctx = CI.getASTContext();
    const unsigned CW = Ctx.getCharWidth();
    bool IsPacked = false;
    unsigned Align = Ctx.getTypeAlign(RD->getTypeForDecl()) / CW;
    for (const clang::Attr *A : RD->attrs())
      if (isa<clang::PackedAttr>(A))
        IsPacked |= true;
      else if (const clang::AlignedAttr *AA = dyn_cast<clang::AlignedAttr>(A))
        Align = AA->getAlignment(CI.getASTContext()) / CW;

    unsigned MaxFieldAlign = Align;
    for (const clang::FieldDecl *F : RD->fields())
      MaxFieldAlign = std::max(MaxFieldAlign, Ctx.getTypeAlign(F->getType()));

    if (IsPacked && Align == 1 && MaxFieldAlign != 1 &&
        !RD->hasAttr<AlignedAttr>()) {
      clang::DiagnosticsEngine &Diag = CI.getDiagnostics();
      unsigned ID = Diag.getCustomDiagID(
          DiagnosticsEngine::Remark,
          "packed structure has alignment of 1 (byte), are you sure this is "
          "what you want? Consider using __attribute__((aligned(...))).");
      Diag.Report(RD->getLocation(), ID);
    }

    auto IsPackedStruct = [](const clang::FieldDecl &FD) -> bool {
      return FD.getType()->isStructureType() &&
             FD.getType()->getAsRecordDecl()->hasAttr<clang::PackedAttr>();
    };

    auto IsCharArray = [](const clang::FieldDecl &FD) -> bool {
      const auto *ArrTy = dyn_cast<clang::ArrayType>(FD.getType().getTypePtr());
      return ArrTy && ArrTy->getElementType()->isCharType();
    };

    llvm::SmallVector<const clang::FieldDecl *> Fields(RD->fields());
    if (Fields.size() == 2 &&
        ((IsPackedStruct(*Fields[0]) && IsCharArray(*Fields[1])) ||
         (IsCharArray(*Fields[0]) && IsPackedStruct(*Fields[1]))) &&
        Ctx.getTypeSize(Fields[0]->getType()) !=
            Ctx.getTypeSize(Fields[1]->getType())) {
      clang::DiagnosticsEngine &Diag = CI.getDiagnostics();
      unsigned ID = Diag.getCustomDiagID(
          DiagnosticsEngine::Remark,
          "union of packed structure and character array of unequal size.");
      Diag.Report(RD->getLocation(), ID);
    }

    return;
  }

public:
  Consumer(clang::CompilerInstance &CI) : CI(CI) {}

  bool HandleTopLevelDecl(clang::DeclGroupRef DG) override {
    for (clang::DeclGroupRef::iterator I = DG.begin(), E = DG.end(); I != E;
         ++I)
      if (const auto *TypeDecl = dyn_cast<clang::TypeDecl>(*I))
        visitTypeDecl(*TypeDecl);
    return true;
  }
};

class Action : public clang::PluginASTAction {
public:
  std::unique_ptr<clang::ASTConsumer>
  CreateASTConsumer(clang::CompilerInstance &CI,
                    clang::StringRef file) override {
    return std::make_unique<Consumer>(CI);
  }

  bool ParseArgs(const clang::CompilerInstance &CI,
                 const std::vector<std::string> &args) override {
    return true;
  }

  clang::PluginASTAction::ActionType getActionType() override {
    return clang::PluginASTAction::ActionType::AddBeforeMainAction;
  }
};

static clang::FrontendPluginRegistry::Add<Action>
    X("warn-packed", "Warnings for structs with packed attributes");
