use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    ast::{BinOp, Node},
    core::{Builtin, Error, SLoc, Type, TypeParam, Value},
};

pub struct Env {
    // TODO: Add the string_pool hashset here or to a runtime object.
    // TODO: Add pre-allocated type RCs here that can be used by the parser!
    globals: std::collections::HashMap<&'static str, Value>,
    locals: Vec<(Rc<str>, Value)>,
    pub string_pool: std::collections::HashSet<Rc<str>>,

    pub int_type: Rc<Type>,
    pub bool_type: Rc<Type>,
    pub str_type: Rc<Type>,
    pub type_type: Rc<Type>,
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
            type_type: Rc::new(Type::Type(None, None)),
        };
        env.globals.insert("Int", Value::Type(env.int_type.clone()));
        env.globals
            .insert("Bool", Value::Type(env.bool_type.clone()));
        env.globals.insert("Str", Value::Type(env.str_type.clone()));
        env.globals
            .insert("Type", Value::Type(env.type_type.clone()));
        env
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
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
        }
    }
}

pub fn add_builtins(env: &mut Env) {
    let t1_str: Rc<str> = Rc::from("A");
    let t2_str: Rc<str> = Rc::from("B");
    let x_str: Rc<str> = Rc::from("x");
    let f_str: Rc<str> = Rc::from("x");

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

    let t1tp = TypeParam {
        name: t1_str.clone(),
        id: line!() as u64,
    };
    let t1 = Rc::new(Type::Generic(t1tp.clone()));
    env.add_global(
        "Option",
        Value::Builtin(Rc::new(Builtin {
            name: "Option",
            argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t1tp), None)))],
            rettyp: Rc::new(Type::Type(None, Some(Rc::new(Type::Option(t1))))),
            f: Box::new(|_env, args| {
                let t = args[0].expect_type();
                Ok(Value::Type(Rc::new(Type::Option(t))))
            }),
        })),
    );

    let t1tp = TypeParam {
        name: t1_str.clone(),
        id: line!() as u64,
    };
    let t1 = Rc::new(Type::Generic(t1tp.clone()));
    env.add_global(
        "None",
        Value::Builtin(Rc::new(Builtin {
            name: "Some",
            argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t1tp), None)))],
            rettyp: Rc::new(Type::Option(t1)),
            f: Box::new(|_env, args| {
                let t = args[0].expect_type();
                Ok(Value::Option(t, None))
            }),
        })),
    );

    {
        let x_str = x_str.clone();
        let t1tp = TypeParam {
            name: t1_str.clone(),
            id: line!() as u64,
        };
        let t1 = Rc::new(Type::Generic(t1tp.clone()));
        env.add_global(
            "Some",
            Value::Builtin(Rc::new(Builtin {
                name: "Some",
                argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t1tp), None)))],
                rettyp: Rc::new(Type::Lambda(
                    vec![(x_str.clone(), t1.clone())],
                    Rc::new(Type::Option(t1.clone())),
                )),
                f: Box::new(move |_env, args| {
                    let t = args[0].expect_type();
                    Ok(Value::Builtin(Rc::new(Builtin {
                        name: "Some(A)",
                        argtypes: vec![(x_str.clone(), t1.clone())],
                        rettyp: Rc::new(Type::Option(t.clone())),
                        f: Box::new(move |_env, args| {
                            Ok(Value::Option(t.clone(), Some(Box::new(args[0].clone()))))
                        }),
                    })))
                }),
            })),
        );
    }

    {
        // Option/fold: ∀(A: Type, B: Type) -> ∀(o: Option(A)) -> ∀(f: ∀(a: A) -> B, b: B) -> B
        let t1tp = TypeParam {
            name: t1_str.clone(),
            id: line!() as u64,
        };
        let t2tp = TypeParam {
            name: t2_str.clone(),
            id: line!() as u64,
        };
        let t1 = Rc::new(Type::Generic(t1tp.clone()));
        let t2 = Rc::new(Type::Generic(t2tp.clone()));

        let mapftyp = Rc::new(Type::Lambda(vec![(x_str.clone(), t1.clone())], t2.clone()));
        let ftyp = Rc::new(Type::Lambda(
            vec![(x_str.clone(), Rc::new(Type::Option(t1)))],
            Rc::new(Type::Lambda(
                vec![(f_str.clone(), mapftyp), (x_str.clone(), t2.clone())],
                t2,
            )),
        ));

        env.add_global(
            "Option/fold",
            Value::Builtin(Rc::new(Builtin {
                name: "Option/fold",
                argtypes: vec![
                    (
                        t1_str.clone(),
                        Rc::new(Type::Type(Some(t1tp.clone()), None)),
                    ),
                    (
                        t2_str.clone(),
                        Rc::new(Type::Type(Some(t2tp.clone()), None)),
                    ),
                ],
                rettyp: ftyp.clone(),
                f: Box::new(move |_env, args| {
                    let t1 = args[0].expect_type();
                    let t2 = args[1].expect_type();
                    let (args, ftyp) = ftyp.subst(&t1tp, &t1).subst(&t2tp, &t2).decompose_lambda();
                    Ok(Value::Builtin(Rc::new(Builtin {
                        name: "Option/fold(A, B)",
                        argtypes: args,
                        rettyp: ftyp.clone(),
                        f: Box::new(move |_env, args| {
                            let opt = match &args[0] {
                                Value::Option(_, x) => x.clone(),
                                _ => panic!(),
                            };
                            let (args, ftyp) = ftyp.decompose_lambda();
                            Ok(Value::Builtin(Rc::new(Builtin {
                                name: "Option/fold(A, B)(Option(A))",
                                argtypes: args,
                                rettyp: ftyp,
                                f: Box::new(move |env, args| {
                                    let mapf = &args[0];
                                    let fallback = &args[1];
                                    match &opt {
                                        Some(x) => {
                                            mapf.apply(SLoc::default(), env, vec![(**x).clone()])
                                        }
                                        None => Ok(fallback.clone()),
                                    }
                                }),
                            })))
                        }),
                    })))
                }),
            })),
        );
    }

    drop(t1_str);
    drop(t2_str);
    drop(f_str);
    drop(x_str);
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
