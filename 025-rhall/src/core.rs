use std::{
    fmt::{Debug, Display},
    rc::Rc, cell::RefCell,
};

use crate::{lex, eval::Env, ast::Node};

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
    Uncallable(SLoc, String),
    ExpectedType(SLoc),
    TypeError(SLoc, String),
}

// TODO: Do something string_pool like for types?
// As types will basically never change but be shared/cross-reference a lot
// that would be nice and beneficial.
#[derive(Debug, Clone)]
pub enum Type {
    Boolean,
    Integer,
    String,
    Type(Option<Rc<Type>>),
    Lambda(Vec<(Rc<str>, Rc<Type>)>, Rc<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Boolean => write!(f, "Bool"),
            Type::Integer => write!(f, "Int"),
            Type::String => write!(f, "Str"),
            Type::Type(None) => write!(f, "Type"),
            Type::Type(Some(t)) => write!(f, "Type /* {} */", t.as_ref()),
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

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::Boolean, Type::Boolean) => true,
            (Type::Integer, Type::Integer) => true,
            (Type::String, Type::String) => true,
            (Type::Type(Some(t1)), Type::Type(Some(t2))) => t1 == t2,
            (Type::Type(_), Type::Type(_)) => true, // TODO: Check stuff like: 1 -> Int -> Type -> Kind...
            (Type::Lambda(args1, rettyp1), Type::Lambda(args2, rettyp2)) =>
                args1.len() == args2.len() &&
                args1.iter().zip(args2.iter()).all(|((_, t1), (_, t2))| t1.as_ref() == t2.as_ref()) &&
                rettyp1.as_ref() == rettyp2.as_ref(),
            (_, _) => false
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Pseudo(Rc<Type>),
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    Type(Rc<Type>),
    Lambda(Vec<(Rc<str>, Rc<Type>)>, Rc<RefCell<Node>>),
}

impl Value {
    pub fn get_type(&self, env: &Env) -> Rc<Type> {
        match self {
            Value::Pseudo(t) => t.clone(),
            Value::Bool(_) => env.bool_type.clone(),
            Value::Int(_) => env.int_type.clone(),
            Value::Str(_) => env.str_type.clone(),
            Value::Type(t) => Rc::new(Type::Type(Some(t.clone()))),
            Value::Lambda(args, body) => Rc::new(Type::Lambda(args.clone(), body.borrow().get_type().unwrap())),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Pseudo(t) => write!(f, "(PSEUDO:{})", t.as_ref()),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Int(x) => write!(f, "{}", x),
            Value::Str(s) => write!(f, "{:?}", s.as_ref()),
            Value::Type(t) => Display::fmt(t.as_ref(), f),
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
