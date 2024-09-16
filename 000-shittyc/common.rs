use std::{fmt::Display, rc::Rc};

use crate::lex::Tok;

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct SLoc {
    pub file: Rc<std::path::Path>,
    pub line: u32,
    pub col: u32,
}

impl SLoc {
    pub fn new(file: &std::path::Path, line: usize, col: usize) -> Self {
        Self {
            file: Rc::from(file),
            line: line.min(u32::MAX as usize) as u32,
            col: col.min(u32::MAX as usize) as u32,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    IO(SLoc, std::io::Error),
    EndOfFile(SLoc),
    PreProcessor(SLoc, Tok, &'static str),
    InvalidInt(SLoc, std::num::ParseIntError),
    InvalidTok(SLoc, &'static str),
    Lex(SLoc, String)
}

pub enum Type {
    Void,
    Int64,
    Ptr(Rc<Type>),
    Fn(Rc<Type>, Vec<Rc<Type>>)
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Int64 => write!(f, "uint64"),
            Type::Ptr(elm) => write!(f, "*{}", &**elm),
            Type::Fn(retty, argtys) => {
                write!(f, "fn(")?;
                for (i, ty) in argtys.iter().enumerate() {
                    write!(f, "{}{}", if i == 0 { "" } else { ", " }, &**ty)?;
                }
                write!(f, "): {}", &**retty)?;
                Ok(())
            }
        }
    }
}
