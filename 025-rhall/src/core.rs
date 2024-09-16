use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{ast::Node, eval::Env, lex};

#[derive(Clone, Copy, Debug, Default)]
pub struct SLoc {
    pub line: u32,
    pub col: u16,
    pub file_id: u16,
}

impl SLoc {
    #[allow(unused)]
    pub fn hash(&self) -> u64 {
        (self.line as u64) << 32 | (self.col as u64) << 8 | self.file_id as u64
    }
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
// that would be nice and beneficial. Also, tagged-union/enum types!
#[derive(Debug, Clone)]
pub enum Type {
    Placeholder(Rc<str>),
    Boolean,
    Integer,
    String,
    Any,
    TypeOfType,
    TypeOf(Rc<Type>),
    Lambda(Vec<(Rc<str>, Rc<Type>)>, Rc<Type>),
    Option(Rc<Type>),
    Record(Vec<(Rc<str>, Rc<Type>)>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Placeholder(name) => write!(f, "{}", name.as_ref()),
            Type::Boolean => write!(f, "Bool"),
            Type::Integer => write!(f, "Int"),
            Type::String => write!(f, "Str"),
            Type::Any => write!(f, "Any"),
            Type::TypeOfType => write!(f, "Type"),
            Type::TypeOf(t) => write!(f, "typeof({})", t.as_ref()),
            Type::Lambda(args, rettyp) => {
                write!(f, "∀(")?;
                for (i, (name, argtyp)) in args.iter().enumerate() {
                    write!(
                        f,
                        "{}{}: {}",
                        if i != 0 { ", " } else { "" },
                        name.as_ref(),
                        argtyp
                    )?;
                }
                write!(f, ") -> ({})", rettyp)
            }
            Type::Option(t) => write!(f, "Option({})", t.as_ref()),
            Type::Record(fields) if fields.is_empty() => write!(f, "{{:}}"),
            Type::Record(fields) => {
                write!(f, "{{ {}: {}", fields[0].0.as_ref(), fields[0].1.as_ref())?;
                for (name, typ) in fields[1..].iter() {
                    write!(f, ", {}: {}", name.as_ref(), typ.as_ref())?;
                }
                write!(f, " }}")
            }
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // TODO: Only comparing names could lead to false positive typechecks when two type
            // parameters have the same name in different scopes. Comparing the ids for equality
            // is not ok, one would need to check that every position a ID is used in the lhs,
            // a different but equal in all positions ID is used in the rhs.
            (Type::Placeholder(tp1), Type::Placeholder(tp2)) => tp1.as_ref() == tp2.as_ref(),
            (Type::Boolean, Type::Boolean) => true,
            (Type::Integer, Type::Integer) => true,
            (Type::String, Type::String) => true,
            (Type::TypeOfType, Type::TypeOfType) => true,
            (Type::TypeOf(t1), Type::TypeOf(t2)) => t1.as_ref() == t2.as_ref(),
            (Type::Lambda(args1, rettyp1), Type::Lambda(args2, rettyp2)) => {
                args1.len() == args2.len()
                    && args1
                        .iter()
                        .zip(args2.iter())
                        .all(|((_, t1), (_, t2))| t1.as_ref() == t2.as_ref())
                    && rettyp1.as_ref() == rettyp2.as_ref()
            }
            (Type::Option(t1), Type::Option(t2)) => t1 == t2,
            (Type::Record(fields1), Type::Record(fields2)) if fields1.len() == fields2.len() => {
                fields1
                    .iter()
                    .zip(fields2.iter())
                    .all(|((n1, t1), (n2, t2))| {
                        n1.as_ref() == n2.as_ref() && t1.as_ref() == t2.as_ref()
                    })
            }
            (_, _) => false,
        }
    }
}

