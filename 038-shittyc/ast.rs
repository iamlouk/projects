use std::{cell::RefCell, collections::HashMap, fmt::Display, hash::Hash, rc::Rc};

use crate::{common::*, lex::{Lexer, Tok}};

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub struct Function {
    pub name: Rc<str>,
    pub sloc: SLoc,
    pub retty: Type,
    pub args: Vec<(Rc<str>, Type)>,
    pub body: Option<Box<Stmt>>,
    pub is_static: bool,
    pub locals: Vec<Rc<Decl>>
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Decl {
    pub sloc: SLoc,
    pub is_argument: bool,
    pub is_local: bool,
    pub name: Rc<str>,
    pub ty: Type,
    pub init: Option<Box<Expr>>,
    pub func: RefCell<Option<Rc<Function>>>, // FIXME: Might cause ref. count loop for rec. functions.
    pub idx: usize
}

impl Hash for Decl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i8((self.is_local as i8) * 2 + (self.is_argument as i8));
        state.write_usize(self.idx);
    }
}

impl PartialEq for Decl {
    fn eq(&self, other: &Self) -> bool { self.sloc == other.sloc }
}

impl Eq for Decl {}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    NoOp     { sloc: SLoc, ident: u8 },
    Expr     { sloc: SLoc, ident: u8, expr: Box<Expr> },
    Decls    { sloc: SLoc, ident: u8, decls: Vec<Rc<Decl>> },
    Compound { sloc: SLoc, ident: u8, stmts: Vec<Stmt> },
    While    { sloc: SLoc, ident: u8, cond: Box<Expr>, body: Box<Stmt> },
    For      { sloc: SLoc, ident: u8, init: Box<Stmt>, cond: Box<Expr>, incr: Box<Stmt>, body: Box<Stmt> },
    If       { sloc: SLoc, ident: u8, cond: Box<Expr>, then: Box<Stmt>, otherwise: Option<Box<Stmt>> },
    Ret      { sloc: SLoc, ident: u8, val: Option<Box<Expr>> },
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOp {
    EQ, NE, LT, LE, GT, GE,
    Add, Sub, Mul, Div, Mod, BitwiseAnd, BitwiseOr, BitwiseXOr,
    LogicalAnd, LogicalOr
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnaryOp { Neg, LogicalNot, BitwiseNot }

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Id { sloc: SLoc, typ: Type, name: Rc<str>, decl: Rc<Decl> },
    Int { sloc: SLoc, typ: Type, num: i64 },
    Assign {
        sloc: SLoc, typ: Type, op: Option<BinOp>,
        lhs: Box<Expr>, rhs: Box<Expr>
    },
    Cast {
        sloc: SLoc, typ: Type, val: Box<Expr>
    },
    UnaryOp {
        sloc: SLoc, typ: Type, op: UnaryOp, val: Box<Expr>
    },
    BinOp {
        sloc: SLoc, typ: Type, op: BinOp,
        lhs: Box<Expr>, rhs: Box<Expr>,
    },
    Call {
        sloc: SLoc, typ: Type,
        func: Box<Expr>, args: Vec<Expr>,
    },
    Deref {
        sloc: SLoc, typ: Type, ptr: Box<Expr>,
    },
    FieldAccess {
        sloc: SLoc, typ: Type,
        obj: Box<Expr>, field: Rc<str>, idx: usize
    },
    Subscript { // TODO: Remove, this is just `*(a+offset)`.
        sloc: SLoc, typ: Type,
        ptr: Box<Expr>, offset: Box<Expr>
    },
    Tenary {
        sloc: SLoc, typ: Type,
        cond: Box<Expr>, then: Box<Expr>, otherwise: Box<Expr>
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Id { name, .. } => write!(f, "{}", name),
            Expr::Int { num, .. } => write!(f, "{:#x}", num),
            Expr::Assign { op: Some(op), lhs, rhs, .. } =>
                write!(f, "({}) {}= ({})", lhs, Expr::binop_to_str(*op), rhs),
            Expr::Assign { op: None, lhs, rhs, .. } =>
                write!(f, "({}) = ({})", lhs, rhs),
            Expr::Cast { typ, val, .. } => write!(f, "({})({})", typ, val),
            Expr::UnaryOp { op, val, .. } => match op {
                UnaryOp::Neg => write!(f, "-({})", val),
                UnaryOp::BitwiseNot => write!(f, "~({})", val),
                UnaryOp::LogicalNot => write!(f, "!({})", val)
            },
            Expr::BinOp { op, lhs, rhs, .. } =>
                write!(f, "({}) {} ({})", lhs, Expr::binop_to_str(*op), rhs),
            Expr::Call { func, args, .. } => {
                write!(f, "({})(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    write!(f, "{}{}", if i == 0 {""} else {", "}, arg)?;
                }
                write!(f, ")")
            },
            Expr::Deref { ptr, .. } => write!(f, "*({})", ptr),
            Expr::FieldAccess { obj, field, .. } => write!(f, "({}).{}", obj, &**field),
            Expr::Subscript { ptr, offset, .. } => write!(f, "({})[{}]", ptr, offset),
            Expr::Tenary { cond, then, otherwise, .. } =>
                write!(f, "({}) ? ({}) : ({})", cond, then, otherwise)
        }
    }
}

