#![feature(assert_matches)]

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

    let mut spool = std::collections::HashSet::<std::rc::Rc<str>>::new();
    let mut lexer = Lexer::new(buf.as_str(), 0, &mut spool);
    let mut parser = Parser::new(&mut lexer);
    let node = parser.parse_all().expect("parsing failure");
    println!("# AST: {}", node.as_ref().borrow());

    let mut env = Env::new();
    match env.eval(&node) {
        Ok(val) => println!("{}", val),
        Err(e) => eprintln!("{:?}", e),
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn examples() {
        let mut spool = std::collections::HashSet::<std::rc::Rc<str>>::new();
        let mut env = Env::new();

        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("examples");
        for entry in std::fs::read_dir(&path).unwrap() {
            let (expected, name) = match entry {
                Ok(entry) if entry.file_name().to_str().unwrap().ends_with(".expected") => (
                    std::fs::read_to_string(entry.path()).unwrap(),
                    entry.file_name().into_string().unwrap()),
                Ok(_) => continue,
                Err(e) => panic!("{:?}", e)
            };
            let name = name.strip_suffix(".expected").unwrap();
            path.push(name.to_string() + ".dhall");
            let sourcecode = std::fs::read_to_string(&path).unwrap();
            eprintln!("running example: {}", name);

            let mut lexer = Lexer::new(sourcecode.as_str(), 0, &mut spool);
            let mut parser = Parser::new(&mut lexer);
            let example = parser.parse_all().expect("parsing failed");
            let res = format!("{}", env.eval(&example).expect("evaluation failed"));

            assert_eq!(expected.trim(), res.trim());

            path.pop();
        }
    }
}
