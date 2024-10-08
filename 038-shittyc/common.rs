use std::{fmt::Display, rc::Rc};

use crate::lex::Tok;

#[derive(Clone, PartialEq, Eq, PartialOrd, Debug)]
pub struct SLoc {
    pub file: Rc<std::path::Path>,
    pub line: u32,
    pub col: u32,
}

impl SLoc {
    pub fn new(file: &std::path::Path, line: usize, col: usize) -> Self {
        Self {
            file: Rc::from(file),
            line: line.min(u32::MAX as usize) as u32,
            col: col.min(u32::MAX as usize) as u32,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    IO(SLoc, std::io::Error),
    UnresolvedSymbol(SLoc, Rc<str>),
    Type(SLoc, Type, &'static str),
    EndOfFile(SLoc),
    PreProcessor(SLoc, Tok, &'static str),
    InvalidInt(SLoc, std::num::ParseIntError),
    InvalidTok(SLoc, &'static str),
    UnexpectedTok(SLoc, String),
    Lex(SLoc, String),
    ExpectedType(SLoc, Tok),
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Debug)]
pub enum Type {
    Unknown,
    Void,
    Bool,
    Int { bits: u8, signed: bool },
    Ptr { ety: Rc<Type>, volatile: bool, constant: bool, restrict: bool },
    Array(Rc<Type>, Option<usize>),
    Struct { name: Option<Rc<str>>, fields: Rc<Vec<(Rc<str>, Type)>> },
    Union { name: Option<Rc<str>>, fields: Rc<Vec<(Rc<str>, Type)>> },
    Enum { name: Option<Rc<str>>, ety: Rc<Type>, vals: Rc<Vec<(Rc<str>, u64)>> },
    Fn { retty: Rc<Type>, argtys: Rc<Vec<Type>> },
}

impl Type {
    pub fn is_bool(&self) -> bool { *self == Type::Bool }
    pub fn is_numerical(&self) -> bool {
        match self {
            Self::Int { .. } => true,
            _ => false
        }
    }
    pub fn is_pointer(&self) -> bool {
        match self {
            Self::Ptr { .. } => true,
            _ => false
        }
    }

    pub fn lookup_field(&self, sloc: &SLoc, name: Rc<str>) -> Result<(Type, usize), Error> {
        let fields = match self {
            Type::Struct { name: _, fields } => fields,
            Type::Union { name: _, fields } => fields,
            other => return Err(Error::Type(sloc.clone(), other.clone(), "struct or union expected"))
        };

        for (idx, (fname, ftyp)) in fields.iter().enumerate() {
            if &**fname == &*name {
                return Ok((ftyp.clone(), idx))
            }
        }

        return Err(Error::Type(sloc.clone(), self.clone(), "struct or union does not have such a field"))
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Unknown => write!(f, "???"),
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::Int { bits, signed } => write!(f, "{}int{}_t",
                if *signed { "" } else { "u" }, bits),
            Type::Ptr { ety, volatile, constant, restrict } => {
                // TODO: Check if ety is a function or array, and change
                // repr. in that case.
                write!(f, "{}{}{}{}*", &**ety,
                    if *volatile {" volatile"} else {""},
                    if *constant {" constant"} else {""},
                    if *restrict {" restrict"} else {""})?;
                Ok(())
            }
            Type::Array(ety, Some(nelms)) => write!(f, "{}[{}]", &**ety, nelms),
            Type::Array(ety, None) => write!(f, "{}[]", &**ety),
            Type::Struct { name, fields: _ } if name.is_some()
                => write!(f, "struct {}", name.clone().unwrap()),
            Type::Struct { name: _, fields } => {
                write!(f, "struct {{")?;
                for (name, typ) in fields.iter() {
                    write!(f, " {} {};", typ, name)?;
                }
                write!(f, " }}")
            },
            Type::Union { name, fields: _ } if name.is_some()
                => write!(f, "union {}", name.clone().unwrap()),
            Type::Union { name: _, fields } => {
                write!(f, "union {{")?;
                for (name, typ) in fields.iter() {
                    write!(f, " {} {};", typ, name)?;
                }
                write!(f, " }}")
            },
            Type::Enum { name, ety: _, vals: _ } if name.is_some()
                => write!(f, "enum {}", name.clone().unwrap()),
            Type::Enum { name: _, ety, vals } => {
                write!(f, "enum: {} {{", &**ety)?;
                for (i, (name, val)) in vals.iter().enumerate() {
                    write!(f, "{}{} = {:x}", if i == 0 {" "} else {", "}, name, val)?;
                }
                write!(f, " }}")
            },
            Type::Fn { retty, argtys } => {
                write!(f, "{}(*)(", &**retty)?;
                for (i, typ) in argtys.iter().enumerate() {
                    write!(f, "{}{}", if i == 0 {""} else {", "}, typ)?;
                }
                write!(f, ")")
            }
        }
    }
}