impl Display for Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

impl Expr {
    fn get_typ(&self) -> Type {
        (match self {
            Expr::Id          { typ, .. } => typ,
            Expr::Int         { typ, .. } => typ,
            Expr::Assign      { typ, .. } => typ,
            Expr::Cast        { typ, .. } => typ,
            Expr::UnaryOp     { typ, .. } => typ,
            Expr::BinOp       { typ, .. } => typ,
            Expr::Call        { typ, .. } => typ,
            Expr::Deref       { typ, .. } => typ,
            Expr::FieldAccess { typ, .. } => typ,
            Expr::Subscript   { typ, .. } => typ,
            Expr::Tenary      { typ, .. } => typ,
        }).clone()
    }

    fn binop_to_str(op: BinOp) -> &'static str {
        match op {
            BinOp::Add => "+", BinOp::Sub => "-",
            BinOp::Mul => "*", BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::BitwiseAnd => "&",
            BinOp::BitwiseOr => "|",
            BinOp::BitwiseXOr => "^",
            BinOp::EQ => "==", BinOp::NE => "!=",
            BinOp::GE => ">=", BinOp::GT => ">",
            BinOp::LE => "<=", BinOp::LT => "<",
            BinOp::LogicalOr => "||",
            BinOp::LogicalAnd => "&&"
        }
    }

    fn is_cmp(op: BinOp) -> bool {
        match op {
            BinOp::EQ | BinOp::NE |
            BinOp::GE | BinOp::GT |
            BinOp::LE | BinOp::LT => true,
            _ => false
        }
    }

    fn is_assignable(&self) -> bool {
        match self {
            Expr::Id { .. } => true,
            Expr::Deref { .. } => true,
            Expr::FieldAccess { .. } => true,
            _ => false
        }
    }

    pub fn is_constant(&self, max: i64) -> Option<i64> {
        match self {
            Expr::Int { num, .. } if *num >= 0 && *num < max => Some(*num),
            _ => None
        }
    }
}

pub struct Parser {
    types: HashMap<Rc<str>, Type>,
    globals: HashMap<Rc<str>, Rc<Decl>>,
    current_function: Option<Box<Function>>
}

#[allow(dead_code)]
impl Parser {
    pub fn new() -> Self {
        Parser {
            types: HashMap::new(), // TODO: Do stuff with this.
            globals: HashMap::new(),
            current_function: None
        }
    }

