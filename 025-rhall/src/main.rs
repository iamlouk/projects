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

    let mut env = Env::new();
    match env.eval(&node) {
        Ok(val) => println!("{}", val),
        Err(e) => eprintln!("{:?}", e),
    };
}
