use std::fmt;
use std::{cell::RefCell, rc::Rc};

use crate::core::{Error, SLoc, Type, Value};
use crate::eval::Runtime;
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
    As {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: Box<Node>,
        as_raw: Box<Node>,
        as_typ: Option<Rc<Type>>,
    },
    TypeOf {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: Box<Node>
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
            Node::As { typ, .. } => typ,
            Node::TypeOf { typ, .. } => typ,
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
            Node::As { typ, .. } => *typ = Some(t),
            Node::TypeOf { typ, .. } => *typ = Some(t),
        };
    }

    pub fn typecheck(
        &mut self,
        rt: &mut Runtime,
        _hint: Option<Rc<Type>>,
    ) -> Result<Rc<Type>, Error> {
        match self {
            Node::Id { typ: Some(t), .. } => Ok(t.clone()),
            Node::Id { sloc, typ, name } => match rt.lookup(name) {
                Some(Value::Type(t)) if *t == Type::TypeOfType => {
                    let t = rt.type_type.clone();
                    *typ = Some(t.clone());
                    Ok(t)
                }
                Some(Value::Type(t)) if *t != Type::TypeOfType => {
                    let t = Rc::new(Type::TypeOf(t));
                    *typ = Some(t.clone());
                    Ok(t)
                }
                Some(v) => {
                    let t = v.get_type();
                    *typ = Some(t.clone());
                    Ok(t)
                }
                None => Err(Error::TypeError(
                    *sloc,
                    format!("undefined id: {}", name.as_ref()),
                )),
            },
            Node::Integer { typ: Some(t), .. } => Ok(t.clone()),
            Node::Integer { typ, .. } => {
                let t = rt.int_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::Boolean { typ: Some(t), .. } => Ok(t.clone()),
            Node::Boolean { typ, .. } => {
                let t = rt.bool_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::String { typ: Some(t), .. } => Ok(t.clone()),
            Node::String { typ, .. } => {
                let t = rt.text_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::TypeAnno { typ: Some(t), .. } => Ok(t.clone()),
            Node::TypeAnno {
                sloc,
                typ,
                op0,
                rawtyp,
            } => match rawtyp.typecheck(rt, None)?.as_ref() {
                Type::TypeOf(t) => {
                    let op0t = op0.typecheck(rt, Some(t.clone()))?;
                    if *op0t != **t {
                        return Err(Error::TypeError(
                            *sloc,
                            format!(
                                "{} expected to be of type {}, found {}",
                                op0,
                                t,
                                op0t.as_ref()
                            ),
                        ));
                    }
                    *typ = Some(op0t.clone());
                    Ok(op0t)
                }
                other => Err(Error::TypeError(
                    *sloc,
                    format!("expected a type, found {} ({:?})", other, other),
                )),
            },
            Node::Invert { typ: Some(t), .. } => Ok(t.clone()),
            Node::Invert { sloc, typ, op0 } => {
                let t = op0.typecheck(rt, None)?;
                if *t != Type::Bool && *t != Type::Int {
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
                let lhsty = lhs.typecheck(rt, None)?;
                let rhsty = rhs.typecheck(rt, None)?;
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
                    (_, true, Type::Int) => rt.bool_type.clone(),
                    (BinOp::And | BinOp::Or, _, Type::Bool) => lhsty,
                    (BinOp::And | BinOp::Or, _, _) => {
                        return Err(Error::TypeError(
                            *sloc,
                            "'&' and '|' only work for booleans".to_string(),
                        ))
                    }
                    (_, false, Type::Int) => lhsty,
                    (BinOp::Add, _, Type::Text) => lhsty,
                    (op, _, _) => panic!(
                        "binop error: {:?} {:?} {:?}",
                        lhsty.as_ref(),
                        op,
                        rhsty.as_ref()
                    ),
                };
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::IfThenElse { typ: Some(t), .. } => Ok(t.clone()),
            Node::IfThenElse {
                sloc,
                typ,
                op0,
                op1,
                op2,
            } => {
                let op0ty = op0.typecheck(rt, None)?;
                if *op0ty != Type::Bool {
                    return Err(Error::TypeError(
                        *sloc,
                        format!(
                            "condition of if-then-else needs to be a boolean, not: {}",
                            op0ty.as_ref()
                        ),
                    ));
                }

                let op1ty = op1.typecheck(rt, None)?;
                let op2ty = op2.typecheck(rt, None)?;
                if *op1ty != *op2ty {
                    return Err(Error::TypeError(
                        *sloc,
                        format!(
                            "branches of if-then-else needs to be of same type, not: {} and {}",
                            op1ty.as_ref(),
                            op2ty.as_ref()
                        ),
                    ));
                }

                *typ = Some(op1ty.clone());
                Ok(op1ty)
            }
            Node::Lambda { typ: Some(t), .. } => Ok(t.clone()),
            Node::Lambda {
                sloc,
                typ,
                args,
                body,
            } => {
                let mut arg_types = Vec::with_capacity(args.len());
                for (arg_name, arg_typ, arg_typ_ast) in args.iter_mut() {
                    let t = arg_typ_ast.typecheck(rt, None)?;
                    match t.as_ref() {
                        Type::TypeOfType => {
                            *arg_typ = Some(t.clone());
                            arg_types.push((arg_name.clone(), t.clone()));
                            let ph = Rc::new(Type::Placeholder(arg_name.clone()));
                            rt.push(arg_name, Value::Type(ph));
                        }
                        Type::TypeOf(t) => {
                            *arg_typ = Some(t.clone());
                            arg_types.push((arg_name.clone(), t.clone()));
                            rt.push(arg_name, Value::Pseudo(t.clone()));
                        }
                        _ => {
                            return Err(Error::TypeError(
                                *sloc,
                                format!(
                                    "{} expected to be a type in lambda argument list, found {}",
                                    arg_name.as_ref(),
                                    t
                                ),
                            ))
                        }
                    };
                }
                let ret_type = body.borrow_mut().typecheck(rt, None)?;
                rt.pop(args.len());
                let t = Rc::new(Type::Lambda(arg_types, ret_type));
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
                let mut arg_types = Vec::with_capacity(argtypes.len());
                for (arg_name, arg_typ, arg_typ_ast) in argtypes.iter_mut() {
                    let t = arg_typ_ast.typecheck(rt, None)?;
                    match t.as_ref() {
                        Type::TypeOfType => {
                            *arg_typ = Some(t.clone());
                            arg_types.push((arg_name.clone(), t.clone()));
                            let ph = Rc::new(Type::Placeholder(arg_name.clone()));
                            rt.push(arg_name, Value::Type(ph));
                        }
                        Type::TypeOf(t) => {
                            *arg_typ = Some(t.clone());
                            arg_types.push((arg_name.clone(), t.clone()));
                            rt.push(arg_name, Value::Pseudo(t.clone()));
                        }
                        _ => {
                            return Err(Error::TypeError(
                                *sloc,
                                format!(
                                    "{} expected to be a type in forall argument list, found {}",
                                    arg_name.as_ref(),
                                    t
                                ),
                            ))
                        }
                    };
                }
                let ret_type = rettyp.borrow_mut().typecheck(rt, None)?;
                let ret_type = match ret_type.as_ref() {
                    Type::TypeOf(t) => t.clone(),
                    other => {
                        return Err(Error::TypeError(
                            *sloc,
                            format!("expected a type, found: {}", other),
                        ))
                    }
                };
                rt.pop(argtypes.len());
                let t = Rc::new(Type::TypeOf(Rc::new(Type::Lambda(arg_types, ret_type))));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::Call { typ: Some(t), .. } => Ok(t.clone()),
            Node::Call {
                sloc,
                typ,
                callable,
                args,
            } => {
                let callable_typ = callable.typecheck(rt, None)?;
                let Type::Lambda(arg_types, ret_type) = callable_typ.as_ref() else {
                    return Err(Error::Uncallable(*sloc, format!("not callable: {}", callable)));
                };

                if args.len() != arg_types.len() {
                    return Err(Error::Uncallable(
                        *sloc,
                        format!("not callable with {} arguments: {}", args.len(), callable),
                    ));
                }

                let mut ret_type = ret_type.clone();
                for (arg, (arg_name, arg_type)) in args.iter_mut().zip(arg_types) {
                    let typ = arg.typecheck(rt, None)?;
                    if **arg_type == Type::TypeOfType {
                        ret_type = ret_type.subst(
                            arg_name.as_ref(),
                            match typ.as_ref() {
                                Type::TypeOf(t) => t,
                                _ => panic!(),
                            },
                        );
                        continue;
                    }
                    if **arg_type != *typ {
                        return Err(Error::Uncallable(
                            *sloc,
                            format!(
                                "argument {} has type {} ({:?}), expected: {} ({:?})",
                                arg_name.as_ref(),
                                typ.as_ref(),
                                typ.as_ref(),
                                arg_type.as_ref(),
                                arg_type.as_ref()
                            ),
                        ));
                    }
                }

                *typ = Some(ret_type.clone());
                Ok(ret_type)
            }
            Node::LetIn { typ: Some(t), .. } => Ok(t.clone()),
            Node::LetIn {
                sloc: _,
                typ,
                name,
                value,
                typeannot: None,
                body,
            } => {
                let valuety = value.typecheck(rt, None)?;
                rt.push(name, Value::Pseudo(valuety));
                let bodyty = body.typecheck(rt, None)?;
                rt.pop(1);
                *typ = Some(bodyty.clone());
                Ok(bodyty)
            }
            Node::LetIn {
                sloc,
                typ,
                name,
                value,
                typeannot: Some(rawtypeannot),
                body,
            } => {
                let annot = rawtypeannot.typecheck(rt, None)?;
                let annot = match annot.as_ref() {
                    Type::TypeOf(t) => t,
                    other => {
                        return Err(Error::TypeError(
                            *sloc,
                            format!(
                                "expected a type in let-in annotation, not something of type {}",
                                other
                            ),
                        ))
                    }
                };
                rt.push(name, Value::Pseudo(annot.clone()));
                let valuety = value.typecheck(rt, Some(annot.clone()))?;
                if **annot != *valuety {
                    return Err(Error::TypeError(
                        *sloc,
                        format!("type missmatch: {} vs {}", valuety.as_ref(), annot.as_ref()),
                    ));
                }
                let bodyty = body.typecheck(rt, None)?;
                rt.pop(1);
                *typ = Some(bodyty.clone());
                Ok(bodyty)
            }
            Node::Record { typ: Some(t), .. } => Ok(t.clone()),
            Node::Record { typ, fields, .. } => {
                let fields: Result<Vec<_>, _> = fields
                    .iter_mut()
                    .map(|(name, val)| val.typecheck(rt, None).map(|t| (name.clone(), t)))
                    .collect();
                let t = Rc::new(Type::Record(fields?));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::RecordType { typ: Some(t), .. } => Ok(t.clone()),
            Node::RecordType {
                sloc, typ, fields, ..
            } => {
                // TODO: field type must be type!
                let fields: Result<Vec<_>, _> = fields
                    .iter_mut()
                    .map(|(name, rawtyp)| {
                        rawtyp
                            .typecheck(rt, None)
                            .and_then(|t| match t.as_ref() {
                                Type::TypeOf(t) => Ok(t.clone()),
                                other => Err(Error::TypeError(
                                    *sloc,
                                    format!(
                                        "expected a type in record type, not something of type {}",
                                        other
                                    ),
                                )),
                            })
                            .map(|t| (name.clone(), t))
                    })
                    .collect();
                let t = Rc::new(Type::TypeOf(Rc::new(Type::Record(fields?))));
                *typ = Some(t.clone());
                Ok(t)
            }
            Node::AccessField { typ: Some(t), .. } => Ok(t.clone()),
            Node::AccessField {
                sloc,
                typ,
                op0,
                field,
            } => match op0.typecheck(rt, None)?.as_ref() {
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
            Node::As { typ: Some(t), .. } => Ok(t.clone()),
            Node::As {
                sloc,
                typ,
                op0,
                as_typ,
                as_raw,
            } => {
                let t = as_raw.typecheck(rt, None)?;
                let t = match t.as_ref() {
                    Type::TypeOf(t) => t,
                    other => {
                        return Err(Error::TypeError(
                            *sloc,
                            format!(
                                "expected a type on the right-hand-side of 'as', not: {}",
                                other
                            ),
                        ))
                    }
                };
                *as_typ = Some(t.clone());
                let op0type = op0.typecheck(rt, None)?;
                match (op0type.as_ref(), t.as_ref()) {
                    (_, Type::Any) => {
                        *typ = Some(t.clone());
                        Ok(t.clone())
                    }
                    (Type::Any, _) => {
                        let t = Rc::new(Type::Option(t.clone()));
                        *typ = Some(t.clone());
                        Ok(t)
                    }
                    (_, Type::Text) => {
                        let t = rt.text_type.clone();
                        *typ = Some(t.clone());
                        Ok(t)
                    }
                    (a, b) => todo!("{} as {}", a, b),
                }
            }
            Node::TypeOf { typ: Some(t), .. } => Ok(t.clone()),
            Node::TypeOf { typ, op0, .. } => {
                let t = op0.typecheck(rt, None)?;
                *typ = Some(t.clone());
                let t = Rc::new(Type::TypeOf(t));
                Ok(t)
            }
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
                write!(f, "({})(", callable.as_ref())?;
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
                write!(f, ") -> ({})", body.borrow())
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
                write!(f, ") -> ({})", rettyp.borrow())
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
            Node::As { op0, as_raw, .. } => write!(f, "({} as {})", op0, as_raw),
            Node::TypeOf { op0, .. } => write!(f, "typeof({})", op0)
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
                true => Some(self.parse_expr0()?),
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
        let mut expr = self.parse_binop(0)?;
        if self.consume_if(Tok::Colon) {
            let typhint = self.parse_expr0()?;
            expr = Box::new(Node::TypeAnno {
                sloc: self.consumed_sloc,
                typ: None,
                op0: expr,
                rawtyp: typhint,
            });
        }
        if self.consume_if(Tok::As) {
            let typ = self.parse_expr0()?;
            expr = Box::new(Node::As {
                sloc: self.consumed_sloc,
                typ: None,
                op0: expr,
                as_raw: typ,
                as_typ: None,
            });
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
            Tok::Bool(value) => Node::Boolean {
                sloc,
                typ: None,
                value,
            },
            Tok::Int(value) => Node::Integer {
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
            Tok::Typeof => Node::TypeOf {
                sloc,
                typ: None,
                op0: self.parse_expr0()?
            },
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