    fn lookup(&self, sloc: &SLoc, name: Rc<str>) -> Result<Rc<Decl>, Error> {
        if let Some(decl) = self.current_function
                .as_ref().unwrap().locals.iter()
                .find(|local| &*local.name == &*name) {
            return Ok(decl.clone())
        }

        if let Some(decl) = self.globals.get(&name) {
            return Ok(decl.clone())
        }

        Err(Error::UnresolvedSymbol(sloc.clone(), name))
    }

    pub fn parse_function(&mut self, lex: &mut Lexer) -> Result<Option<Rc<Function>>, Error> {
        if lex.peek()?.1 == Tok::EndOfFile {
            return Ok(None)
        }

        let is_static = lex.consume_if_next(Tok::Static)?;
        let retty = self.parse_type(lex)?;
        let (sloc, name) = lex.expect_id("function name")?;
        lex.expect_token(Tok::LParen, "start of function parameter list")?;
        let mut args = Vec::new();
        let mut locals = Vec::new();
        loop {
            if lex.consume_if_next(Tok::RParen)? {
                break;
            }

            let argty = self.parse_type(lex)?;
            let (sloc, name) = lex.expect_id("parameter name")?;
            args.push((name.clone(), argty.clone()));
            locals.push(Rc::new(Decl {
                sloc, is_argument: true, is_local: true, name, ty: argty,
                init: None, idx: locals.len(), func: RefCell::new(None) }));
            if !lex.consume_if_next(Tok::Comma)? {
                lex.expect_token(Tok::RParen, "end of parameter list")?;
                break;
            }
        }

        let decl = Rc::new(Decl {
            sloc: sloc.clone(),
            is_argument: false, is_local: false, name: name.clone(),
            ty: Type::Fn {
                retty: Rc::new(retty.clone()),
                argtys: Rc::new(args.iter().map(|(_, t)| t.clone()).collect())
            },
            init: None, func: RefCell::new(None), idx: 0
        });
        self.globals.insert(name.clone(), decl.clone());

        if lex.consume_if_next(Tok::SemiColon)? {
            return Ok(Some(Rc::new(Function {
                name, sloc, retty, args,
                body: None, is_static, locals: Vec::new()
            })))
        }

        self.current_function = Some(Box::new(Function {
            name, sloc, retty, args,
            body: None, is_static, locals
        }));

        let body = self.parse_stmt(lex, 1)?;
        let mut f = self.current_function.take().unwrap();
        f.body = Some(body);
        let f = Rc::new(*f);
        decl.func.replace(Some(f.clone()));
        Ok(Some(f))
    }

