#![feature(assert_matches)]
#![allow(clippy::type_complexity)]

use std::io::Read;

mod ast;
mod core;
mod eval;
mod lex;

use eval::Env;

use crate::ast::Parser;
use crate::lex::Lexer;

fn main() {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .expect("I/O failure");

    let mut env = Env::new();
    eval::add_builtins(&mut env);
    let mut lexer = Lexer::new(buf.as_str(), 0, &mut env.string_pool);
    let mut parser = Parser::new(&mut lexer);
    let mut node = match parser.parse_all() {
        Ok(node) => node,
        Err(e) => {
            eprintln!("parsing failed: {:?}", e);
            std::process::exit(1)
        }
    };
    println!("# AST: {}", node.as_ref());
    let typ = match node.typecheck(&mut env, None) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("type-check failed: {:?}", e);
            std::process::exit(1)
        }
    };
    println!("# TYP: {}", typ);

    match env.eval(node.as_ref()) {
        Ok(val) => println!("{}", val),
        Err(e) => eprintln!("{:?}", e),
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn examples() {
        let mut env = Env::new();
        eval::add_builtins(&mut env);

        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("examples");
        for entry in std::fs::read_dir(&path).unwrap() {
            let (expected, name) = match entry {
                Ok(entry) if entry.file_name().to_str().unwrap().ends_with(".expected") => (
                    std::fs::read_to_string(entry.path()).unwrap(),
                    entry.file_name().into_string().unwrap(),
                ),
                Ok(_) => continue,
                Err(e) => panic!("{:?}", e),
            };
            let name = name.strip_suffix(".expected").unwrap();
            path.push(name.to_string() + ".dhall");
            let sourcecode = std::fs::read_to_string(&path).unwrap();
            eprintln!("testing: {}", name);

            let mut lexer = Lexer::new(sourcecode.as_str(), 0, &mut env.string_pool);
            let mut parser = Parser::new(&mut lexer);
            let mut example = parser.parse_all().expect("parsing failed");

            let typ = example
                .typecheck(&mut env, None)
                .expect("type check failed");
            let val = env.eval(&example).expect("evaluation failed");

            let res = format!("{}", val);
            assert_eq!(expected.trim(), res.trim());
            assert_eq!(typ.as_ref(), val.get_type(&env).as_ref());
            path.pop();
        }
    }
}
