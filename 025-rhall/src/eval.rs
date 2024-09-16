use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{BinOp, Node},
    core::{Builtin, Error, Lambda, Type, Value},
};

#[derive(Debug)]
pub struct Runtime {
    pub globals: std::collections::HashMap<&'static str, Value>,
    pub string_pool: std::collections::HashSet<Rc<str>>,
    pub locals: Vec<(Rc<str>, Value)>, // <- only to use during type-check!
    pub int_type: Rc<Type>,
    pub bool_type: Rc<Type>,
    pub str_type: Rc<Type>,
    pub type_type: Rc<Type>,
    pub any_type: Rc<Type>,
}

impl Runtime {
    pub fn new() -> Rc<RefCell<Runtime>> {
        let mut rt = Runtime {
            globals: std::collections::HashMap::new(),
            string_pool: std::collections::HashSet::new(),
            locals: Vec::new(),
            int_type: Rc::new(Type::Int),
            bool_type: Rc::new(Type::Bool),
            str_type: Rc::new(Type::Str),
            type_type: Rc::new(Type::TypeOfType),
            any_type: Rc::new(Type::Any),
        };
        add_builtins(&mut rt);
        Rc::new(RefCell::new(rt))
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
        let local = self.locals.iter().rev().find(|l| l.0.as_ref() == name);
        if let Some((_, val)) = local {
            return Some(val.clone());
        }
        self.globals.get(name).cloned()
    }

    #[allow(unused)]
    pub fn stringify(&mut self, string: &str) -> Rc<str> {
        if let Some(s) = self.string_pool.get(string) {
            return s.clone();
        }

        let s: Rc<str> = Rc::from(string);
        self.string_pool.insert(s.clone());
        s
    }

    pub fn add_builtin(&mut self, name: &'static str, builtin: Builtin) {
        assert!(!self.globals.contains_key(name));
        self.globals.insert(name, Value::Builtin(Rc::new(builtin)));
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
}

#[derive(Debug)]
pub struct Scope {
    depth: usize,
    up: Option<Rc<Scope>>,
    local: (Rc<str>, Value),
    runtime: Option<Rc<RefCell<Runtime>>>,
}

impl Scope {
    pub fn from(rt: Rc<RefCell<Runtime>>) -> Rc<Self> {
        Rc::new(Self {
            depth: 0,
            up: None,
            local: (Rc::from(""), Value::Int(-1)),
            runtime: Some(rt),
        })
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
        if self.local.0.as_ref() == name {
            return Some(self.local.1.clone());
        }
        if let Some(up) = &self.up {
            return up.lookup(name);
        }
        if let Some(rt) = &self.runtime {
            let rt = rt.borrow();
            return rt.globals.get(name).cloned();
        }
        None
    }

    pub fn push(self: &Rc<Self>, name: &Rc<str>, value: Value) -> Rc<Self> {
        Rc::new(Self {
            depth: self.depth + 1,
            up: Some(self.clone()),
            local: (name.clone(), value),
            runtime: None,
        })
    }

