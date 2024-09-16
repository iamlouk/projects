use std::rc::Rc;

mod lexer;
mod ast;
mod parser;
mod interpreter;

use interpreter::Value;

fn main() {
    let print_ast = false;
    let print_type = false;

    //let code = "
    //    let add = L(a: Int, b: Int) -> a + b in
    //    let double = L(x: Int) -> add(x, x) in double(21)
    //";

    // let code = "let add = L(a: Int, b: Int) -> a + b in add(21, 21)";

    let code = "let id = L(a: Type) -> L(x: a) -> x in id(Int)(42) ";

    let lexer = lexer::Lexer::new(0, code);
    let mut parser = parser::Parser::new(lexer);
    let mut ast = match parser.parse() {
        Ok(ast) => {
            if print_ast {
                let mut buf = String::new();
                ast.to_string(&mut buf).unwrap();
                println!("{}", buf);
            }
            ast
        },
        Err(e) => {
            eprintln!("Parser/Lexer Error: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut env = ast::Env::<ast::Type>::new();
    env.push_scope();
    env.add("PI", ast::Type::Int);
    env.add("Int", ast::Type::Type(Rc::new(ast::Type::Int)));
    env.add("Type", ast::Type::Kind);
    let _ = match ast.check_types(&mut env) {
        Ok(ttype) => {
            if print_type {
                println!("{:?}", ttype);
            }
            ttype
        },
        Err(e) => {
            eprintln!("Type Error: {:?}", e);
            std::process::exit(2);
        },
    };
    env.pop_scope();

    let mut env = ast::Env::<Value>::new();
    env.push_scope();
    env.add("PI", Value::Real(std::f64::consts::PI));
    env.add("Int", Value::Type(ast::Type::Int));
    env.add("Type", Value::Kind);
    println!("{:?}", ast.run(&mut env));
    env.pop_scope();
}
