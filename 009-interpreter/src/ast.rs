use std::rc::Rc;
use std::fmt::Write;
use std::collections::HashMap;

use crate::lexer::{Pos};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Unkown,
    Unresolved(Rc<Node>),
    Type(Rc<Type>),
    Int,
    Real,
    Bool,
    Str,
    Lambda(Vec<Rc<Type>>, Rc<Type>)
}

#[derive(Debug)]
pub enum TCError {
    Unresolvable(Pos),
    OperandsDoNotMatch(Pos),
    FunctionArgsDoNotMatch(Pos),
    ExpectedBool(Pos)
}

impl Type {
    pub fn to_string(&self, out: &mut String) -> std::fmt::Result {
        match self {
            Self::Unkown => write!(out, "<?>"),
            Self::Unresolved(x) => {
                write!(out, "<")?;
                x.to_string(out)?;
                write!(out, ">")
            },
            Self::Type(_) => write!(out, "Type"),
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

    pub fn resolve(&self, env: &Env<Self>) -> Result<Self, TCError> {
        match self {
            Self::Unresolved(node) => match node.as_ref() {
                Node::Id(md, name) => match env.lookup(name) {
                    Some(Type::Type(t)) => Ok(t.as_ref().clone()),
                    Some(_) => unimplemented!(),
                    None => Err(TCError::Unresolvable(md.pos))
                },
                _ => unimplemented!()
            },
            _ => unimplemented!()
        }
    }
}

pub struct Env<V> {
    // TODO: Smarter DS with less copies:
    scopes: Vec<HashMap<String, V>>
}

impl<V> Env<V> {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()]
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn lookup(&self, key: &str) -> Option<&V> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(key) {
                return Some(val)
            }
        }
        None
    }

    pub fn add(&mut self, key: &str, val: V) {
        let scope = self.scopes.last_mut().unwrap();
        scope.insert(key.to_string(), val);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    And, Or,
    Eq, NotEq,
    Lt, LtEq,
    Gt, GtEq,
    Add, Sub,
    Mul, Div,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub pos: Pos,
    pub ttype: Type
}

#[derive(Debug, Clone, PartialEq)]
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
    pub fn check_types(&mut self, env: &mut Env<Type>) -> Result<&Type, TCError> {
        match self {
            Self::Int(md, _) | Self::Real(md, _) | Self::Bool(md, _) | Self::Str(md, _)
                => Ok(&md.ttype),
            Self::Id(md, id) => match env.lookup(id.as_str()) {
                Some(ttype) => { md.ttype = ttype.clone(); Ok(&md.ttype) },
                None => Err(TCError::Unresolvable(md.pos))
            },
            Self::BinOp(md, op, lhs, rhs) => {
                let lhs = lhs.check_types(env)?;
                let rhs = rhs.check_types(env)?;
                let ttype = match (op, lhs.clone(), rhs.clone()) {
                    (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div,
                        Type::Int, Type::Int) => Type::Int,
                    (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div,
                        Type::Real, Type::Real) => Type::Real,
                    (BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq,
                        Type::Int, Type::Int) => Type::Bool,
                    (BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq,
                        Type::Real, Type::Real) => Type::Bool,
                    (BinOp::And | BinOp::Or, Type::Bool, Type::Bool) => Type::Bool,
                    (BinOp::And | BinOp::Or, Type::Int, Type::Int) => Type::Bool,
                    (_, _, _) => { return Err(TCError::OperandsDoNotMatch(md.pos)); }
                };
                md.ttype = ttype;
                Ok(&md.ttype)
            },
            Self::LetIn(md, name, expr1, expr2) => {
                let ttype = expr1.check_types(env)?;
                env.push_scope();
                env.add(&name, ttype.clone());
                let ttype = expr2.check_types(env)?;
                env.pop_scope();
                md.ttype = ttype.clone();
                Ok(&md.ttype)
            },
            Self::Lambda(md, args, body) => {
                env.push_scope();
                let mut argtypes = Vec::with_capacity(args.len());
                for arg in args {
                    let ttype = arg.1.resolve(env)?;
                    argtypes.push(Rc::new(ttype.clone()));
                    env.add(arg.0.as_str(), ttype);
                }
                let rettype = body.check_types(env)?;
                env.pop_scope();
                md.ttype = Type::Lambda(argtypes, Rc::new(rettype.clone()));
                Ok(&md.ttype)
            },
            Self::Call(md, callee, args) => {
                let calleetype = callee.check_types(env)?;
                let (argtypes, rettype) = match calleetype {
                    Type::Lambda(argtypes, rettype) => (argtypes, rettype),
                    _ => { return Err(TCError::FunctionArgsDoNotMatch(md.pos)); }
                };
                md.ttype = rettype.as_ref().clone();
                if args.len() != argtypes.len() {
                    return Err(TCError::FunctionArgsDoNotMatch(md.pos));
                }
                for i in 0..args.len() {
                    if argtypes[i].as_ref() != args[i].check_types(env)? {
                        return Err(TCError::FunctionArgsDoNotMatch(md.pos));
                    }
                }

                Ok(&md.ttype)
            },
            Self::If(md, cond, iftrue, iffalse) => {
                if *cond.check_types(env)? != Type::Bool {
                    return Err(TCError::ExpectedBool(md.pos));
                }

                let t1 = iftrue.check_types(env)?;
                let t2 = iffalse.check_types(env)?;
                if t1 == t2 {
                    return Err(TCError::OperandsDoNotMatch(md.pos));
                }

                md.ttype = t1.clone();
                Ok(&md.ttype)
            }
        }
    }

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
