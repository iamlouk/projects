
#include "clang/AST/ASTConsumer.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/FrontendPluginRegistry.h"

using namespace clang;

class HelloWorldConsumer: public clang::ASTConsumer {
	clang::CompilerInstance &CI;

	void traverseStatements(const clang::Stmt *stmt) {
		if (const clang::CompoundStmt *cs = clang::dyn_cast<clang::CompoundStmt>(stmt)) {
			for (const clang::Stmt *s: cs->body()) {
				this->traverseStatements(s);
			}
			return;
		}

		if (const clang::ValueStmt *vc = clang::dyn_cast<clang::ValueStmt>(stmt)) {
			const clang::Expr *expr = vc->getExprStmt();
			if (const clang::BinaryOperator *bo = clang::dyn_cast<clang::BinaryOperator>(expr)) {
				if (bo->getOpcode() != clang::BinaryOperator::Opcode::BO_Assign)
					return;

				const clang::DeclRefExpr *lhs = clang::dyn_cast<clang::DeclRefExpr>(bo->getLHS()->IgnoreUnlessSpelledInSource()),
										 *rhs = clang::dyn_cast<clang::DeclRefExpr>(bo->getRHS()->IgnoreUnlessSpelledInSource());

				if (lhs && rhs && lhs->getDecl()->getName() == rhs->getDecl()->getName()) {
					clang::DiagnosticsEngine &Diag = CI.getDiagnostics();
					unsigned int ID = Diag.getCustomDiagID(DiagnosticsEngine::Warning, "self-assignment found");
					Diag.Report(expr->getExprLoc(), ID);
				}
			}
			return;
		}

		// TODO...
	}

public:
	HelloWorldConsumer(clang::CompilerInstance &CI) : CI(CI) {}

	bool HandleTopLevelDecl(clang::DeclGroupRef DG) override {
		for (clang::DeclGroupRef::iterator I = DG.begin(), E = DG.end(); I != E; ++I) {
			const clang::Decl *D = *I;
			if (const clang::FunctionDecl *FD = clang::dyn_cast<clang::FunctionDecl>(D)) {
				// std::string Name = FD->getNameInfo().getName().getAsString();
				// clang::DiagnosticsEngine &Diag = CI.getDiagnostics();
				// unsigned int ID = Diag.getCustomDiagID(DiagnosticsEngine::Warning, "function declaration found");
				// Diag.Report(FD->getLocation(), ID);

				if (!FD->hasBody())
					continue;

				const clang::Stmt *body = FD->getBody();
				this->traverseStatements(body);
			}
		}
		return true;
	}
};

class HelloWorldAction: public clang::PluginASTAction {
public:
	std::unique_ptr<clang::ASTConsumer> CreateASTConsumer(clang::CompilerInstance &CI, clang::StringRef file) override {
		return std::make_unique<HelloWorldConsumer>(CI);
	}

	bool ParseArgs(const clang::CompilerInstance &CI, const std::vector<std::string> &args) override {
		return true;
	}

	clang::PluginASTAction::ActionType getActionType() override {
		return clang::PluginASTAction::ActionType::AddAfterMainAction;
	}
};

static clang::FrontendPluginRegistry::Add<HelloWorldAction> X("hello-world", "Hello World");

