use std::fmt;
use std::{cell::RefCell, rc::Rc};

use crate::core::{Error, SLoc, Type, TypeParam, Value};
use crate::eval::Env;
use crate::lex::{Lexer, Tok};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    EQ,
    NE,
    LT,
    LE,
    GT,
    GE,
}

#[derive(Debug, Clone)]
pub enum Node {
    Id {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        name: Rc<str>,
    },
    Integer {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        value: i64,
    },
    Boolean {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        value: bool,
    },
    String {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        value: Rc<str>,
    },
    TypeAnno {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: Box<Node>,
        rawtyp: Box<Node>,
    },
    Invert {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: Box<Node>,
    },
    BinOp {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op: BinOp,
        lhs: Box<Node>,
        rhs: Box<Node>,
        iscmp: bool,
    },
    Call {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        callable: Box<Node>,
        args: Vec<Node>,
    },
    IfThenElse {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: Box<Node>,
        op1: Box<Node>,
        op2: Box<Node>,
    },
    LetIn {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        name: Rc<str>,
        value: Box<Node>,
        typeannot: Option<Box<Node>>,
        body: Box<Node>,
    },
    Lambda {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        args: Vec<(Rc<str>, Option<Rc<Type>>, Box<Node>)>,
        body: Rc<RefCell<Node>>,
    },
    Forall {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        argtypes: Vec<(Rc<str>, Option<Rc<Type>>, Box<Node>)>,
        rettyp: Rc<RefCell<Node>>,
    },
    Record {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        fields: Vec<(Rc<str>, Node)>,
    },
    RecordType {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        fields: Vec<(Rc<str>, Node)>,
    },
    AccessField {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: Box<Node>,
        field: Rc<str>,
    },
}

impl Node {
    #[allow(unused)]
    pub fn get_type(&self) -> Option<Rc<Type>> {
        let typ = match self {
            Node::Id { typ, .. } => typ,
            Node::Integer { typ, .. } => typ,
            Node::Boolean { typ, .. } => typ,
            Node::String { typ, .. } => typ,
            Node::TypeAnno { typ, .. } => typ,
            Node::Invert { typ, .. } => typ,
            Node::BinOp { typ, .. } => typ,
            Node::Call { typ, .. } => typ,
            Node::IfThenElse { typ, .. } => typ,
            Node::LetIn { typ, .. } => typ,
            Node::Lambda { typ, .. } => typ,
            Node::Forall { typ, .. } => typ,
            Node::Record { typ, .. } => typ,
            Node::RecordType { typ, .. } => typ,
            Node::AccessField { typ, .. } => typ,
        };
        typ.clone()
    }

    #[allow(unused)]
    pub fn set_type(&mut self, t: Rc<Type>) {
        match self {
            Node::Id { typ, .. } => *typ = Some(t),
            Node::Integer { typ, .. } => *typ = Some(t),
            Node::Boolean { typ, .. } => *typ = Some(t),
            Node::String { typ, .. } => *typ = Some(t),
            Node::TypeAnno { typ, .. } => *typ = Some(t),
            Node::Invert { typ, .. } => *typ = Some(t),
            Node::BinOp { typ, .. } => *typ = Some(t),
            Node::Call { typ, .. } => *typ = Some(t),
            Node::IfThenElse { typ, .. } => *typ = Some(t),
            Node::LetIn { typ, .. } => *typ = Some(t),
            Node::Lambda { typ, .. } => *typ = Some(t),
            Node::Forall { typ, .. } => *typ = Some(t),
            Node::Record { typ, .. } => *typ = Some(t),
            Node::RecordType { typ, .. } => *typ = Some(t),
            Node::AccessField { typ, .. } => *typ = Some(t),
        };
    }

