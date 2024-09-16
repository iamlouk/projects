use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    ast::{BinOp, Node},
    core::{Builtin, Error, Type, Value},
};

#[derive(Debug)]
pub struct Env {
    // TODO: Add the string_pool hashset here or to a runtime object.
    // TODO: Add pre-allocated type RCs here that can be used by the parser!
    globals: std::collections::HashMap<&'static str, Value>,
    pub locals: Vec<(Rc<str>, Value)>,
    pub string_pool: std::collections::HashSet<Rc<str>>,

    pub int_type: Rc<Type>,
    pub bool_type: Rc<Type>,
    pub str_type: Rc<Type>,
    pub type_type: Rc<Type>,
    pub any_type: Rc<Type>,
}

impl Env {
    pub fn new() -> Self {
        let mut env = Self {
            globals: HashMap::new(),
            locals: Vec::with_capacity(16),
            string_pool: HashSet::new(),

            int_type: Rc::new(Type::Integer),
            bool_type: Rc::new(Type::Boolean),
            str_type: Rc::new(Type::String),
            type_type: Rc::new(Type::TypeOfType),
            any_type: Rc::new(Type::Any),
        };
        env.globals.insert("Int", Value::Type(env.int_type.clone()));
        env.globals
            .insert("Bool", Value::Type(env.bool_type.clone()));
        env.globals.insert("Str", Value::Type(env.str_type.clone()));
        env.globals
            .insert("Type", Value::Type(env.type_type.clone()));
        env.globals.insert("Any", Value::Type(env.any_type.clone()));
        env
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
        let local = self.locals.iter().rev().find(|l| l.0.as_ref() == name);
        if let Some((_, val)) = local {
            return Some(val.clone());
        }
        self.globals.get(name).cloned()
    }

    pub fn stringify(&mut self, string: &str) -> Rc<str> {
        if let Some(s) = self.string_pool.get(string) {
            return s.clone();
        }

        let s: Rc<str> = Rc::from(string);
        self.string_pool.insert(s.clone());
        s
    }

    #[allow(unused)]
    pub fn add_global(&mut self, name: &'static str, value: Value) {
        assert!(!self.globals.contains_key(name));
        self.globals.insert(name, value);
    }

    pub fn push(&mut self, name: &Rc<str>, value: Value) {
        self.locals.push((name.clone(), value));
    }

    pub fn pop(&mut self, n: usize) {
        assert!(self.locals.len() >= n);
        for _ in 0..n {
            self.locals.pop();
        }
    }