    fn parse_stmt(&mut self, lex: &mut Lexer, ident: u8) -> Result<Box<Stmt>, Error> {
        let (sloc, tok) = lex.peek()?;
        if Tok::SemiColon == tok {
            lex.next()?;
            return Ok(Box::new(Stmt::NoOp { sloc, ident }))
        }

        if Tok::Return == tok {
            lex.next()?;
            if lex.peek()?.1 == Tok::SemiColon {
                let expected = self.current_function.as_ref().unwrap().retty.clone();
                if expected != Type::Void {
                    return Err(Error::Type(sloc, expected, "expected non-void return"))
                }
                lex.next()?;
                return Ok(Box::new(Stmt::Ret { sloc, ident, val: None }))
            }

            let expr = self.parse_expr(lex)?;
            lex.expect_token(Tok::SemiColon, "end of return statement")?;
            let expected = self.current_function.as_ref().unwrap().retty.clone();
            if expected != expr.get_typ() {
                return Err(Error::Type(sloc, expected, "wrong return type"))
            }
            return Ok(Box::new(Stmt::Ret { sloc, ident, val: Some(expr) }))
        }

        if Tok::LBraces == tok {
            lex.next()?;
            let mut stmts = vec![];
            while lex.peek()?.1 != Tok::RBraces {
                stmts.push(*self.parse_stmt(lex, ident + 1)?);
            }
            lex.next()?;
            return Ok(Box::new(Stmt::Compound { sloc, ident, stmts }))
        }

        if Tok::While == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen, "while condition")?;
            let cond = self.parse_expr(lex)?;
            lex.expect_token(Tok::RParen, "while condition")?;
            if cond.get_typ() != Type::Bool {
                return Err(Error::Type(sloc, cond.get_typ(), "expected boolean condition for while"))
            }
            let body = self.parse_stmt(lex, ident + 1)?;
            return Ok(Box::new(Stmt::While { sloc, ident, cond, body }))
        }

        if Tok::For == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen, "expected '(' after for")?;
            let init = self.parse_stmt(lex, 0)?;
            let cond = self.parse_expr(lex)?;
            if cond.get_typ() != Type::Bool {
                return Err(Error::Type(sloc, cond.get_typ(), "expected boolean condition for while"))
            }
            lex.expect_token(Tok::SemiColon, "expected for loop condition")?;
            let incr = self.parse_stmt(lex, 0)?;
            lex.expect_token(Tok::LParen, "expected ')' after for")?;
            let body = self.parse_stmt(lex, ident + 1)?;
            return Ok(Box::new(Stmt::For { sloc, ident, init, cond, incr, body }))
        }

        if Tok::If == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen, "if condition")?;
            let cond = self.parse_expr(lex)?;
            lex.expect_token(Tok::RParen, "if condition")?;
            if cond.get_typ() != Type::Bool {
                return Err(Error::Type(sloc, cond.get_typ(), "expected boolean condition for if"))
            }
            let then = self.parse_stmt(lex, ident + 1)?;
            let mut otherwise: Option<Box<Stmt>> = None;
            if Tok::Else == lex.peek()?.1 {
                lex.next()?;
                otherwise = Some(self.parse_stmt(lex, ident + 1)?)
            }

            return Ok(Box::new(Stmt::If { sloc, ident, cond, then, otherwise }))
        }

        let ty = match self.parse_type(lex) {
            Ok(t) => t,
            Err(Error::ExpectedType(sloc, tok)) => {
                lex.unread(sloc.clone(), tok);
                let expr = self.parse_expr(lex)?;
                lex.expect_token(Tok::SemiColon, "expression statement")?;
                return Ok(Box::new(Stmt::Expr { sloc, ident, expr }))
            }
            Err(e) => return Err(e)
        };

        let mut decls: Vec<Rc<Decl>> = Vec::new();
        loop {
            let (sloc, name) = lex.expect_id("local declaration name")?;
            let init = if lex.consume_if_next(Tok::Assign)? {
                let expr = self.parse_expr(lex)?;
                if expr.get_typ() != ty {
                    return Err(Error::Type(sloc, expr.get_typ(), "wrong initializer type"))
                }
                Some(expr)
            } else {
                None
            };
            let ld = Rc::new(Decl {
                sloc, name, ty: ty.clone(),
                init, idx: self.current_function.as_ref().unwrap().locals.len(),
                is_argument: false, is_local: true, func: RefCell::new(None) });
            decls.push(ld.clone());
            self.current_function.as_mut().unwrap().locals.push(ld);
            if lex.consume_if_next(Tok::Comma)? {
                continue;
            }

            lex.expect_token(Tok::SemiColon, "end of declarations")?;
            break;
        }

        Ok(Box::new(Stmt::Decls { sloc, ident, decls }))
    }

    fn parse_expr(&mut self, lex: &mut Lexer) -> Result<Box<Expr>, Error> {
        let expr = self.parse_binary_expr(lex, 0)?;
        let (sloc, tok) = lex.peek()?;
        if let Some(op) = match tok {
            Tok::Assign => Some(None),
            Tok::AssignAdd => Some(Some(BinOp::Add)),
            Tok::AssignSub => Some(Some(BinOp::Sub)),
            _ => None
        } {
            if !expr.is_assignable() {
                return Err(Error::Type(sloc, expr.get_typ(), "cannot assign to this expr. kind"))
            }

            lex.next()?;
            let rhs = self.parse_expr(lex)?;
            if expr.get_typ() != rhs.get_typ() {
                return Err(Error::Type(sloc, rhs.get_typ(), "both sides of assignment need to be of equal type"))
            }
            return Ok(Box::new(Expr::Assign {
                sloc, typ: Type::Unknown, op,
                lhs: expr, rhs }))
        }

        if lex.consume_if_next(Tok::QuestionMark)? {
            if expr.get_typ() != Type::Bool {
                return Err(Error::Type(sloc, expr.get_typ(), "expected boolean condition for tenary op."))
            }

            let sloc = lex.peek()?.0;
            let then = self.parse_expr(lex)?;
            lex.expect_token(Tok::Colon, "tenary expression")?;
            let otherwise = self.parse_expr(lex)?;
            if then.get_typ() != otherwise.get_typ() {
                return Err(Error::Type(sloc, otherwise.get_typ(), "expected both branches of tenary to have same type"))
            }
            return Ok(Box::new(Expr::Tenary {
                sloc, typ: then.get_typ(), cond: expr, then, otherwise }))
        }
        Ok(expr)
    }

    fn parse_binary_expr(&mut self, lex: &mut Lexer, min_prec: u64) -> Result<Box<Expr>, Error> {
        fn precedence(tok: Tok) -> Option<(BinOp, u64)> {
            match tok {
                Tok::LogicalOr      => Some((BinOp::LogicalOr,  100)),
                Tok::LogicalAnd     => Some((BinOp::LogicalAnd, 100)),
                Tok::BitwiseOr      => Some((BinOp::BitwiseOr,  200)),
                Tok::Ampersand      => Some((BinOp::BitwiseAnd, 200)),
                Tok::BitwiseXOr     => Some((BinOp::BitwiseXOr, 200)),
                Tok::Equal          => Some((BinOp::EQ,         300)),
                Tok::NotEqual       => Some((BinOp::NE,         300)),
                Tok::Smaller        => Some((BinOp::LT,         400)),
                Tok::Bigger         => Some((BinOp::GT,         400)),
                Tok::SmallerOrEqual => Some((BinOp::LE,         400)),
                Tok::BiggerOrEqual  => Some((BinOp::GT,         400)),
                Tok::Plus           => Some((BinOp::Add,        600)),
                Tok::Minus          => Some((BinOp::Sub,        600)),
                Tok::Star           => Some((BinOp::Mul,        700)),
                Tok::Divide         => Some((BinOp::Div,        700)),
                Tok::Modulo         => Some((BinOp::Mod,        700)),
                _ => None
            }
        }

        let mut lhs = self.parse_final_expr(lex)?;
        while let Some((op, prec)) = precedence(lex.peek()?.1) {
            if prec < min_prec { break }
            let (sloc, _) = lex.next()?;
            let rhs = self.parse_binary_expr(lex, prec + 1)?;
            let t = lhs.get_typ();
            if t != rhs.get_typ() {
                return Err(Error::Type(sloc, t, "different types on sides of boolean expr."))
            }
            if (op == BinOp::LogicalOr || op == BinOp::LogicalAnd) && !t.is_bool() {
                return Err(Error::Type(sloc, t, "'&&' and '||' operands need to be boolean"))
            }
            if !(op == BinOp::LogicalOr || op == BinOp::LogicalAnd) && !t.is_numerical() {
                return Err(Error::Type(sloc, t, "expected operands of numerical type"))
            }
            lhs = Box::new(Expr::BinOp {
                sloc, typ: if Expr::is_cmp(op) { Type::Bool } else { lhs.get_typ() },
                op, lhs, rhs });
        }
        Ok(lhs)
    }

    fn parse_final_expr(&mut self, lex: &mut Lexer) -> Result<Box<Expr>, Error> {
        let (sloc, tok) = lex.next()?;
        let mut expr = match tok {
            Tok::True => Box::new(Expr::Int { sloc, typ: Type::Bool, num: 1 }),
            Tok::False => Box::new(Expr::Int { sloc, typ: Type::Bool, num: 0 }),
            Tok::BitwiseNot => {
                let val = self.parse_final_expr(lex)?;
                let typ = val.get_typ();
                if !typ.is_numerical() {
                    return Err(Error::Type(sloc, typ, "expected a numerical type"))
                }
                Box::new(Expr::UnaryOp { sloc, typ, op: UnaryOp::BitwiseNot, val })
            },
            Tok::Minus => {
                let val = self.parse_final_expr(lex)?;
                let typ = val.get_typ();
                if !typ.is_numerical() {
                    return Err(Error::Type(sloc, typ, "expected a numerical type"))
                }
                Box::new(Expr::UnaryOp { sloc, typ, op: UnaryOp::Neg, val })
            },
            Tok::Star => {
                let ptr = self.parse_final_expr(lex)?;
                let typ = match ptr.get_typ() {
                    Type::Ptr { ety, .. } => (*ety).clone(),
                    typ => return Err(Error::Type(sloc, typ, "expected a pointer"))
                };
                Box::new(Expr::Deref { sloc, typ, ptr })
            },
            Tok::LParen => match self.parse_type(lex) {
                Ok(typ) => {
                    lex.expect_token(Tok::RParen, "cast expression")?;
                    Box::new(Expr::Cast { sloc, typ, val: self.parse_final_expr(lex)? })
                },
                Err(Error::ExpectedType(sloc, tok)) => {
                    lex.unread(sloc, tok);
                    let res = self.parse_expr(lex)?;
                    lex.expect_token(Tok::RParen, "closing parenthesis")?;
                    res
                },
                Err(e) => return Err(e)
            },
            Tok::IntLit { signed, bits, val } => Box::new(Expr::Int {
                sloc, num: val,
                typ: Type::Int { bits, signed }
            }),
            Tok::Id(name) => match self.lookup(&sloc, name.clone()) {
                Ok(decl) => Box::new(Expr::Id { sloc, typ: decl.ty.clone(), name, decl }),
                Err(e) => return Err(e)
            },
            _ => unimplemented!(),
        };

        loop {
            expr = match lex.peek()?.1 {
                Tok::Dot => {
                    let (sloc, _) = lex.next()?;
                    let field = lex.expect_id("field name")?.1;
                    let (typ, idx) = expr.get_typ().lookup_field(&sloc, field.clone())?;
                    Box::new(Expr::FieldAccess { sloc, typ, obj: expr, field, idx })
                },
                Tok::Arrow => {
                    let (sloc, _) = lex.next()?;
                    let field = lex.expect_id("field name")?.1;
                    let (styp, (typ, idx)) = match expr.get_typ() {
                        Type::Ptr { ety, .. } => (ety.clone(), ety.lookup_field(&sloc, field.clone())?),
                        t => return Err(Error::Type(sloc, t, "expected a pointer to a struct"))
                    };
                    Box::new(Expr::FieldAccess {
                        sloc: sloc.clone(), typ,
                        obj: Box::new(Expr::Deref { sloc, typ: (*styp).clone(), ptr: expr }),
                        field, idx })
                },
                Tok::LBracket => {
                    let (sloc, _) = lex.next()?;
                    let offset = self.parse_expr(lex)?;
                    lex.expect_token(Tok::RBracket, "closing subscript bracket")?;
                    if !offset.get_typ().is_numerical() {
                        return Err(Error::Type(sloc, expr.get_typ(), "expected a numerical offset"))
                    }
                    let typ = match expr.get_typ() {
                        Type::Ptr { ety, .. } => (*ety).clone(),
                        t => return Err(Error::Type(sloc, t, "expected a pointer"))
                    };
                    Box::new(Expr::Deref {
                        sloc: sloc.clone(), typ,
                        ptr: Box::new(Expr::BinOp {
                            sloc, typ: expr.get_typ(),
                            op: BinOp::Add,
                            lhs: expr,
                            rhs: offset
                        })
                    })
                },
                Tok::LParen => {
                    let (sloc, _) = lex.next()?;
                    let mut args: Vec<Expr> = vec![];
                    loop {
                        let arg = self.parse_expr(lex)?;
                        args.push(*arg);
                        if lex.peek()?.1 == Tok::Comma {
                            lex.next()?;
                            continue
                        }
                        lex.expect_token(Tok::RParen, "end of call argument list")?;
                        break
                    }

                    let typ = match expr.get_typ() {
                        Type::Fn { retty, argtys } => {
                            if args.len() != argtys.len() {
                                return Err(Error::Type(sloc, expr.get_typ(), "wrong number of arguments"))
                            }
                            for (a, b) in args.iter().zip(argtys.iter()) {
                                if a.get_typ() != *b {
                                    return Err(Error::Type(sloc, b.clone(), "wrong argument type"))
                                }
                            }
                            (*retty).clone()
                        },
                        other => return Err(Error::Type(sloc, other, "expected a function"))
                    };

                    Box::new(Expr::Call { sloc, typ, func: expr, args })
                },
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_type(&mut self, lex: &mut Lexer) -> Result<Type, Error> {
        let mut ty = match lex.next()? {
            (_, Tok::Void) => Type::Void,
            (_, Tok::Bool) => Type::Bool,
            (_, Tok::Int) => Type::Int { bits: 32, signed: true },
            (_, Tok::Signed) => match lex.peek()?.1 {
                Tok::Char => {
                    lex.next()?;
                    Type::Int { bits: 8, signed: true }
                }
                Tok::Short => {
                    lex.next()?;
                    Type::Int { bits: 16, signed: true }
                }
                Tok::Int => {
                    lex.next()?;
                    Type::Int { bits: 32, signed: true }
                }
                Tok::Long => {
                    lex.next()?;
                    lex.consume_if_next(Tok::Int)?;
                    Type::Int { bits: 64, signed: true }
                }
                _ => Type::Int { bits: 32, signed: false }
            },
            (_, Tok::Unsigned) => match lex.peek()?.1 {
                Tok::Char => {
                    lex.next()?;
                    Type::Int { bits: 8, signed: false }
                }
                Tok::Short => {
                    lex.next()?;
                    Type::Int { bits: 16, signed: false }
                }
                Tok::Int => {
                    lex.next()?;
                    Type::Int { bits: 32, signed: false }
                }
                Tok::Long => {
                    lex.next()?;
                    lex.consume_if_next(Tok::Int)?;
                    Type::Int { bits: 64, signed: false }
                }
                _ => Type::Int { bits: 32, signed: false }
            },
            (_, Tok::Long) => {
                lex.consume_if_next(Tok::Long)?;
                lex.consume_if_next(Tok::Int)?;
                Type::Int { bits: 64, signed: true }
            }
            (_, Tok::Char) => Type::Int { bits: 8, signed: false },
            (_, Tok::Struct) => {
                let name = match lex.peek()? {
                    (_, Tok::Id(name)) => {
                        let res = Some(name.clone());
                        lex.next()?;
                        res
                    },
                    _ => None
                };

                lex.expect_token(Tok::LBraces, "start of struct type def. list")?;
                let mut fields = vec![];
                loop {
                    let (_, tok) = lex.peek()?;
                    if tok == Tok::RBraces {
                        lex.next()?;
                        break;
                    }

                    let fieldty = self.parse_type(lex)?;
                    let fieldname = lex.expect_id("struct field name")?.1;
                    lex.expect_token(Tok::SemiColon, "end of decl.")?;
                    fields.push((fieldname, fieldty));
                }
                Type::Struct { name, fields: Rc::new(fields) }
            }
            (sloc, Tok::Id(id)) => match self.types.get(&id) {
                Some(t) => t.clone(),
                None => return Err(Error::ExpectedType(sloc, Tok::Id(id)))
            },
            (sloc, tok) => return Err(Error::ExpectedType(sloc, tok))
        };
        loop {
            ty = match lex.peek()? {
                (_, Tok::Star) => {
                    lex.next()?;
                    let (volatile, constant, restrict) = (false, false, false);
                    Type::Ptr { ety: Rc::new(ty), volatile, constant, restrict }
                }
                (_, Tok::LBracket) => {
                    lex.next()?;
                    let size = match lex.next()? {
                        (_, Tok::IntLit { signed: _, bits: _, val: n }) => n,
                        (sloc, t) => return Err(Error::UnexpectedTok(
                            sloc, format!("expected array size, found: {:?}", t))),
                    };
                    lex.expect_token(Tok::RBracket, "closing square bracket for array")?;
                    Type::Array(Rc::new(ty), Some(size as usize))
                }
                _ => break,
            };
        }
        Ok(ty)
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches::assert_matches;
    use super::*;

    fn parse_type(input: &str) -> Type {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser::new();
        p.parse_type(&mut lex).unwrap()
    }

    fn parse_func(input: &str) -> Rc<Function> {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser::new();
        p.parse_function(&mut lex).unwrap().unwrap()
    }

    #[test]
    fn foo() {
        let f = parse_func("static int foo(int n) { return n + 42; }");
        assert_eq!(f.is_static, true);
        assert_eq!(&*f.name, "foo");
        assert!(f.args.len() == 1 && &*f.args[0].0 == "n" &&
            f.args[0].1 == Type::Int { bits: 32, signed: true });
        assert_eq!(f.retty, Type::Int { bits: 32, signed: true });
        assert_matches!(f.body.as_ref().unwrap().as_ref(), Stmt::Compound { stmts, .. } if stmts.len() == 1 &&
            matches!(&stmts[0],
                Stmt::Ret { val: Some(x), .. } if
                    matches!(&**x, Expr::BinOp { sloc: _, typ: _, op: BinOp::Add, lhs, rhs } if
                        matches!(&**lhs, Expr::Id { sloc: _, typ: _, name, decl: _ } if &**name == "n") &&
                        matches!(&**rhs, Expr::Int { sloc: _, typ: _, num: 42 }))));
    }

    #[test]
    fn bar() {
        let f = parse_func("static int bar(struct { int x; int y; } *s) { return s->y; }");
        assert_eq!(f.is_static, true);
        assert_eq!(&*f.name, "bar");
        assert_eq!(f.retty, Type::Int { bits: 32, signed: true });
        assert_matches!(f.body.as_ref().unwrap().as_ref(), Stmt::Compound { stmts, .. } if stmts.len() == 1 &&
            matches!(&stmts[0],
                Stmt::Ret { val: Some(x), .. } if
                    matches!(&**x, Expr::FieldAccess { obj, field, idx: 1, .. } if
                        matches!(&**obj, Expr::Deref { ptr, .. } if
                            matches!(&**ptr, Expr::Id { name, .. } if &**name == "s")) &&
                        &**field == "y")));
    }

    #[test]
    fn types() {
        let t1 = parse_type("unsigned long int *[42]");
        assert_eq!(
            t1,
            Type::Array(Rc::new(Type::Ptr {
                ety: Rc::new(Type::Int { bits: 64, signed: false }),
                volatile: false,
                constant: false,
                restrict: false }), Some(42))
        );

        let t2 = parse_type("struct bla { char *foo; int bar; }");
        assert_eq!(
            t2,
            Type::Struct {
                name: Some(Rc::from("bla")),
                fields: Rc::new(vec![
                    (Rc::from("foo"), Type::Ptr {
                        ety: Rc::new(Type::Int { bits: 8, signed: false }),
                        volatile: false, constant: false, restrict: false
                    }),
                    (Rc::from("bar"), Type::Int { bits: 32, signed: true })
                ])
            }
        );
    }
}