impl Type {
    pub fn subst(self: &Rc<Self>, name: &str, subst: &Rc<Type>) -> Rc<Self> {
        // TODO: Lazy substitution would be nice, if a inner subst call returns self, don't add a
        // new allocation? Clone self instead?
        match self.as_ref() {
            Type::Placeholder(placeholder) if placeholder.as_ref() == name => subst.clone(),
            Type::Placeholder(_) => self.clone(),
            Type::Boolean => self.clone(),
            Type::Integer => self.clone(),
            Type::String => self.clone(),
            Type::Any => self.clone(),
            Type::TypeOfType => self.clone(),
            Type::TypeOf(t) => Rc::new(Type::TypeOf(t.subst(name, subst))),
            Type::Lambda(args, rettyp) => {
                let mut dosubst = true;
                let mut nargs = Vec::with_capacity(args.len());
                for (argname, argt) in args {
                    if argname.as_ref() == name {
                        dosubst = false;
                    }
                    nargs.push((
                        argname.clone(),
                        if dosubst {
                            argt.subst(name, subst)
                        } else {
                            argt.clone()
                        },
                    ));
                }

                Rc::new(Type::Lambda(
                    nargs,
                    if dosubst {
                        rettyp.subst(name, subst)
                    } else {
                        rettyp.clone()
                    },
                ))
            }
            Type::Option(t) => Rc::new(Type::Option(t.subst(name, subst))),
            Type::Record(fields) => Rc::new(Type::Record(
                fields
                    .iter()
                    .map(|(name, t)| (name.clone(), t.subst(name, subst)))
                    .collect(),
            )),
        }
    }

    #[allow(unused)]
    pub fn decompose_lambda(self: &Rc<Self>) -> (Vec<(Rc<str>, Rc<Type>)>, Rc<Self>) {
        match self.as_ref() {
            Type::Lambda(argtypes, rettyp) => (argtypes.clone(), rettyp.clone()),
            _ => panic!(),
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
    Builtin(Rc<Builtin>),
    Option(Rc<Type>, Option<Box<Value>>),
    Record(Vec<(Rc<str>, Value)>),
}

pub struct Builtin {
    pub name: &'static str,
    pub argtypes: Vec<(Rc<str>, Rc<Type>)>,
    pub rettyp: Rc<Type>,
    pub f: Box<dyn Fn(&mut Env, Vec<Value>) -> Result<Value, Error>>,
}

impl Debug for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<builtin:{} ({:?}) -> {}>",
            self.name, self.argtypes, self.rettyp
        )
    }
}

impl Value {
    pub fn get_type(&self, env: &Env) -> Rc<Type> {
        match self {
            Value::Pseudo(t) => t.clone(),
            Value::Bool(_) => env.bool_type.clone(),
            Value::Int(_) => env.int_type.clone(),
            Value::Str(_) => env.str_type.clone(),
            Value::Type(t) => {
                if **t == Type::TypeOfType {
                    env.type_type.clone()
                } else {
                    Rc::new(Type::TypeOf(t.clone()))
                }
            }
            Value::Lambda(args, body) => Rc::new(Type::Lambda(
                args.clone(),
                body.borrow().get_type().unwrap(),
            )),
            Value::Builtin(b) => Rc::new(Type::Lambda(b.argtypes.clone(), b.rettyp.clone())),
            Value::Option(t, _) => t.clone(),
            Value::Record(fields) => Rc::new(Type::Record(
                fields
                    .iter()
                    .map(|(name, val)| (name.clone(), val.get_type(env)))
                    .collect(),
            )),
        }
    }

    pub fn apply(&self, sloc: SLoc, env: &mut Env, args: Vec<Value>) -> Result<Value, Error> {
        match self {
            Value::Lambda(argnames, body) => {
                assert!(args.len() == argnames.len());
                for (value, (name, _)) in args.into_iter().zip(argnames) {
                    env.push(name, value);
                }

                let res = env.eval(&body.borrow())?;
                env.pop(argnames.len());
                Ok(res)
            }
            Value::Builtin(b) => (b.f)(env, args),
            _ => Err(Error::Uncallable(sloc, format!("{}", self))),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Pseudo(t) => write!(f, "<Something of Type {}>", t.as_ref()),
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
                write!(f, ") -> ({})", node.as_ref().borrow())
            }
            Value::Builtin(b) => write!(f, "{}", b.name),
            Value::Option(_, Some(val)) => write!(f, "Some({})", val),
            Value::Option(t, None) => write!(f, "None({})", t.as_ref()),
            Value::Record(fields) if fields.is_empty() => write!(f, "{{=}}"),
            Value::Record(fields) => {
                write!(f, "{{ {} = {}", fields[0].0.as_ref(), fields[0].1)?;
                for (name, val) in fields[1..].iter() {
                    write!(f, ", {} = {}", name.as_ref(), val)?;
                }
                write!(f, " }}")
            }
        }
    }
}
