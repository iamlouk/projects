use std::rc::Rc;

use crate::{core::{Value, Type, Error}, ast::{NodeRef, BinOp}};


pub struct Env {
    globals: std::collections::HashMap<&'static str, Value>,
    locals: Vec<(Rc<str>, Value)>,
}

impl Env {
    pub fn new() -> Self {
        let mut globals = std::collections::HashMap::<&'static str, Value>::new();
        globals.insert("Int", Value::Type(Type::Integer));
        globals.insert("Bool", Value::Type(Type::Boolean));
        globals.insert("String", Value::Type(Type::String));
        Self {
            globals,
            locals: Vec::with_capacity(16)
        }
    }

    fn lookup(&self, name: &str) -> Option<Value> {
        let local = self.locals.iter().rev().find(|l| l.0.as_ref() == name);
        if let Some((_, val)) = local {
            return Some(val.clone())
        }
        self.globals.get(name).cloned()
    }

    pub fn add_global(&mut self, name: &'static str, value: Value) {
        assert!(!self.globals.contains_key(name));
        self.globals.insert(name, value);
    }

    fn push(&mut self, name: &Rc<str>, value: Value) {
        self.locals.push((name.clone(), value));
    }

    fn pop(&mut self, n: usize) {
        assert!(self.locals.len() >= n);
        for _ in 0..n {
            self.locals.pop();
        }
    }

    pub fn eval(&mut self, node: &NodeRef) -> Result<Value, Error> {
        use crate::ast::Node;
        match &*node.borrow() {
            Node::Id { name, .. } => self.lookup(name.as_ref()).ok_or(Error::UndefinedValue(name.clone())),
            Node::Integer { value, .. } => Ok(Value::Integer(*value)),
            Node::Boolean { value, .. } => Ok(Value::Boolean(*value)),
            Node::String { value, .. } => Ok(Value::String(value.clone())),
            Node::BinOp { op, lhs, rhs, .. } => Ok(match (op, self.eval(lhs)?, self.eval(rhs)?) {
                (BinOp::Add, Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs + rhs),
                (BinOp::Sub, Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs - rhs),
                (BinOp::Mul, Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs * rhs),
                (BinOp::Div, Value::Integer(lhs), Value::Integer(rhs)) => Value::Integer(lhs / rhs),
                (BinOp::LT, Value::Integer(lhs), Value::Integer(rhs)) => Value::Boolean(lhs < rhs),
                (BinOp::LE, Value::Integer(lhs), Value::Integer(rhs)) => Value::Boolean(lhs <= rhs),
                _ => unimplemented!()
            }),
            Node::Call { callable, args, .. } => match self.eval(callable)? {
                Value::Lambda(argnames, body) if argnames.len() == args.len() => {
                    for (i, (name, _)) in argnames.iter().enumerate() {
                        let arg = self.eval(&args[i])?;
                        self.push(name, arg);
                    }

                    let value = self.eval(&body)?;
                    self.pop(args.len());
                    Ok(value)
                },
                _ => Err(Error::Uncallable(callable.clone()))
            },
            Node::IfThenElse { op0, op1, op2, .. } => match self.eval(op0)? {
                Value::Boolean(true) => self.eval(op1),
                Value::Boolean(false) => self.eval(op2),
                _ => unimplemented!()
            },
            Node::LetIn { name, value, body, .. } => {
                let value = self.eval(value)?;
                self.push(name, value);
                let res = self.eval(body)?;
                self.pop(1);
                Ok(res)
            },
            Node::Lambda { args, body, .. } => Ok(Value::Lambda(args.clone(), body.clone())),
            _ => unimplemented!()
        }
    }
}