    pub fn typecheck(&mut self, env: &mut Env, _hint: Option<Rc<Type>>) -> Result<Rc<Type>, Error> {
        match self {
            Node::Id { typ: Some(t), .. } => Ok(t.clone()),
            Node::Id { sloc, typ, name } => match env.lookup(name) {
                Some(v) => {
                    let t = v.get_type(env);
                    *typ = Some(t.clone());
                    Ok(t)
                }
                None => Err(Error::TypeError(
                    *sloc,
                    format!(
                        "unknown: {} (if this is a recursive function, add a type annotation)",
                        name.as_ref()
                    ),
                )),
            },
            Node::Integer { typ: Some(t), .. } => Ok(t.clone()),
            Node::Integer { typ, .. } => {
                let t = env.int_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::Boolean { typ: Some(t), .. } => Ok(t.clone()),
            Node::Boolean { typ, .. } => {
                let t = env.bool_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::String { typ: Some(t), .. } => Ok(t.clone()),
            Node::String { typ, .. } => {
                let t = env.str_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::TypeAnno { typ: Some(t), .. } => Ok(t.clone()),
            Node::TypeAnno {
                sloc,
                typ,
                op0,
                rawtyp,
            } => {
                let hint = rawtyp.typecheck_type(env, None, *sloc, "type annotation")?;
                let optyp = op0.typecheck(env, Some(hint.clone()))?;
                if optyp.as_ref() != hint.as_ref() {
                    return Err(Error::TypeError(
                        *sloc,
                        format!("type conflict: {} vs {}", optyp.as_ref(), hint.as_ref()),
                    ));
                }

                *typ = Some(optyp.clone());
                Ok(optyp)
            }
            Node::Invert { typ: Some(t), .. } => Ok(t.clone()),
            Node::Invert { sloc, typ, op0 } => {
                let t = op0.typecheck(env, None)?;
                if *t.as_ref() != Type::Boolean && *t.as_ref() != Type::Integer {
                    return Err(Error::TypeError(
                        *sloc,
                        format!("invalid type for '~' operator: {}", t),
                    ));
                }
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::BinOp { typ: Some(t), .. } => Ok(t.clone()),
            Node::BinOp {
                sloc,
                typ,
                op,
                lhs,
                rhs,
                iscmp,
            } => {
                let lhsty = lhs.typecheck(env, None)?;
                let rhsty = rhs.typecheck(env, None)?;
                if *lhsty != *rhsty {
                    return Err(Error::TypeError(
                        *sloc,
                        format!(
                            "binary operator with different operand types: {} and {}",
                            lhsty.as_ref(),
                            rhsty.as_ref()
                        ),
                    ));
                }

                let t = match (op, iscmp, lhsty.as_ref()) {
                    (_, true, Type::Integer) => env.bool_type.clone(),
                    (BinOp::And | BinOp::Or, _, Type::Boolean) => lhsty,
                    (BinOp::And | BinOp::Or, _, _) => {
                        return Err(Error::TypeError(
                            *sloc,
                            "'&' and '|' only work for booleans".to_string(),
                        ))
                    }
                    (_, false, Type::Integer) => lhsty,
                    _ => unimplemented!(),
                };
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::Call { typ: Some(t), .. } => Ok(t.clone()),
            Node::Call {
                sloc,
                typ,
                callable,
                args,
            } => match callable.typecheck(env, None)?.as_ref() {
                Type::Lambda(argtypes, rettyp) if argtypes.len() == args.len() => {
                    let mut rettyp = rettyp.clone();
                    for (arg, (_, typ)) in args.iter_mut().zip(argtypes.iter()) {
                        let t = arg.typecheck(env, None)?;
                        /* These error messages are horrible and the checks probably far from perfect! */
                        if let Type::Type(Some(tp), _) = typ.as_ref() {
                            let Type::Type(_, Some(argt)) = t.as_ref() else {
                                return Err(Error::TypeError(*sloc, format!("lambda call: expected a type parameter, not {}", t.as_ref())));
                            };

                            rettyp = rettyp.subst(tp, argt);
                            continue;
                        }

                        if t.as_ref() != typ.as_ref() {
                            return Err(Error::TypeError(
                                *sloc,
                                format!(
                                    "argument has wrong type: {} has type {}, expected {}",
                                    arg, t, typ
                                ),
                            ));
                        }
                    }
                    *typ = Some(rettyp.clone());
                    Ok(rettyp)
                }
                t => Err(Error::TypeError(
                    *sloc,
                    format!("cannot call: {} (type: {})", callable.as_ref(), t),
                )),
            },
            Node::IfThenElse { typ: Some(t), .. } => Ok(t.clone()),
            Node::IfThenElse {
                sloc,
                typ,
                op0,
                op1,
                op2,
            } => {
                let op0ty = op0.typecheck(env, None)?;
                if op0ty.as_ref() != &Type::Boolean {
                    return Err(Error::TypeError(
                        *sloc,
                        format!(
                            "condition of if-then-else needs to be a boolean, not: {}",
                            op0ty.as_ref()
                        ),
                    ));
                }

                let op1ty = op1.typecheck(env, None)?;
                let op2ty = op2.typecheck(env, None)?;
                if op1ty.as_ref() != op2ty.as_ref() {
                    return Err(Error::TypeError(
                        *sloc,
                        format!(
                            "branches of if-then-else needs to be of same type, not: {} and {}",
                            op1ty.as_ref(),
                            op2ty.as_ref()
                        ),
                    ));
                }

                *typ = Some(op1ty);
                Ok(op2ty)
            }
            Node::LetIn { typ: Some(t), .. } => Ok(t.clone()),
            Node::LetIn {
                typ,
                name,
                value,
                typeannot: None,
                body,
                ..
            } => {
                let valtyp = value.typecheck(env, None)?;
                env.push(name, Value::Pseudo(valtyp));
                let bodytyp = body.typecheck(env, None)?;
                env.pop(1);
                *typ = Some(bodytyp.clone());
                Ok(bodytyp)
            }
            Node::LetIn {
                sloc,
                typ,
                name,
                value,
                typeannot: Some(rawtypeannot),
                body,
            } => {
                let valtyphint = rawtypeannot.typecheck_type(env, None, *sloc, "let-in")?;
                env.push(name, Value::Pseudo(valtyphint.clone()));
                let valtyp = value.typecheck(env, Some(valtyphint.clone()))?;
                if valtyphint.as_ref() != valtyp.as_ref() {
                    println!("{:?}", env.locals);
                    return Err(Error::TypeError(
                        *sloc,
                        format!(
                            "computed type: {}, expected: {} ({:?}, {:?})",
                            valtyp.as_ref(), valtyphint.as_ref(),
                            valtyp.as_ref(), valtyphint.as_ref()
                        ),
                    ));
                }
                let bodytyp = body.typecheck(env, None)?;
                env.pop(1);
                *typ = Some(bodytyp.clone());
                Ok(bodytyp)
            }
            Node::Lambda { typ: Some(t), .. } => Ok(t.clone()),
            Node::Lambda {
                sloc,
                typ,
                args,
                body,
                ..
            } => {
                for (name, argtyp, rawargtyp) in args.iter_mut() {
                    let t = rawargtyp.typecheck_type(env, None, *sloc, "lambda (argument)")?;
                    match t.as_ref() {
                        Type::Type(None, None) => {
                            let tp = TypeParam {
                                name: name.clone(),
                                id: sloc.hash(),
                            };
                            let gent = Rc::new(Type::Generic(tp.clone()));
                            *argtyp = Some(Rc::new(Type::Type(Some(tp), None)));
                            env.push(name, Value::Pseudo(gent));
                        }
                        _ => {
                            *argtyp = Some(t.clone());
                            env.push(name, Value::Pseudo(t));
                        }
                    };
                }

                let rettyp = body.borrow_mut().typecheck(env, None)?;
                env.pop(args.len());
                let t = Rc::new(Type::Lambda(
                    args.iter()
                        .map(|(name, t, _)| (name.clone(), t.clone().unwrap()))
                        .collect(),
                    rettyp,
                ));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::Forall { typ: Some(t), .. } => Ok(t.clone()),
            Node::Forall {
                sloc,
                typ,
                argtypes,
                rettyp,
            } => {
                for (name, argtyp, rawargtyp) in argtypes.iter_mut() {
                    let t = rawargtyp.typecheck_type(env, None, *sloc, "forall (argument)")?;
                    if t.as_ref() == &Type::Type(None, None) {
                        let tp = TypeParam {
                            name: name.clone(),
                            id: sloc.hash(),
                        };
                        let gent = Rc::new(Type::Generic(tp.clone()));
                        *argtyp = Some(Rc::new(Type::Type(Some(tp), None)));
                        env.push(name, Value::Pseudo(gent));
                        continue;
                    }
                    *argtyp = Some(t.clone());
                    env.push(name, Value::Pseudo(t));
                }
                let rettyp =
                    rettyp
                        .borrow_mut()
                        .typecheck_type(env, None, *sloc, "forall (return type)")?;
                env.pop(argtypes.len());
                let t = Rc::new(Type::Lambda(
                    argtypes
                        .iter()
                        .map(|(name, t, _)| (name.clone(), t.clone().unwrap()))
                        .collect(),
                    rettyp,
                ));
                let t = Rc::new(Type::Type(None, Some(t)));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::Record { typ: Some(t), .. } => Ok(t.clone()),
            Node::Record { typ, fields, .. } => {
                let fields: Result<Vec<_>, _> = fields
                    .iter_mut()
                    .map(|(name, val)| val.typecheck(env, None).map(|t| (name.clone(), t)))
                    .collect();
                let t = Rc::new(Type::Record(fields?));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::RecordType { typ: Some(t), .. } => Ok(t.clone()),
            Node::RecordType {
                sloc, typ, fields, ..
            } => {
                let fields: Result<Vec<_>, _> = fields
                    .iter_mut()
                    .map(|(name, rawtyp)| {
                        rawtyp
                            .typecheck_type(env, None, *sloc, "record type literal")
                            .map(|t| (name.clone(), t))
                    })
                    .collect();
                let t = Rc::new(Type::Type(None, Some(Rc::new(Type::Record(fields?)))));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::AccessField { typ: Some(t), .. } => Ok(t.clone()),
            Node::AccessField {
                sloc,
                typ,
                op0,
                field,
            } => match op0.typecheck(env, None)?.as_ref() {
                Type::Record(fields) => match fields
                    .iter()
                    .find(|(name, _)| name.as_ref() == field.as_ref())
                {
                    Some((_, t)) => {
                        *typ = Some(t.clone());
                        Ok(t.clone())
                    }
                    None => Err(Error::TypeError(
                        *sloc,
                        format!("record does not have field {:?}: {}", field.as_ref(), op0),
                    )),
                },
                _ => Err(Error::TypeError(*sloc, format!("not a record: {}", op0))),
            },
        }
    }

    pub fn typecheck_type(
        &mut self,
        env: &mut Env,
        hint: Option<Rc<Type>>,
        sloc: SLoc,
        explain: &'static str,
    ) -> Result<Rc<Type>, Error> {
        let t = self.typecheck(env, hint)?;
        match t.as_ref() {
            Type::Type(None, Some(t)) => Ok(t.clone()),
            Type::Type(_, None) => Ok(t.clone()),
            Type::Generic(_) => Ok(t),
            _ => Err(Error::TypeError(
                sloc,
                format!(
                    "{}: expected a type, not {} of type {}",
                    explain,
                    self,
                    t.as_ref()
                ),
            )),
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Id { name, .. } => f.write_str(name.as_ref()),
            Node::Integer { value, .. } => write!(f, "{}", value),
            Node::Boolean { value: true, .. } => f.write_str("⊤"),
            Node::Boolean { value: false, .. } => f.write_str("⊥"),
            Node::String { value, .. } => write!(f, "{:?}", value.as_ref()),
            Node::TypeAnno {
                typ: Some(t), op0, ..
            } => write!(f, "({} : {})", op0.as_ref(), t.as_ref()),
            Node::TypeAnno {
                typ: None,
                op0,
                rawtyp,
                ..
            } => write!(f, "({} : {})", op0.as_ref(), rawtyp.as_ref()),
            Node::Invert { op0, .. } => write!(f, "(~{})", op0.as_ref()),
            Node::BinOp { op, lhs, rhs, .. } => write!(
                f,
                "({} {} {})",
                lhs.as_ref(),
                match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "*",
                    BinOp::And => "&",
                    BinOp::Or => "|",
                    BinOp::EQ => "==",
                    BinOp::NE => "!=",
                    BinOp::LT => "<",
                    BinOp::LE => "<=",
                    BinOp::GT => ">",
                    BinOp::GE => "<=",
                },
                rhs.as_ref()
            ),
            Node::Call { callable, args, .. } => {
                write!(f, "{}(", callable.as_ref())?;
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Node::IfThenElse { op0, op1, op2, .. } => write!(
                f,
                "if ({}) then ({}) else ({})",
                op0.as_ref(),
                op1.as_ref(),
                op2.as_ref()
            ),
            Node::LetIn {
                name, value, body, ..
            } => write!(
                f,
                "let {} = {} in {}",
                name.as_ref(),
                value.as_ref(),
                body.as_ref()
            ),
            Node::Lambda { args, body, .. } => {
                write!(f, "λ(")?; // TODO: Simplify print code like for records!
                for (i, (name, typ, rawtyp)) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    if let Some(t) = typ {
                        write!(f, "{}: {}", name.as_ref(), t.as_ref())?;
                    } else {
                        write!(f, "{}: {}", name.as_ref(), rawtyp.as_ref())?;
                    }
                }
                write!(f, ") -> {}", body.borrow())
            }
            Node::Forall {
                argtypes, rettyp, ..
            } => {
                write!(f, "∀(")?; // TODO: Simplify print code like for records!
                for (i, (name, typ, rawtyp)) in argtypes.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    if let Some(t) = typ {
                        write!(f, "{}: {}", name.as_ref(), t.as_ref())?;
                    } else {
                        write!(f, "{}: {}", name.as_ref(), rawtyp.as_ref())?;
                    }
                }
                write!(f, ") -> {}", rettyp.borrow())
            }
            Node::Record { fields, .. } if fields.is_empty() => write!(f, "{{=}}"),
            Node::Record { fields, .. } => {
                write!(f, "{{ {} = {}", fields[0].0.as_ref(), fields[0].1)?;
                for (name, value) in &fields[1..] {
                    write!(f, ", {} = {}", name.as_ref(), value)?;
                }
                write!(f, " }}")
            }
            Node::RecordType { fields, .. } if fields.is_empty() => write!(f, "{{:}}"),
            Node::RecordType { fields, .. } => {
                write!(f, "{{ {}: {}", fields[0].0.as_ref(), fields[0].1)?;
                for (name, value) in &fields[1..] {
                    write!(f, ", {}: {}", name.as_ref(), value)?;
                }
                write!(f, " }}")
            }
            Node::AccessField { op0, field, .. } => {
                write!(f, "({}).{}", op0.as_ref(), field.as_ref())
            }
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

pub struct Parser<'a> {
    lexer: &'a mut Lexer<'a>,
    consumed_sloc: SLoc,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: &'a mut Lexer<'a>) -> Parser {
        Parser {
            lexer,
            consumed_sloc: SLoc {
                line: 0,
                col: 0,
                file_id: 0,
            },
        }
    }

    fn expect_token(&mut self, expected: Tok) -> Result<SLoc, Error> {
        let (sloc, tok) = self.lexer.next().ok_or(Error::UnexpectedEOF)??;
        if tok == expected {
            Ok(sloc)
        } else {
            Err(Error::ExpectedToken {
                sloc,
                expected,
                found: tok,
            })
        }
    }

    fn expect_id(&mut self) -> Result<(SLoc, Rc<str>), Error> {
        match self.lexer.next().ok_or(Error::UnexpectedEOF)?? {
            (sloc, Tok::Id(name)) => Ok((sloc, name)),
            (sloc, t) => Err(Error::ExpectedToken {
                sloc,
                expected: Tok::Id(Rc::from("<whatever>")),
                found: t,
            }),
        }
    }

    fn consume_if(&mut self, tok: Tok) -> bool {
        if let Some(Ok((sloc, t))) = self.lexer.peek() {
            self.consumed_sloc = sloc;
            if t == tok {
                self.lexer.next();
                return true;
            }
        }
        false
    }

    pub fn parse_all(&mut self) -> Result<Box<Node>, Error> {
        let node = self.parse()?;
        match self.lexer.next() {
            Some(Ok((sloc, tok))) => Err(Error::Parser(
                sloc,
                format!("EOF expected, found: {:?}", tok),
            )),
            Some(Err(e)) => Err(e),
            None => Ok(node),
        }
    }

    fn parse(&mut self) -> Result<Box<Node>, Error> {
        let (sloc, tok) = self.lexer.peek().ok_or(Error::UnexpectedEOF)??;
        if tok == Tok::If {
            self.lexer.next();
            let op0 = self.parse_expr1()?;
            self.expect_token(Tok::Then)?;
            let op1 = self.parse_expr1()?;
            self.expect_token(Tok::Else)?;
            let op2 = self.parse_expr1()?;
            return Ok(Box::new(Node::IfThenElse {
                sloc,
                typ: None,
                op0,
                op1,
                op2,
            }));
        }

        if tok == Tok::Let {
            self.lexer.next();
            let (_, name) = self.expect_id()?;
            let typhint = match self.consume_if(Tok::Colon) {
                true => Some(self.parse_final()?),
                false => None,
            };
            self.expect_token(Tok::Assign)?;
            let value = self.parse()?;

            let body = if let Some(Ok((_, Tok::Let))) = self.lexer.peek() {
                self.parse()?
            } else {
                self.expect_token(Tok::In)?;
                self.parse()?
            };
            return Ok(Box::new(Node::LetIn {
                sloc,
                name,
                typ: None,
                value,
                typeannot: typhint,
                body,
            }));
        }

        self.parse_expr1()
    }

    /* Returns (binop, precedence, is_left_associative, is_compare). */
    fn binop_precedence(tok: &Tok) -> Option<(BinOp, usize, bool, bool)> {
        match tok {
            Tok::Star => Some((BinOp::Mul, 100, true, false)),
            Tok::Slash => Some((BinOp::Div, 100, true, false)),
            Tok::Plus => Some((BinOp::Add, 90, true, false)),
            Tok::Minus => Some((BinOp::Sub, 90, true, false)),
            Tok::Equal => Some((BinOp::EQ, 80, true, true)),
            Tok::NotEqual => Some((BinOp::NE, 80, true, true)),
            Tok::Lower => Some((BinOp::LT, 80, true, true)),
            Tok::LowerOrEqual => Some((BinOp::LE, 80, true, true)),
            Tok::Greater => Some((BinOp::GT, 80, true, true)),
            Tok::GreaterOrEqual => Some((BinOp::GE, 80, true, true)),
            Tok::Ampersand => Some((BinOp::And, 70, true, false)),
            Tok::Pipe => Some((BinOp::Or, 70, true, false)),
            _ => None,
        }
    }

    fn parse_binop(&mut self, min_prec: usize) -> Result<Box<Node>, Error> {
        let mut lhs = self.parse_expr0()?;
        while let Some(Ok((sloc, tok))) = self.lexer.peek() {
            let (binop, prec, leftassoc, iscmp) = match Self::binop_precedence(&tok) {
                Some(t) if t.1 >= min_prec => t,
                _ => break,
            };

            self.lexer.next().unwrap().unwrap();

            let next_min_prev = if leftassoc { prec + 1 } else { prec };
            let rhs = self.parse_binop(next_min_prev)?;
            lhs = Box::new(Node::BinOp {
                sloc,
                typ: None,
                op: binop,
                lhs,
                rhs,
                iscmp,
            });
        }
        Ok(lhs)
    }

    fn parse_expr1(&mut self) -> Result<Box<Node>, Error> {
        let expr = self.parse_binop(0)?;
        if self.consume_if(Tok::Colon) {
            let typhint = self.parse_expr0()?;
            return Ok(Box::new(Node::TypeAnno {
                sloc: self.consumed_sloc,
                typ: None,
                op0: expr,
                rawtyp: typhint,
            }));
        }
        Ok(expr)
    }

    fn parse_expr0(&mut self) -> Result<Box<Node>, Error> {
        let mut expr = self.parse_final()?;
        loop {
            if self.consume_if(Tok::LParen) {
                let mut args = vec![];
                loop {
                    args.push(*self.parse()?);
                    if self.consume_if(Tok::Comma) {
                        continue;
                    }

                    if self.consume_if(Tok::RParen) {
                        break;
                    }

                    return Err(Error::Parser(
                        self.consumed_sloc,
                        "Expected ',' or ')'".to_string(),
                    ));
                }

                expr = Box::new(Node::Call {
                    sloc: self.consumed_sloc,
                    typ: None,
                    callable: expr,
                    args,
                });
                continue;
            }

            if self.consume_if(Tok::Dot) {
                let (sloc, name) = self.expect_id()?;
                expr = Box::new(Node::AccessField {
                    sloc,
                    typ: None,
                    op0: expr,
                    field: name,
                });
            }

            break;
        }
        Ok(expr)
    }

    fn parse_final(&mut self) -> Result<Box<Node>, Error> {
        let (sloc, tok) = self.lexer.next().ok_or(Error::UnexpectedEOF)??;
        Ok(Box::new(match tok {
            Tok::Id(name) => Node::Id {
                sloc,
                typ: None,
                name,
            },
            Tok::Boolean(value) => Node::Boolean {
                sloc,
                typ: None,
                value,
            },
            Tok::Integer(value) => Node::Integer {
                sloc,
                typ: None,
                value,
            },
            Tok::String(value) => Node::String {
                sloc,
                typ: None,
                value,
            },
            Tok::Tilde => Node::Invert {
                sloc,
                typ: None,
                op0: self.parse_final()?,
            },
            Tok::LParen => {
                let expr = self.parse()?;
                self.expect_token(Tok::RParen)?;
                return Ok(expr);
            }
            Tok::LBrace => {
                if self.consume_if(Tok::Colon) {
                    self.expect_token(Tok::RBrace)?;
                    return Ok(Box::new(Node::RecordType {
                        sloc,
                        typ: None,
                        fields: vec![],
                    }));
                }

                if self.consume_if(Tok::Assign) {
                    self.expect_token(Tok::RBrace)?;
                    return Ok(Box::new(Node::Record {
                        sloc,
                        typ: None,
                        fields: vec![],
                    }));
                }

                let (_, id) = self.expect_id()?;
                if self.consume_if(Tok::Colon) {
                    return self.parse_record_type(sloc, id);
                }

                self.expect_token(Tok::Assign)?;
                return self.parse_record(sloc, id);
            }
            Tok::Lambda => {
                self.expect_token(Tok::LParen)?;
                let mut args = vec![];
                loop {
                    let (_, name) = self.expect_id()?;
                    self.expect_token(Tok::Colon)?;
                    let typannot = self.parse()?;
                    args.push((name, None, typannot));

                    if !self.consume_if(Tok::Comma) {
                        break;
                    }
                }
                self.expect_token(Tok::RParen)?;
                self.expect_token(Tok::Arrow)?;
                let body = self.parse()?;
                Node::Lambda {
                    sloc,
                    typ: None,
                    args,
                    body: Rc::new(RefCell::new(*body)),
                }
            }
            Tok::Forall => {
                self.expect_token(Tok::LParen)?;
                let mut argtypes = vec![];
                loop {
                    let (_, name) = self.expect_id()?;
                    self.expect_token(Tok::Colon)?;

                    let argtyp = self.parse()?;
                    argtypes.push((name, None, argtyp));
                    if !self.consume_if(Tok::Comma) {
                        break;
                    }
                }
                self.expect_token(Tok::RParen)?;
                self.expect_token(Tok::Arrow)?;
                let rettyp = self.parse()?;
                Node::Forall {
                    sloc,
                    typ: None,
                    argtypes,
                    rettyp: Rc::new(RefCell::new(*rettyp)),
                }
            }
            _ => unimplemented!(),
        }))
    }

    fn parse_record(&mut self, sloc: SLoc, id0: Rc<str>) -> Result<Box<Node>, Error> {
        let val0 = self.parse()?;
        let mut fields = vec![(id0, *val0)];
        loop {
            if self.consume_if(Tok::RBrace) {
                break;
            }

            self.expect_token(Tok::Comma)?;
            let (_, id) = self.expect_id()?;
            self.expect_token(Tok::Assign)?;
            let val = self.parse()?;
            fields.push((id, *val));
        }
        Ok(Box::new(Node::Record {
            sloc,
            typ: None,
            fields,
        }))
    }

    fn parse_record_type(&mut self, sloc: SLoc, id0: Rc<str>) -> Result<Box<Node>, Error> {
        let typ0 = self.parse()?;
        let mut fields = vec![(id0, *typ0)];
        loop {
            if self.consume_if(Tok::RBrace) {
                break;
            }

            self.expect_token(Tok::Comma)?;
            let (_, id) = self.expect_id()?;
            self.expect_token(Tok::Colon)?;
            let typ = self.parse()?;
            fields.push((id, *typ));
        }
        Ok(Box::new(Node::RecordType {
            sloc,
            typ: None,
            fields,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(
        input: &'static str,
        spool: &mut std::collections::HashSet<Rc<str>>,
    ) -> Result<Box<Node>, Error> {
        let mut lexer = Lexer::new(input, 0, spool);
        let mut parser = Parser::new(&mut lexer);
        parser.parse()
    }

    #[test]
    fn lambda() {
        let mut spool = std::collections::HashSet::new();
        let node = parse("let inc = λ(x: Int) -> x + 1 in inc(41)", &mut spool).unwrap();

        assert!(match node.as_ref() {
            Node::LetIn {
                name, value, body, ..
            } if name.as_ref() == "inc" =>
                (match value.as_ref() {
                    Node::Lambda { args, body, .. } if args.len() == 1 =>
                        (match &args[0] {
                            (name, None, typ) if name.as_ref() == "x" => match typ.as_ref() {
                                Node::Id { name, .. } if name.as_ref() == "Int" => true,
                                _ => false,
                            },
                            _ => false,
                        } && match &*body.borrow() {
                            Node::BinOp {
                                op: BinOp::Add,
                                lhs,
                                rhs,
                                iscmp: false,
                                ..
                            } =>
                                (match lhs.as_ref() {
                                    Node::Id { name, .. } if name.as_ref() == "x" => true,
                                    _ => false,
                                } && match rhs.as_ref() {
                                    Node::Integer { value, .. } if *value == 1 => true,
                                    _ => false,
                                }),
                            _ => false,
                        }),
                    _ => false,
                } && match body.as_ref() {
                    Node::Call { callable, args, .. } if args.len() == 1 =>
                        (match callable.as_ref() {
                            Node::Id { name, .. } if name.as_ref() == "inc" => true,
                            _ => false,
                        } && match args[0] {
                            Node::Integer { value, .. } if value == 41 => true,
                            _ => false,
                        }),
                    _ => false,
                }),
            _ => false,
        });
    }
}
