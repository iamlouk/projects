use std::rc::Rc;

use crate::{common::*, lex::{Lexer, Tok}};

#[allow(dead_code)]
pub struct Function {
    sloc: SLoc,
    fty: Type,
    retty: Type,
    args: Vec<(Rc<str>, Type)>,
    body: Stmt,
    is_static: bool
}

pub struct LocalDecl {
    sloc: SLoc,
    name: Rc<str>,
    ty: Type,
    init: Option<Box<Expr>>
}

#[allow(dead_code)]
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
pub enum Predicate { EQ, NE, LT, LE, GT, GE }

#[allow(dead_code)]
pub enum BinOp { Add, Sub, Mul, Div, BitwiseAnd, BitwiseOr, BitwiseXOr }

#[allow(dead_code)]
pub enum Expr {
    Id { sloc: SLoc, typ: Type, name: Rc<str> },
    Int { sloc: SLoc, typ: Type, num: i64 },
    Cmp {
        sloc: SLoc, typ: Type,
        pred: Predicate,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    BinOp {
        sloc: SLoc, typ: Type,
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        sloc: SLoc, typ: Type,
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Deref {
        sloc: SLoc, typ: Type,
        ptr: Box<Expr>,
    },
    FieldAccess {
        sloc: SLoc, typ: Type,
        obj: Box<Expr>,
        field: Rc<str>,
    },
}

pub struct Parser {}

#[allow(dead_code)]
impl Parser {
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
            lex.expect_token(Tok::SemiColon)?;
            return Ok(Box::new(Stmt::Ret { sloc, val: Some(expr) }))
        }

        if Tok::LBraces == tok {
            lex.next()?;
            let mut stmts = vec![];
            while lex.peek()?.1 != Tok::RBraces {
                stmts.push(*self.parse_stmt(lex)?);
                lex.expect_token(Tok::SemiColon)?
            }

            return Ok(Box::new(Stmt::Compound { sloc, stmts }))
        }

        if Tok::While == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen)?;
            let cond = self.parse_expr(lex)?;
            lex.expect_token(Tok::RParen)?;
            let body = self.parse_stmt(lex)?;
            return Ok(Box::new(Stmt::While { sloc, cond, body }))
        }

        if Tok::If == tok {
            lex.next()?;
            lex.expect_token(Tok::LParen)?;
            let cond = self.parse_expr(lex)?;
            lex.expect_token(Tok::RParen)?;
            let then = self.parse_stmt(lex)?;
            let mut otherwise: Option<Box<Stmt>> = None;
            if Tok::Else == lex.peek()?.1 {
                lex.next()?;
                otherwise = Some(self.parse_stmt(lex)?)
            }

            return Ok(Box::new(Stmt::If { sloc, cond, then, otherwise }))
        }

        // FIXME: Parse local decls!

        let expr = self.parse_expr(lex)?;
        Ok(Box::new(Stmt::Expr { sloc, expr }))
    }

    fn parse_expr(&mut self, lex: &mut Lexer) -> Result<Box<Expr>, Error> {
        unimplemented!()
    }

    fn parse_final_expr(&mut self, lex: &mut Lexer) -> Result<Box<Expr>, Error> {
        let (sloc, tok) = lex.next()?;
        let mut expr = match tok {
            Tok::LParen => {
                let res = self.parse_expr(lex)?;
                lex.expect_token(Tok::RParen)?;
                res
            }
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
                    let field = lex.expect_id()?;
                    Box::new(Expr::FieldAccess {
                        sloc, typ: Type::Unknown, obj: expr, field })
                }
                Tok::Arrow => {
                    let (sloc, _) = lex.next()?;
                    let field = lex.expect_id()?;
                    Box::new(Expr::FieldAccess {
                        sloc: sloc.clone(),
                        typ: Type::Unknown,
                        obj: Box::new(Expr::Deref { sloc, typ: Type::Unknown, ptr: expr }),
                        field
                    })
                }
                Tok::LParen => {
                    let mut args: Vec<Expr> = vec![];
                    let (sloc, _) = lex.next()?;
                    loop {
                        let arg = self.parse_expr(lex)?;
                        args.push(*arg);
                        if lex.peek()?.1 == Tok::Comma {
                            lex.next()?;
                            continue
                        }
                        lex.expect_token(Tok::RParen)?;
                        break
                    }

                    Box::new(Expr::Call {
                        sloc, typ: Type::Unknown,
                        func: expr, args,
                    })
                }
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
                lex.expect_token(Tok::LBraces)?;
                let mut fields = vec![];
                loop {
                    let (_, tok) = lex.peek()?;
                    if tok == Tok::RBraces {
                        lex.next()?;
                        break;
                    }

                    let fieldty = self.parse_type(lex)?;
                    let fieldname = lex.expect_id()?;
                    lex.expect_token(Tok::SemiColon)?;
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
                            sloc, t, Tok::IntLit(42), "expected a size as integer lit.")),
                    };
                    lex.expect_token(Tok::RBracket)?;
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
    use super::*;

    fn parse_type(input: &str) -> Type {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser {};
        p.parse_type(&mut lex).unwrap()
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
