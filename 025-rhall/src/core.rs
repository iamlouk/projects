use std::{rc::Rc, fmt::{Display, Debug}};

use crate::{lex, ast::NodeRef, ast::Node};

#[derive(Clone, Copy, Debug)]
pub struct SLoc {
    pub line: u32,
    pub col: u16,
    pub file_id: u16
}

#[derive(Debug, Clone)]
pub enum Error {
    Lexer(SLoc, String),
    Parser(SLoc, String),
    UnexpectedEOF,
    ExpectedToken { sloc: SLoc, expected: lex::Tok, found: lex::Tok },
    UndefinedValue(Rc<str>),
    Uncallable(NodeRef),
}

// TODO: Do something string_pool like for types?
// As types will basically never change but be shared/cross-reference a lot
// that would be nice and beneficial.
#[derive(Debug, Clone)]
pub enum Type {
    Unresolved(Option<NodeRef>),
    Boolean,
    Integer,
    String,
    // List(Rc<Type>),
    // Record(Rc<[(Rc<str>, Rc<Type>)]>),
    Lambda(Rc<[Rc<Type>]>, Rc<Type>)
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Unresolved(None) => write!(f, "???"),
            Type::Unresolved(Some(node)) => node.fmt(f),
            Type::Boolean => write!(f, "Bool"),
            Type::Integer => write!(f, "Int"),
            Type::String => write!(f, "String"),
            Type::Lambda(args, rettyp) => {
                write!(f, "∀(")?;
                for (i, argtyp) in args.iter().enumerate() {
                    write!(f, "{}{}", if i != 0 { ", " } else { "" }, argtyp)?;
                }
                write!(f, ") -> {}", rettyp)
            }
        }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::new();
        self.stringify("", &mut buf)?;
        write!(f, "{}", buf)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Boolean(bool),
    Integer(i64),
    String(Rc<str>),
    Type(Type),
    Lambda(Vec<(Rc<str>, Type)>, NodeRef),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Boolean(true) => write!(f, "true"),
            Value::Boolean(false) => write!(f, "false"),
            Value::Integer(x) => write!(f, "{}", x),
            Value::String(s) => write!(f, "{:?}", s.as_ref()),
            Value::Type(t) => Debug::fmt(t, f),
            Value::Lambda(args, node) => {
                write!(f, "λ(")?;
                for (i, (name, typ)) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, "{}: {}, ", name.as_ref(), typ)?;
                    } else {
                        write!(f, "{}: {}", name.as_ref(), typ)?;
                    }
                }
                write!(f, ") -> {}", node.as_ref().borrow())
            },
        }
    }
}

