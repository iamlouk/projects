use std::rc::Rc;
use std::fmt::Write;

use crate::lexer::{Pos};

#[derive(Debug)]
pub enum Type {
    Unkown,
    Unresolved(Rc<String>),
    Error,

    Int,
    Real,
    Bool,
    Str,
    Lambda(Vec<Type>, Box<Type>)
}

#[derive(Debug)]
pub enum BinOp {
    And, Or,
    Eq, NotEq,
    Lt, LtEq,
    Gt, GtEq,
    Add, Sub,
    Mul, Div,
}

#[derive(Debug)]
pub struct Metadata {
    pub pos: Pos,
    pub ttype: Type
}

#[derive(Debug)]
pub enum Node {
    Int(Metadata, i64),
    Real(Metadata, f64),
    Bool(Metadata, bool),
    Str(Metadata, Rc<String>),
    Id(Metadata, Rc<String>),
    BinOp(Metadata, BinOp, Box<Node>, Box<Node>),
    Call(Metadata, Box<Node>, Vec<Node>),
    LetIn(Metadata, Rc<String>, Box<Node>, Box<Node>),
    IfThenElse(Metadata, Box<Node>, Box<Node>, Box<Node>),
    Lambda(Metadata, Vec<(Rc<String>, Type)>, Box<Node>)
}

impl Node {
    pub fn to_string(&self, out: &mut String) -> std::fmt::Result {
        match self {
            Self::Int(_, x) => write!(out, "{:?}", x),
            Self::Real(_, x) => write!(out, "{:?}", x),
            Self::Id(_, x) => write!(out, "id:{}", x),
            Self::BinOp(_, op, lhs, rhs) => {
                out.push('(');
                lhs.to_string(out)?;
                out.push_str(match op {
                    BinOp::And => " and ", BinOp::Or => " or ",
                    BinOp::Eq => " == ", BinOp::NotEq => " != ",
                    BinOp::Lt => " < ", BinOp::LtEq => " <= ",
                    BinOp::Gt => " > ", BinOp::GtEq => " >= ",
                    BinOp::Add => " + ", BinOp::Sub => " - ", BinOp::Mul => " * ", BinOp::Div => " / "
                });
                rhs.to_string(out)?;
                out.push(')');
                Ok(())
            },
            Self::LetIn(_, id, ea, eb) => {
                write!(out, "\nlet id:{} = ", id)?;
                ea.to_string(out)?;
                out.push_str(" in ");
                eb.to_string(out)
            },
            _ => unimplemented!()
        }
    }
}
