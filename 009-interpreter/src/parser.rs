use crate::ast::{Node, Type, Metadata, BinOp};
use crate::lexer::{Lexer, Pos, Tok, NULLPOS};

use std::rc::Rc;
use std::iter::Peekable;
use std::collections::HashMap;

pub struct Parser<'input> {
    lexer: Peekable<Lexer<'input>>,
    string_pool: HashMap<&'input str, Rc<String>>,
}

#[derive(Debug)]
pub enum Error<'input> {
    Message(&'static str, Pos),
    UnexpectedToken(Tok<'input>, Tok<'input>),
    UnexpectedEOF
}

impl<'input> Parser<'input> {
    pub fn new(lexer: Lexer<'input>) -> Self {
        Self {
            lexer: lexer.peekable(),
            string_pool: HashMap::new()
        }
    }

    pub fn parse(&mut self) -> Result<Node, Error<'input>> {
        let node = self.parse_expr()?;
        if !self.lexer.next().is_none() {
            return Err(Error::Message("expected EOF, found trailing tokens", (0, 0, 0)))
        }
        Ok(node)
    }

    fn stringify(&mut self, str: &'input str) -> Rc<String> {
        if let Some(res) = self.string_pool.get(&str) {
            return res.clone();
        } else {
            let res = Rc::new(String::from(str));
            self.string_pool.insert(str, res.clone());
            return res
        }
    }

    fn operator_precedence(tok: &Tok<'input>) -> Option<(Pos, BinOp, i32)> {
        match tok {
            Tok::Or(pos)       => Some((*pos, BinOp::Or, 10)),
            Tok::And(pos)      => Some((*pos, BinOp::And, 15)),
            Tok::Equal(pos)    => Some((*pos, BinOp::Eq, 20)),
            Tok::NotEqual(pos) => Some((*pos, BinOp::NotEq, 20)),
            Tok::Lower(pos)    => Some((*pos, BinOp::Lt, 20)),
            Tok::Greater(pos)  => Some((*pos, BinOp::Gt, 20)),
            Tok::LowerOrEqual(pos)   => Some((*pos, BinOp::LtEq, 20)),
            Tok::GreaterOrEqual(pos) => Some((*pos, BinOp::GtEq, 20)),
            Tok::Plus(pos)  => Some((*pos, BinOp::Add, 30)),
            Tok::Minus(pos) => Some((*pos, BinOp::Sub, 30)),
            Tok::Star(pos)  => Some((*pos, BinOp::Mul, 50)),
            Tok::Div(pos)   => Some((*pos, BinOp::Div, 50)),
            _ => None
        }
    }

    #[inline]
    fn expect(&mut self, expected: Tok<'input>) -> Result<(), Error<'input>> {
        let tok = match self.lexer.next() {
            Some(tok) => tok,
            None => return Err(Error::UnexpectedEOF)
        };
        match (tok, expected) {
            (Tok::LeftParen(_), Tok::LeftParen(_)) => Ok(()),
            (Tok::RightParen(_), Tok::RightParen(_)) => Ok(()),
            (Tok::RightCurly(_), Tok::RightCurly(_)) => Ok(()),
            (Tok::RightSquare(_), Tok::RightSquare(_)) => Ok(()),
            (Tok::Colon(_), Tok::Colon(_)) => Ok(()),
            (Tok::Comma(_), Tok::Comma(_)) => Ok(()),
            (Tok::In(_), Tok::In(_)) => Ok(()),
            (Tok::Then(_), Tok::Then(_)) => Ok(()),
            (Tok::Else(_), Tok::Else(_)) => Ok(()),
            (Tok::Assign(_), Tok::Assign(_)) => Ok(()),
            (Tok::ThinArrow(_), Tok::ThinArrow(_)) => Ok(()),
            (tok, expected) => Err(Error::UnexpectedToken(tok, expected))
        }
    }

    /// Parse let and if...
    pub fn parse_expr(&mut self) -> Result<Node, Error<'input>> {
        match self.lexer.peek() {
            Some(&Tok::Let(pos)) => {
                self.lexer.next();
                if let Some(Tok::Id(_, id)) = self.lexer.next() {
                    self.expect(Tok::Assign(NULLPOS))?;
                    let named = self.parse_expr_lvl1(0)?;
                    self.expect(Tok::In(NULLPOS))?;
                    let body = self.parse_expr()?;
                    Ok(Node::LetIn(
                            Metadata{ pos, ttype: Type::Unkown },
                            self.stringify(id), Box::new(named), Box::new(body)))
                } else {
                    Err(Error::UnexpectedToken(Tok::Nil, Tok::Id(NULLPOS, "<name-of-let>")))
                }
            },
            Some(&Tok::If(pos)) => {
                self.lexer.next();
                let cond = self.parse_expr_lvl1(0)?;
                self.expect(Tok::Then(NULLPOS))?;
                let iftrue = self.parse_expr_lvl1(0)?;
                self.expect(Tok::Else(NULLPOS))?;
                let iffalse = self.parse_expr_lvl1(0)?;
                Ok(Node::If(
                        Metadata{ pos, ttype: Type::Unkown },
                        Box::new(cond), Box::new(iftrue), Box::new(iffalse)))
            },
            _ => self.parse_expr_lvl1(0)
        }
    }

    /// Parse binary operators...
    fn parse_expr_lvl1(&mut self, precedance: i32) -> Result<Node, Error<'input>> {
        let mut node = self.parse_expr_lvl2()?;
        loop {
            let peeked = match self.lexer.peek() {
                Some(tok) => tok,
                None => break
            };

            if let Some((pos, op, prec)) = Self::operator_precedence(peeked) {
                if prec <= precedance {
                    break
                }

                self.lexer.next();
                let operand = self.parse_expr_lvl1(prec)?;
                node = Node::BinOp(Metadata { pos, ttype: Type::Unkown }, op,
                    Box::new(node), Box::new(operand));
            } else {
                break
            }
        }
        Ok(node)
    }

    /// Parse last level expressions (ids, literals, lambdas and calls)
    fn parse_expr_lvl2(&mut self) -> Result<Node, Error<'input>> {
        let mut node = match self.lexer.next() {
            Some(Tok::LeftParen(_)) => {
                let expr = self.parse_expr()?;
                self.expect(Tok::RightParen(NULLPOS))?;
                expr
            },
            Some(Tok::Lambda(pos)) => {
                self.expect(Tok::LeftParen(NULLPOS))?;
                let mut args = Vec::new();
                loop {
                    if let Some(&Tok::RightParen(_)) = self.lexer.peek() {
                        self.lexer.next();
                        break;
                    }

                    if args.len() != 0 {
                        self.expect(Tok::Comma(NULLPOS))?;
                    }

                    let id = match self.lexer.next() {
                        Some(Tok::Id(_, id)) => id,
                        Some(_) => { return Err(Error::Message(
                                "expected id in lambda expression", pos)); },
                        None => { return Err(Error::UnexpectedEOF); }
                    };

                    self.expect(Tok::Colon(NULLPOS))?;
                    let ttype = self.parse_expr()?;
                    args.push((
                        self.stringify(id),
                        Type::Unresolved(Rc::new(ttype))));
                }

                self.expect(Tok::ThinArrow(NULLPOS))?;
                let body = self.parse_expr()?;
                Node::Lambda(Metadata { pos, ttype: Type::Unkown }, args, Rc::new(body))
            },
            Some(Tok::Int(pos, x))   => Node::Int(Metadata{ pos, ttype: Type::Int }, x),
            Some(Tok::Real(pos, x))  => Node::Real(Metadata{ pos, ttype: Type::Real }, x),
            Some(Tok::Bool(pos, x))  => Node::Bool(Metadata{ pos, ttype: Type::Bool }, x),
            Some(Tok::Str(pos, str)) => Node::Str(
                Metadata{ pos, ttype: Type::Str }, self.stringify(str)),
            Some(Tok::Id(pos, str))  => Node::Id(
                Metadata{ pos, ttype: Type::Unkown }, self.stringify(str)),
            _ => unimplemented!()
        };

        loop {
            // A call:
            if let Some(Tok::LeftParen(pos)) = self.lexer.peek() {
                let pos = *pos;
                self.lexer.next();
                let mut args = Vec::new();
                loop {
                    if let Some(&Tok::RightParen(_)) = self.lexer.peek() {
                        self.lexer.next();
                        break;
                    }

                    if args.len() != 0 {
                        self.expect(Tok::Comma(NULLPOS))?;
                    }

                    args.push(self.parse_expr()?);
                }

                node = Node::Call(Metadata { pos, ttype: Type::Unkown },
                    Box::new(node), args);
            } else {
                break;
            }
        }

        Ok(node)
    }
}

