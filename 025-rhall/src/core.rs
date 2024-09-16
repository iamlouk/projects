use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{ast::NodeRef, lex};

#[derive(Clone, Copy, Debug, Default)]
pub struct SLoc {
    pub line: u32,
    pub col: u16,
    pub file_id: u16,
}

#[derive(Debug, Clone)]
pub enum Error {
    Lexer(SLoc, String),
    Parser(SLoc, String),
    UnexpectedEOF,
    ExpectedToken {
        sloc: SLoc,
        expected: lex::Tok,
        found: lex::Tok,
    },
    UndefinedValue(Rc<str>),
    Uncallable(NodeRef),
    ExpectedType(SLoc)
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
    Type,
    Lambda(Vec<(Rc<str>, Rc<Type>)>, Rc<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Unresolved(None) => write!(f, "???"),
            Type::Unresolved(Some(node)) => write!(f, "{} /* Unresolved */", node.as_ref().borrow()),
            Type::Boolean => write!(f, "Bool"),
            Type::Integer => write!(f, "Int"),
            Type::String => write!(f, "Str"),
            Type::Type => write!(f, "Type"),
            Type::Lambda(args, rettyp) => {
                write!(f, "∀(")?;
                for (i, (name, argtyp)) in args.iter().enumerate() {
                    write!(f, "{}{}: {}", if i != 0 { ", " } else { "" }, name.as_ref(), argtyp)?;
                }
                write!(f, ") -> {}", rettyp)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    Type(Option<Rc<str>>, Rc<Type>),
    Lambda(Vec<(Rc<str>, Rc<Type>)>, NodeRef),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Int(x) => write!(f, "{}", x),
            Value::Str(s) => write!(f, "{:?}", s.as_ref()),
            Value::Type(None, t) => Display::fmt(t.as_ref(), f),
            Value::Type(Some(name), t) => write!(f, "{} /* supertype: {} */", name.as_ref(), t.as_ref()),
            Value::Lambda(args, node) => {
                write!(f, "λ(")?;
                for (i, (name, typ)) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", {}: {}", name.as_ref(), typ)?;
                    } else {
                        write!(f, "{}: {}", name.as_ref(), typ)?;
                    }
                }
                write!(f, ") -> {}", node.as_ref().borrow())
            }
        }
    }
}
