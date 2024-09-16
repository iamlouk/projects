use crate::ast::*;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Real(f64),
    Bool(bool),
    Str(Rc<String>),
    Lambda(Vec<Rc<String>>, Rc<Node>)
}

impl Node {
    pub fn run(&self, env: &mut Env<Value>) -> Value {
        match self {
            Self::Int(_, x) => Value::Int(*x),
            Self::Real(_, x) => Value::Real(*x),
            Self::Bool(_, x) => Value::Bool(*x),
            Self::Str(_, x) => Value::Str(x.clone()),
            Self::Id(_, name) => env.lookup(name.as_str())
                .expect("typecheck should have found this...").clone(),
            Self::BinOp(_, op, lhs, rhs) => match (op, lhs.run(env), rhs.run(env)) {
                (BinOp::Add, Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
                (BinOp::Add, Value::Real(lhs), Value::Real(rhs)) => Value::Real(lhs + rhs),
                _ => unimplemented!()
            },
            Self::Call(_, callee, args) => match callee.run(env) {
                Value::Lambda(argnames, body) => {
                    env.push_scope();
                    assert!(args.len() == argnames.len());
                    for i in 0..args.len() {
                        let arg = args[i].run(env);
                        env.add(argnames[i].as_str(), arg);
                    }
                    let val = body.run(env);
                    env.pop_scope();
                    val
                },
                _ => panic!("typecheck should have found this...")
            },
            Self::LetIn(_, name, expr1, expr2) => {
                let val = expr1.run(env);
                env.push_scope();
                env.add(name.as_str(), val);
                let val = expr2.run(env);
                env.pop_scope();
                val
            },
            Self::If(_, cond, iftrue, ifflase) => match cond.run(env) {
                Value::Bool(true) => iftrue.run(env),
                Value::Bool(false) => ifflase.run(env),
                _ => panic!("typecheck should have found this...")
            },
            Self::Lambda(_, args, body) => {
                let mut lambdaargs = Vec::with_capacity(args.len());
                for i in 0..args.len() {
                    lambdaargs.push(args[i].0.clone());
                }

                Value::Lambda(lambdaargs, body.clone())
            },
        }
    }
}

