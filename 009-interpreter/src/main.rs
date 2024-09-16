use std::rc::Rc;

mod lexer;
mod ast;
mod parser;
mod interpreter;

fn main() {
    println!("Hello, world!");

    //let code = "
    //    let add = L(a: Int, b: Int) -> a + b in
    //    let double = L(x: Int) -> add(x, x) in double(21)
    //";

    let code = "let add = L(a: Int, b: Int) -> a + b in add(21, 21)";

    let lexer = lexer::Lexer::new(0, code);
    let mut parser = parser::Parser::new(lexer);
    let mut ast = match parser.parse_expr() {
        Ok(ast) => {
            let mut buf = String::new();
            ast.to_string(&mut buf).unwrap();
            println!("{}", buf);
            ast
        },
        Err(e) => {
            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    };

    let mut env = ast::Env::<ast::Type>::new();
    env.push_scope();
    env.add("PI", ast::Type::Int);
    env.add("Int", ast::Type::Type(Rc::new(ast::Type::Int)));
    env.add("Real", ast::Type::Type(Rc::new(ast::Type::Real)));
    env.add("Bool", ast::Type::Type(Rc::new(ast::Type::Bool)));
    env.add("Type", ast::Type::Type(Rc::new(ast::Type::Type(Rc::new(ast::Type::Unkown)))));
    let ttype = ast.check_types(&mut env);
    println!("{:?}", ttype);
    env.pop_scope();

    let mut env = ast::Env::<interpreter::Value>::new();
    env.push_scope();
    println!("{:?}", ast.run(&mut env));
    env.pop_scope();
}