    #[allow(unused)]
    pub fn pop(self) -> Rc<Scope> {
        self.up.expect("expected a non-empty scope")
    }
}

pub fn eval(node: &Node, scope: &Rc<Scope>) -> Result<Value, Error> {
    match node {
        Node::Id { name, .. } => scope
            .lookup(name.as_ref())
            .ok_or(Error::UndefinedValue(name.clone())),
        Node::Integer { value, .. } => Ok(Value::Int(*value)),
        Node::Boolean { value, .. } => Ok(Value::Bool(*value)),
        Node::String { value, .. } => Ok(Value::Str(value.clone())),
        Node::TypeAnno { op0, .. } => eval(op0, scope),
        Node::Invert { op0, .. } => Ok(match eval(op0, scope)? {
            Value::Bool(value) => Value::Bool(!value),
            Value::Int(value) => Value::Int(!value),
            _ => unimplemented!(),
        }),
        Node::BinOp { op, lhs, rhs, .. } => Ok(match (op, eval(lhs, scope)?, eval(rhs, scope)?) {
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
            let callable = eval(callable, scope)?;
            let args: Result<Vec<_>, _> = args.iter().map(|arg| eval(arg, scope)).collect();
            callable.apply(*sloc, args?)
        }
        Node::IfThenElse { op0, op1, op2, .. } => match eval(op0, scope)? {
            Value::Bool(true) => eval(op1, scope),
            Value::Bool(false) => eval(op2, scope),
            _ => unimplemented!(),
        },
        Node::LetIn {
            name, value, body, ..
        } => {
            let value = eval(value, scope)?;
            if let Value::Lambda(l) = &value {
                let oldscope = l.scope.borrow().clone();
                l.scope
                    .replace(oldscope.push(name, Value::Lambda(l.clone())));
            }

            let scope = scope.push(name, value);
            let res = eval(body, &scope)?;
            Ok(res)
        }
        Node::Lambda { args, body, .. } => Ok(Value::Lambda(Rc::new(Lambda {
            args: args
                .iter()
                .map(|(name, typ, _)| (name.clone(), typ.clone().unwrap()))
                .collect(),
            body: body.clone(),
            scope: RefCell::new(scope.clone()),
        }))),
        Node::Forall {
            sloc,
            argtypes,
            rettyp,
            ..
        } => {
            let mut args = Vec::with_capacity(argtypes.len());
            let mut scope = scope.clone();
            for (name, argtyp, rawargtyp) in argtypes.iter() {
                if let Some(typ) = argtyp {
                    scope = scope.push(name, Value::Type(typ.clone()));
                    args.push((name.clone(), typ.clone()));
                } else {
                    let typval = eval(rawargtyp, &scope)?;
                    let typ = match typval {
                        Value::Type(ref t) => t.clone(),
                        _ => return Err(Error::ExpectedType(*sloc)),
                    };
                    scope = scope.push(name, typval);
                    args.push((name.clone(), typ));
                }
            }

            let rettyp = match eval(&rettyp.borrow(), &scope)? {
                Value::Type(t) => t,
                _ => return Err(Error::ExpectedType(*sloc)),
            };

            let res = Value::Type(Rc::new(Type::Lambda(args, rettyp)));
            Ok(res)
        }
        Node::Record { fields, .. } => {
            let fields: Result<Vec<_>, _> = fields
                .iter()
                .map(|(name, value)| eval(value, scope).map(|v| (name.clone(), v)))
                .collect();
            Ok(Value::Record(fields?))
        }
        Node::RecordType { typ, .. } => {
            // TODO: Now that type checks are always enabled and work well, something
            // like this should also work elsewhere:
            Ok(Value::Type(typ.clone().unwrap()))
        }
        Node::AccessField { op0, field, .. } => match eval(op0, scope)? {
            Value::Record(fields) => Ok(fields
                .iter()
                .find(|(name, _)| name.as_ref() == field.as_ref())
                .unwrap()
                .1
                .clone()),
            _ => panic!(),
        },
        Node::As { op0, as_typ, .. } => {
            match (eval(op0, scope)?, as_typ.as_ref().unwrap().as_ref()) {
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

pub fn add_builtins(rt: &mut Runtime) {
    rt.globals.insert("Int", Value::Type(rt.int_type.clone()));
    rt.globals.insert("Bool", Value::Type(rt.bool_type.clone()));
    rt.globals.insert("Str", Value::Type(rt.str_type.clone()));
    rt.globals.insert("Type", Value::Type(rt.type_type.clone()));
    rt.globals.insert("Any", Value::Type(rt.any_type.clone()));

    let x_str: Rc<str> = Rc::from("x");
    let a_str: Rc<str> = Rc::from("A");

    rt.add_builtin(
        "Process/exit",
        Builtin {
            name: "Process/exit",
            argtypes: vec![(Rc::from("code"), rt.int_type.clone())],
            rettyp: rt.int_type.clone(),
            f: Box::new(|args| {
                let code = match args[0] {
                    Value::Int(x) => x,
                    _ => panic!(),
                };
                std::process::exit(code as i32)
            }),
        },
    );

    rt.add_builtin(
        "Process/getenv",
        Builtin {
            name: "Process/getenv",
            argtypes: vec![(x_str.clone(), rt.str_type.clone())],
            rettyp: Rc::new(Type::Option(rt.str_type.clone())),
            f: Box::new(|args| {
                let x = match &args[0] {
                    Value::Str(x) => x,
                    _ => panic!(),
                };
                Ok(Value::Option(
                    Rc::new(Type::Str),
                    std::env::var(x.as_ref())
                        .ok()
                        .map(|x| Box::new(Value::Str(Rc::from(x.as_ref())))),
                ))
            }),
        },
    );

    // Option: ∀(A: Type) -> Option(A)
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    rt.add_builtin(
        "Option",
        Builtin {
            name: "Option",
            argtypes: vec![(a_str.clone(), rt.type_type.clone())],
            rettyp: Rc::new(Type::TypeOf(Rc::new(Type::Option(ph)))),
            f: Box::new(|args| {
                let a = args[0].expect_type();
                Ok(Value::Type(Rc::new(Type::Option(a))))
            }),
        },
    );

    // None: ∀(A: Type) -> Option(A)
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    rt.add_builtin(
        "None",
        Builtin {
            name: "None",
            argtypes: vec![(a_str.clone(), rt.type_type.clone())],
            rettyp: Rc::new(Type::Option(ph)),
            f: Box::new(|args| {
                let a = args[0].expect_type();
                Ok(Value::Option(a, None))
            }),
        },
    );

    // Some: ∀(A: Type) -> ∀(x: A) -> Option(A)
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    rt.add_builtin(
        "Some",
        Builtin {
            name: "Some",
            argtypes: vec![(a_str.clone(), rt.type_type.clone())],
            rettyp: Rc::new(Type::Lambda(
                vec![(x_str.clone(), ph.clone())],
                Rc::new(Type::Option(ph)),
            )),
            f: Box::new(|args| {
                let a = args[0].expect_type();
                Ok(Value::Builtin(Rc::new(Builtin {
                    name: "None(A)",
                    argtypes: vec![(Rc::from("x"), a.clone())],
                    rettyp: Rc::new(Type::Option(a.clone())),
                    f: Box::new(move |args| {
                        Ok(Value::Option(a.clone(), Some(Box::new(args[0].clone()))))
                    }),
                })))
            }),
        },
    );

    // Option/or: ∀(A: Type) -> ∀(x1: Option(A), x2: A) -> A
    let ph = Rc::new(Type::Placeholder(a_str.clone()));
    rt.add_builtin(
        "Option/or",
        Builtin {
            name: "Option/or",
            argtypes: vec![(a_str.clone(), rt.type_type.clone())],
            rettyp: Rc::new(Type::Lambda(
                vec![
                    (x_str.clone(), Rc::new(Type::Option(ph.clone()))),
                    (x_str.clone(), ph.clone()),
                ],
                ph,
            )),
            f: Box::new(|args| {
                let a = args[0].expect_type();
                Ok(Value::Builtin(Rc::new(Builtin {
                    name: "Option/or(A)",
                    argtypes: vec![
                        (Rc::from("x"), Rc::new(Type::Option(a.clone()))),
                        (Rc::from("x"), a.clone()),
                    ],
                    rettyp: a,
                    f: Box::new(|args| {
                        Ok(match (&args[0], &args[1]) {
                            (Value::Option(_, Some(v)), _) => *v.clone(),
                            (Value::Option(_, None), v) => v.clone(),
                            _ => panic!(),
                        })
                    }),
                })))
            }),
        },
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
        let rt = Runtime::new();
        let mut expr = parse("let inc = λ(x: Int) -> x + 1 in inc(41)").unwrap();
        expr.typecheck(&mut *rt.borrow_mut(), None)
            .expect("typecheck failed");
        assert_matches!(eval(&expr, &Scope::from(rt)), Ok(Value::Int(42)));
    }

    #[test]
    fn fib10() {
        let rt = Runtime::new();
        let mut expr =
            parse("let fib: ∀(n: Int) -> Int = λ(n: Int) -> if n < 2 then n else fib(n - 1) + fib(n - 2) in fib(10)")
                .unwrap();
        expr.typecheck(&mut *rt.borrow_mut(), None)
            .expect("typecheck failed");
        assert_matches!(eval(&expr, &Scope::from(rt)), Ok(Value::Int(55)));
    }
}
