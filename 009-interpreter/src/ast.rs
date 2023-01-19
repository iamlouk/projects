use std::rc::Rc;
use std::fmt::Write;

use crate::lexer::{Pos};

#[derive(Debug)]
pub enum Type {
    Unkown,
    Unresolved(Box<Node>),
    Type,
    Int,
    Real,
    Bool,
    Str,
    Lambda(Vec<Type>, Box<Type>)
}

impl Type {
    pub fn to_string(&self, out: &mut String) -> std::fmt::Result {
        match self {
            Self::Unkown => write!(out, "<?>"),
            Self::Unresolved(name) => write!(out, "<{:?}>", name),
            Self::Type => write!(out, "Type"),
            Self::Int => write!(out, "Int"),
            Self::Real => write!(out, "Real"),
            Self::Bool => write!(out, "Bool"),
            Self::Str => write!(out, "Str"),
            Self::Lambda(argtypes, rettype) => {
                out.push_str("lambda:(");
                for i in 0..argtypes.len() {
                    if i != 0 { out.push_str(", "); }
                    argtypes[i].to_string(out)?;
                }
                out.push_str(") => ");
                rettype.to_string(out)?;
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    If(Metadata, Box<Node>, Box<Node>, Box<Node>),
    Lambda(Metadata, Vec<(Rc<String>, Type)>, Box<Node>)
}

impl Node {
    pub fn to_string(&self, out: &mut String) -> std::fmt::Result {
        match self {
            Self::Int(_, x) => write!(out, "{:?}", x),
            Self::Real(_, x) => write!(out, "{:?}", x),
            Self::Bool(_, true) => write!(out, "true"),
            Self::Bool(_, false) => write!(out, "false"),
            Self::Str(_, str) => write!(out, "{:?}", str),
            Self::Id(_, x) => write!(out, "{}", x),
            Self::BinOp(_, op, lhs, rhs) => {
                out.push('(');
                lhs.to_string(out)?;
                out.push_str(match op {
                    BinOp::And => " and ", BinOp::Or => " or ",
                    BinOp::Eq => " == ", BinOp::NotEq => " != ",
                    BinOp::Lt => " < ", BinOp::LtEq => " <= ",
                    BinOp::Gt => " > ", BinOp::GtEq => " >= ",
                    BinOp::Add => " + ", BinOp::Sub => " - ",
                    BinOp::Mul => " * ", BinOp::Div => " / "
                });
                rhs.to_string(out)?;
                out.push(')');
                Ok(())
            },
            Self::Call(_, func, args) => {
                out.push('(');
                func.to_string(out)?;
                out.push_str(")(");
                for i in 0..args.len() {
                    if i != 0 { out.push_str(", "); }
                    args[i].to_string(out)?;
                }
                out.push(')');
                Ok(())
            },
            Self::LetIn(_, id, ea, eb) => {
                write!(out, "\nlet {} = ", id)?;
                ea.to_string(out)?;
                out.push_str(" in ");
                eb.to_string(out)
            },
            Self::If(_, cond, iftrue, iffalse) => {
                out.push_str("(if (");
                cond.to_string(out)?;
                out.push_str(") then (");
                iftrue.to_string(out)?;
                out.push_str(") else (");
                iffalse.to_string(out)?;
                out.push_str(")");
                Ok(())
            },
            Self::Lambda(_, args, body) => {
                out.push_str("(");
                for i in 0..args.len() {
                    if i != 0 { out.push_str(", "); }
                    write!(out, "{}: ", args[i].0.as_str())?;
                    args[i].1.to_string(out)?;
                }
                out.push_str(") -> (");
                body.to_string(out)?;
                out.push_str(")");
                Ok(())
            }
        }
    }
}
