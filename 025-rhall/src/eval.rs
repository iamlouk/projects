use std::rc::Rc;

use crate::{
    ast::{BinOp, NodeRef},
    core::{Error, Type, Value},
};

pub struct Env {
    // TODO: Add the string_pool hashset here or to a runtime object.
    // TODO: Add pre-allocated type RCs here that can be used by the parser!
    globals: std::collections::HashMap<&'static str, Value>,
    locals: Vec<(Rc<str>, Value)>,
}

impl Env {
    pub fn new() -> Self {
        let mut globals = std::collections::HashMap::<&'static str, Value>::new();
        globals.insert("Int", Value::Type(None, Rc::new(Type::Integer)));
        globals.insert("Bool", Value::Type(None, Rc::new(Type::Boolean)));
        globals.insert("Str", Value::Type(None, Rc::new(Type::String)));
        globals.insert("Type", Value::Type(None, Rc::new(Type::Type)));
        Self {
            globals,
            locals: Vec::with_capacity(16),
        }
    }

    fn lookup(&self, name: &str) -> Option<Value> {
        let local = self.locals.iter().rev().find(|l| l.0.as_ref() == name);
        if let Some((_, val)) = local {
            return Some(val.clone());
        }
        self.globals.get(name).cloned()
    }

    #[allow(unused)]
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
            Node::Id { name, .. } => self
                .lookup(name.as_ref())
                .ok_or(Error::UndefinedValue(name.clone())),
            Node::Integer { value, .. } => Ok(Value::Int(*value)),
            Node::Boolean { value, .. } => Ok(Value::Bool(*value)),
            Node::String { value, .. } => Ok(Value::Str(value.clone())),
            Node::Invert { op0, .. } => Ok(match self.eval(op0)? {
                Value::Bool(value) => Value::Bool(!value),
                Value::Int(value) => Value::Int(!value),
                _ => unimplemented!()
            }),
            Node::BinOp { op, lhs, rhs, .. } => Ok(match (op, self.eval(lhs)?, self.eval(rhs)?) {
                (BinOp::Add, Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
                (BinOp::Sub, Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
                (BinOp::Mul, Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs * rhs),
                (BinOp::Div, Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs / rhs),
                (BinOp::EQ, Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs == rhs),
                (BinOp::NE, Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs == rhs),
                (BinOp::LT, Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs < rhs),
                (BinOp::LE, Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs <= rhs),
                (op, lhs, rhs) => panic!("op: {:?}, lhs: {:?}, rhs: {:?}", op, lhs, rhs),
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
                }
                _ => Err(Error::Uncallable(callable.clone())),
            },
            Node::IfThenElse { op0, op1, op2, .. } => match self.eval(op0)? {
                Value::Bool(true) => self.eval(op1),
                Value::Bool(false) => self.eval(op2),
                _ => unimplemented!(),
            },
            Node::LetIn {
                name, value, body, ..
            } => {
                let value = self.eval(value)?;
                self.push(name, value);
                let res = self.eval(body)?;
                self.pop(1);
                Ok(res)
            }
            Node::Lambda { args, body, .. } => Ok(Value::Lambda(args.clone(), body.clone())),
            Node::Forall { sloc, argtypes, rettyp, .. } => {
                let mut args = Vec::with_capacity(argtypes.len());
                for (name, argtyp) in argtypes.iter() {
                    let typval = self.eval(argtyp)?;
                    let typ = match typval {
                        Value::Type(_, ref t) => t.clone(),
                        _ => return Err(Error::ExpectedType(*sloc))
                    };
                    self.push(name, typval);
                    args.push((name.clone(), typ));
                }

                let rettyp = match self.eval(rettyp)? {
                    Value::Type(_, t) => t,
                    _ => return Err(Error::ExpectedType(*sloc))
                };

                let res = Value::Type(None, Rc::new(Type::Lambda(args, rettyp)));
                self.pop(argtypes.len());
                Ok(res)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;
    use crate::ast::Parser;
    use crate::lex::Lexer;

    fn parse(input: &'static str) -> Result<NodeRef, Error> {
        let mut spool = std::collections::HashSet::<Rc<str>>::new();
        let mut lexer = Lexer::new(input, 0, &mut spool);
        let mut parser = Parser::new(&mut lexer);
        parser.parse_all()
    }

    #[test]
    fn incto42() {
        let mut env = Env::new();
        let expr = parse("let inc = λ(x: Int) -> x + 1 in inc(41)").unwrap();
        assert_matches!(env.eval(&expr), Ok(Value::Int(42)));
    }

    #[test]
    fn fib10() {
        let mut env = Env::new();
        let expr =
            parse("let fib = λ(n: Int) -> if n < 2 then n else fib(n - 1) + fib(n - 2) in fib(10)")
                .unwrap();
        assert_matches!(env.eval(&expr), Ok(Value::Int(55)));
    }
}