    pub fn eval(&mut self, node: &Node) -> Result<Value, Error> {
        match node {
            Node::Id { name, .. } => self
                .lookup(name.as_ref())
                .ok_or(Error::UndefinedValue(name.clone())),
            Node::Integer { value, .. } => Ok(Value::Int(*value)),
            Node::Boolean { value, .. } => Ok(Value::Bool(*value)),
            Node::String { value, .. } => Ok(Value::Str(value.clone())),
            Node::TypeAnno { op0, .. } => self.eval(op0),
            Node::Invert { op0, .. } => Ok(match self.eval(op0)? {
                Value::Bool(value) => Value::Bool(!value),
                Value::Int(value) => Value::Int(!value),
                _ => unimplemented!(),
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
            Node::Call {
                sloc,
                callable,
                args,
                ..
            } => {
                let callable = self.eval(callable)?;
                let args: Result<Vec<_>, _> = args.iter().map(|arg| self.eval(arg)).collect();
                callable.apply(*sloc, self, args?)
            }
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
            Node::Lambda { args, body, .. } => Ok(Value::Lambda(
                args.iter()
                    .map(|(name, typ, _)| (name.clone(), typ.clone().unwrap()))
                    .collect(),
                body.clone(),
            )),
            Node::Forall {
                sloc,
                argtypes,
                rettyp,
                ..
            } => {
                let mut args = Vec::with_capacity(argtypes.len());
                for (name, argtyp, rawargtyp) in argtypes.iter() {
                    if let Some(typ) = argtyp {
                        self.push(name, Value::Type(typ.clone()));
                        args.push((name.clone(), typ.clone()));
                    } else {
                        let typval = self.eval(rawargtyp)?;
                        let typ = match typval {
                            Value::Type(ref t) => t.clone(),
                            _ => return Err(Error::ExpectedType(*sloc)),
                        };
                        self.push(name, typval);
                        args.push((name.clone(), typ));
                    }
                }

                let rettyp = match self.eval(&rettyp.borrow())? {
                    Value::Type(t) => t,
                    _ => return Err(Error::ExpectedType(*sloc)),
                };

                let res = Value::Type(Rc::new(Type::Lambda(args, rettyp)));
                self.pop(argtypes.len());
                Ok(res)
            }
            Node::Record { fields, .. } => {
                let fields: Result<Vec<_>, _> = fields
                    .iter()
                    .map(|(name, value)| self.eval(value).map(|v| (name.clone(), v)))
                    .collect();
                Ok(Value::Record(fields?))
            }
            Node::RecordType { typ, .. } => {
                // TODO: Now that type checks are always enabled and work well, something
                // like this should also work elsewhere:
                Ok(Value::Type(typ.clone().unwrap()))
            }
            Node::AccessField { op0, field, .. } => match self.eval(op0)? {
                Value::Record(fields) => Ok(fields
                    .iter()
                    .find(|(name, _)| name.as_ref() == field.as_ref())
                    .unwrap()
                    .1
                    .clone()),
                _ => panic!(),
            },
            Node::As { op0, as_typ, .. } => {
                match (self.eval(op0)?, as_typ.as_ref().unwrap().as_ref()) {
                    (v, Type::Any) => Ok(Value::Any(op0.get_type().unwrap(), Box::new(v))),
                    (Value::Any(truetype, v), t) if *truetype == *t => {
                        Ok(Value::Option(as_typ.clone().unwrap(), Some(v)))
                    }
                    (Value::Any(truetype, _), t) if *truetype != *t => {
                        Ok(Value::Option(as_typ.clone().unwrap(), None))
                    }
                    (a, b) => todo!("{} as {}", a, b),
                }
            }
        }
    }
}

pub fn add_builtins(env: &mut Env) {
    let x_str: Rc<str> = Rc::from("x");
    let a_str: Rc<str> = Rc::from("A");

    env.add_global(
        "Process/exit",
        Value::Builtin(Rc::new(Builtin {
            name: "Process/exit",
            argtypes: vec![(Rc::from("code"), env.int_type.clone())],
            rettyp: env.int_type.clone(),
            f: Box::new(|_env, args| {
                let code = match args[0] {
                    Value::Int(x) => x,
                    _ => panic!(),
                };
                std::process::exit(code as i32)
            }),
        })),
    );

    env.add_global(
        "Process/getenv",
        Value::Builtin(Rc::new(Builtin {
            name: "Process/getenv",
            argtypes: vec![(x_str.clone(), env.str_type.clone())],
            rettyp: Rc::new(Type::Option(env.str_type.clone())),
            f: Box::new(|env, args| {
                let x = match &args[0] {
                    Value::Str(x) => x,
                    _ => panic!(),
                };
                Ok(Value::Option(
                    env.str_type.clone(),
                    std::env::var(x.as_ref())
                        .ok()
                        .map(|x| Box::new(Value::Str(env.stringify(x.as_ref())))),
                ))
            }),
        })),
    );

    // Option: ∀(A: Type) -> Option(A)
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    env.add_global(
        "Option",
        Value::Builtin(Rc::new(Builtin {
            name: "Option",
            argtypes: vec![(a_str.clone(), env.type_type.clone())],
            rettyp: Rc::new(Type::TypeOf(Rc::new(Type::Option(ph)))),
            f: Box::new(|_, args| {
                let a = args[0].expect_type();
                Ok(Value::Type(Rc::new(Type::Option(a))))
            }),
        })),
    );

    // None: ∀(A: Type) -> Option(A)
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    env.add_global(
        "None",
        Value::Builtin(Rc::new(Builtin {
            name: "None",
            argtypes: vec![(a_str.clone(), env.type_type.clone())],
            rettyp: Rc::new(Type::Option(ph)),
            f: Box::new(|_, args| {
                let a = args[0].expect_type();
                Ok(Value::Option(a, None))
            }),
        })),
    );

    // Some: ∀(A: Type) -> ∀(x: A) -> Option(A)
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    env.add_global(
        "Some",
        Value::Builtin(Rc::new(Builtin {
            name: "Some",
            argtypes: vec![(a_str.clone(), env.type_type.clone())],
            rettyp: Rc::new(Type::Lambda(
                vec![(x_str.clone(), ph.clone())],
                Rc::new(Type::Option(ph)),
            )),
            f: Box::new(|_, args| {
                let a = args[0].expect_type();
                Ok(Value::Builtin(Rc::new(Builtin {
                    name: "None(A)",
                    argtypes: vec![(Rc::from("x"), a.clone())],
                    rettyp: Rc::new(Type::Option(a.clone())),
                    f: Box::new(move |_, args| {
                        Ok(Value::Option(a.clone(), Some(Box::new(args[0].clone()))))
                    }),
                })))
            }),
        })),
    );

    // Option/or: ∀(A: Type) -> ∀(x1: Option(A), x2: A) -> A
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    env.add_global(
        "Option/or",
        Value::Builtin(Rc::new(Builtin {
            name: "Option/or",
            argtypes: vec![(a_str.clone(), env.type_type.clone())],
            rettyp: Rc::new(Type::Lambda(
                vec![
                    (x_str.clone(), Rc::new(Type::Option(ph.clone()))),
                    (x_str.clone(), ph.clone()),
                ],
                ph,
            )),
            f: Box::new(|_, args| {
                let a = args[0].expect_type();
                Ok(Value::Builtin(Rc::new(Builtin {
                    name: "Option/or(A)",
                    argtypes: vec![
                        (Rc::from("x"), Rc::new(Type::Option(a.clone()))),
                        (Rc::from("x"), a.clone()),
                    ],
                    rettyp: a,
                    f: Box::new(|_, args| {
                        Ok(match (&args[0], &args[1]) {
                            (Value::Option(_, Some(v)), _) => *v.clone(),
                            (Value::Option(_, None), v) => v.clone(),
                            _ => panic!(),
                        })
                    }),
                })))
            }),
        })),
    );

    drop(x_str);
    drop(a_str);
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;
    use crate::ast::Parser;
    use crate::lex::Lexer;

    fn parse(input: &'static str) -> Result<Box<Node>, Error> {
        let mut spool = std::collections::HashSet::<Rc<str>>::new();
        let mut lexer = Lexer::new(input, 0, &mut spool);
        let mut parser = Parser::new(&mut lexer);
        parser.parse_all()
    }

    #[test]
    fn incto42() {
        let mut env = Env::new();
        let mut expr = parse("let inc = λ(x: Int) -> x + 1 in inc(41)").unwrap();
        expr.typecheck(&mut env, None).expect("typecheck failed");
        assert_matches!(env.eval(&expr), Ok(Value::Int(42)));
    }

    #[test]
    fn fib10() {
        let mut env = Env::new();
        let mut expr =
            parse("let fib: ∀(n: Int) -> Int = λ(n: Int) -> if n < 2 then n else fib(n - 1) + fib(n - 2) in fib(10)")
                .unwrap();
        expr.typecheck(&mut env, None).expect("typecheck failed");
        assert_matches!(env.eval(&expr), Ok(Value::Int(55)));
    }
}
