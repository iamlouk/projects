use std::{fmt::Display, rc::Rc};

use crate::{common::*, lex::{Lexer, Tok}};

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Function {
    name: Rc<str>,
    sloc: SLoc,
    retty: Type,
    args: Vec<(Rc<str>, Type)>,
    body: Option<Box<Stmt>>,
    is_static: bool,
    locals: Vec<Rc<LocalDecl>>
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct LocalDecl {
    sloc: SLoc,
    name: Rc<str>,
    ty: Type,
    init: Option<Box<Expr>>
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Stmt {
    NoOp,
    Expr { sloc: SLoc, expr: Box<Expr> },
    Decls { decls: Vec<LocalDecl> },
    Compound { sloc: SLoc, stmts: Vec<Stmt> },
    While { sloc: SLoc, cond: Box<Expr>, body: Box<Stmt> },
    For { sloc: SLoc, init: Box<Stmt>, cond: Box<Expr>, incr: Box<Stmt>, body: Box<Stmt> },
    If { sloc: SLoc, cond: Box<Expr>, then: Box<Stmt>, otherwise: Option<Box<Stmt>> },
    Ret { sloc: SLoc, val: Option<Box<Expr>> },
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOp {
    EQ, NE, LT, LE, GT, GE,
    Add, Sub, Mul, Div, Mod, BitwiseAnd, BitwiseOr, BitwiseXOr,
    LogicalAnd, LogicalOr
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum UnaryOp { Neg, LogicalNot, BitwiseNot }

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Expr {
    Id { sloc: SLoc, typ: Type, name: Rc<str> },
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
        obj: Box<Expr>, field: Rc<str>,
    },
    Subscript {
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

impl Expr {
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
}

pub struct Parser {}

#[allow(dead_code)]
impl Parser {
    fn parse_function(&mut self, lex: &mut Lexer) -> Result<Box<Function>, Error> {
        let is_static = lex.consume_if_next(Tok::Static)?;
        let retty = self.parse_type(lex)?;
        let (sloc, name) = lex.expect_id("function name")?;
        lex.expect_token(Tok::LParen, "start of function parameter list")?;
        let mut args = Vec::new();
        loop {
            if lex.consume_if_next(Tok::RParen)? {
                break;
            }

            let argty = self.parse_type(lex)?;
            let argname = lex.expect_id("parameter name")?.1;
            args.push((argname, argty));
            if !lex.consume_if_next(Tok::Comma)? {
                lex.expect_token(Tok::RParen, "end of parameter list")?;
                break;
            }
        }

        if lex.consume_if_next(Tok::SemiColon)? {
            return Ok(Box::new(Function {
                name, sloc, retty, args,
                body: None, is_static, locals: Vec::new()
            }))
        }

        let body = self.parse_stmt(lex)?;
        Ok(Box::new(Function {
            name, sloc, retty, args,
            body: Some(body), is_static, locals: Vec::new()
        }))
    }

    fn parse_stmt(&mut self, lex: &mut Lexer) -> Result<Box<Stmt>, Error> {
        let (sloc, tok) = lex.peek()?;
        if Tok::SemiColon == tok {
            lex.next()?;
            return Ok(Box::new(Stmt::NoOp))
        }

        if Tok::Return == tok {
            lex.next()?;
            if lex.peek()?.1 == Tok::SemiColon {
                lex.next()?;
                return Ok(Box::new(Stmt::Ret { sloc, val: None }))
            }

            let expr = self.parse_expr(lex)?;
            lex.expect_token(Tok::SemiColon, "end of return statement")?;
            return Ok(Box::new(Stmt::Ret { sloc, val: Some(expr) }))
        }

        if Tok::LBraces == tok {
            lex.next()?;
            let mut stmts = vec![];
            while lex.peek()?.1 != Tok::RBraces {
                stmts.push(*self.parse_stmt(lex)?);
            }
            return Ok(Box::new(Stmt::Compound { sloc, stmts }))
        }

        if Tok::While == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen, "while condition")?;
            let cond = self.parse_expr(lex)?;
            lex.expect_token(Tok::RParen, "while condition")?;
            let body = self.parse_stmt(lex)?;
            return Ok(Box::new(Stmt::While { sloc, cond, body }))
        }

        if Tok::If == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen, "if condition")?;
            let cond = self.parse_expr(lex)?;
            lex.expect_token(Tok::RParen, "if condition")?;
            let then = self.parse_stmt(lex)?;
            let mut otherwise: Option<Box<Stmt>> = None;
            if Tok::Else == lex.peek()?.1 {
                lex.next()?;
                otherwise = Some(self.parse_stmt(lex)?)
            }

            return Ok(Box::new(Stmt::If { sloc, cond, then, otherwise }))
        }

        let ty = match self.parse_type(lex) {
            Ok(t) => t,
            Err(Error::ExpectedType(sloc, tok)) => {
                lex.unread(sloc.clone(), tok);
                let expr = self.parse_expr(lex)?;
                lex.expect_token(Tok::SemiColon, "expression statement")?;
                return Ok(Box::new(Stmt::Expr { sloc, expr }))
            }
            Err(e) => return Err(e)
        };

        let mut decls: Vec<LocalDecl> = Vec::new();
        loop {
            let (sloc, name) = lex.expect_id("local declaration name")?;
            let init = if lex.consume_if_next(Tok::Assign)? {
                Some(self.parse_expr(lex)?)
            } else {
                None
            };
            decls.push(LocalDecl { sloc, name, ty: ty.clone(), init });
            if lex.consume_if_next(Tok::Comma)? {
                continue;
            }

            lex.expect_token(Tok::SemiColon, "end of declarations")?;
            break;
        }

        Ok(Box::new(Stmt::Decls { decls }))
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
            lex.next()?;
            let rhs = self.parse_expr(lex)?;
            return Ok(Box::new(Expr::Assign {
                sloc, typ: Type::Unknown, op,
                lhs: expr, rhs }))
        }

        if lex.consume_if_next(Tok::QuestionMark)? {
            let sloc = lex.peek()?.0;
            let then = self.parse_expr(lex)?;
            lex.expect_token(Tok::Colon, "tenary expression")?;
            let otherwise = self.parse_expr(lex)?;
            return Ok(Box::new(Expr::Tenary {
                sloc, typ: Type::Unknown, cond: expr, then, otherwise }))
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
            lhs = Box::new(Expr::BinOp {
                sloc, typ: Type::Unknown, op, lhs, rhs });
        }
        Ok(lhs)
    }

    fn parse_final_expr(&mut self, lex: &mut Lexer) -> Result<Box<Expr>, Error> {
        let (sloc, tok) = lex.next()?;
        let mut expr = match tok {
            Tok::Minus => Box::new(Expr::UnaryOp {
                sloc, typ: Type::Unknown, op: UnaryOp::Neg,
                val: self.parse_final_expr(lex)? }),
            Tok::LogicalNot => Box::new(Expr::UnaryOp {
                sloc, typ: Type::Unknown, op: UnaryOp::LogicalNot,
                val: self.parse_final_expr(lex)? }),
            Tok::BitwiseNot => Box::new(Expr::UnaryOp {
                sloc, typ: Type::Unknown, op: UnaryOp::BitwiseNot,
                val: self.parse_final_expr(lex)? }),
            Tok::Star => Box::new(Expr::Deref {
                sloc, typ: Type::Unknown,
                ptr: self.parse_final_expr(lex)? }),
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
            Tok::IntLit(n) => Box::new(Expr::Int {
                sloc, num: n,
                typ: Type::Int { bits: 64, signed: true }
            }),
            Tok::Id(name) => Box::new(Expr::Id {
                sloc, typ: Type::Unknown, name }),
            _ => unimplemented!(),
        };

        loop {
            expr = match lex.peek()?.1 {
                Tok::Dot => {
                    let (sloc, _) = lex.next()?;
                    let field = lex.expect_id("field name")?.1;
                    Box::new(Expr::FieldAccess {
                        sloc, typ: Type::Unknown, obj: expr, field })
                },
                Tok::Arrow => {
                    let (sloc, _) = lex.next()?;
                    let field = lex.expect_id("field name")?.1;
                    Box::new(Expr::FieldAccess {
                        sloc: sloc.clone(),
                        typ: Type::Unknown,
                        obj: Box::new(Expr::Deref { sloc, typ: Type::Unknown, ptr: expr }),
                        field
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

                    Box::new(Expr::Call {
                        sloc, typ: Type::Unknown,
                        func: expr, args,
                    })
                },
                Tok::LBracket => {
                    let (sloc, _) = lex.next()?;
                    let offset = self.parse_expr(lex)?;
                    lex.expect_token(Tok::RBracket, "closing subscript bracket")?;
                    Box::new(Expr::Subscript {
                        sloc, typ: Type::Unknown, ptr: expr, offset })
                },
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_type(&mut self, lex: &mut Lexer) -> Result<Type, Error> {
        let mut ty = match lex.next()? {
            (_, Tok::Void) => Type::Void,
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
                lex.expect_token(Tok::LBraces, "end of struct type def. list")?;
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
                        (_, Tok::IntLit(n)) => n,
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
        let mut p = Parser {};
        p.parse_type(&mut lex).unwrap()
    }

    fn parse_func(input: &str) -> Box<Function> {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser {};
        p.parse_function(&mut lex).unwrap()
    }

    fn parse_expr(input: &str) -> Box<Expr> {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser {};
        p.parse_expr(&mut lex).unwrap()
    }

    #[test]
    fn some_final_exprs() {
        assert_eq!(
            parse_expr("-*hallo[42]->foo").to_string(),
            "-(*((*((hallo)[0x2a])).foo))");

        assert_eq!(
            parse_expr("*(int*)foo").to_string(),
            "*((int32_t*)(foo))");

        assert_eq!(
            parse_expr("foo | a + b * c + d & bar").to_string(),
            "((foo) | (((a) + ((b) * (c))) + (d))) & (bar)");
    }

    #[test]
    fn foo() {
        let f = parse_func("static signed long foo(unsigned n) { return bar(n); }");
        assert_eq!(f.is_static, true);
        assert_eq!(&*f.name, "foo");
        assert!(f.args.len() == 1 && &*f.args[0].0 == "n" &&
            f.args[0].1 == Type::Int { bits: 32, signed: false });
        assert_eq!(f.retty, Type::Int { bits: 64, signed: true });
        assert_matches!(*f.body.unwrap(), Stmt::Compound { sloc: _, stmts } if stmts.len() == 1 &&
            matches!(&stmts[0],
                Stmt::Ret { sloc: _, val: Some(x) } if
                    matches!(&**x, Expr::Call { sloc: _, typ: Type::Unknown, func, args } if
                        matches!(&**func, Expr::Id { sloc: _, typ: _, name } if &**name == "bar") &&
                        args.len() == 1 &&
                        matches!(&args[0], Expr::Id { sloc: _, typ: _, name } if &**name == "n"))));
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
