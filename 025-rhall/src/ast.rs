use std::fmt::Write;
use std::{rc::Rc, cell::RefCell};

use crate::lex::{Lexer, Tok};
use crate::core::{Error, SLoc, Type};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOp { Add, Sub, Mul, Div, And, Or, EQ, NE, LT, LE, GT, GE }

pub type NodeRef = Rc<RefCell<Node>>;
#[derive(Debug, Clone)]
pub enum Node {
    Id         { sloc: SLoc, typ: Type, name: Rc<str> },
    Integer    { sloc: SLoc, typ: Type, value: i64 },
    Boolean    { sloc: SLoc, typ: Type, value: bool },
    String     { sloc: SLoc, typ: Type, value: Rc<str> },
    Invert     { sloc: SLoc, typ: Type, op0: NodeRef },
    BinOp      { sloc: SLoc, typ: Type, op: BinOp, lhs: NodeRef, rhs: NodeRef, iscmp: bool },
    Call       { sloc: SLoc, typ: Type, callable: NodeRef, args: Vec<NodeRef> },
    IfThenElse { sloc: SLoc, typ: Type, op0: NodeRef, op1: NodeRef, op2: NodeRef },
    LetIn {
        sloc: SLoc,
        typ: Type,
        name: Rc<str>,
        value: NodeRef,
        body: NodeRef,
    },
    Lambda {
        sloc: SLoc,
        typ: Type,
        args: Vec<(Rc<str>, Type)>,
        body: NodeRef
    }
}

impl Node {
    pub fn get_type(&self) -> &Type {
        match self {
            Node::Id { typ, .. } => typ,
            Node::Integer { typ, .. } => typ,
            Node::Boolean { typ, .. } => typ,
            Node::String { typ, .. } => typ,
            Node::Invert { typ, .. } => typ,
            Node::BinOp { typ, .. } => typ,
            Node::Call { typ, .. } => typ,
            Node::IfThenElse { typ, .. } => typ,
            Node::LetIn { typ, .. } => typ,
            Node::Lambda { typ, .. } => typ
        }
    }

    pub fn stringify(&self, ident: &str, buf: &mut String) -> Result<(), std::fmt::Error> {
        match self {
            Node::Id { name, .. } => buf.write_str(name),
            Node::Integer { value, .. } => write!(buf, "{:#x}", value),
            Node::Boolean { value, .. } => buf.write_str(if *value { "true" } else { "false" }),
            Node::String { value, .. } => write!(buf, "{:?}", value.as_ref()),
            Node::Invert { op0, .. } => {
                buf.write_str("~")?;
                op0.as_ref().borrow().stringify(ident, buf)?;
                buf.write_str("")
            },
            Node::BinOp { op, lhs, rhs, .. } => {
                buf.write_char('(')?;
                lhs.as_ref().borrow().stringify(ident, buf)?;
                buf.write_str(match op {
                    BinOp::Add => ") + (", BinOp::Sub => ") - (",
                    BinOp::Mul => ") * (", BinOp::Div => ") / (",
                    BinOp::And => ") & (", BinOp::Or  => ") | (",
                    BinOp::EQ  => ") == (", BinOp::NE => ") != (",
                    BinOp::LT  => ") < (", BinOp::LE => ") <= (",
                    BinOp::GT  => ") > (", BinOp::GE => ") >= ("
                })?;
                rhs.as_ref().borrow().stringify(ident, buf)?;
                buf.write_char(')')
            },
            Node::Call { callable, args, .. } => {
                callable.as_ref().borrow().stringify(ident, buf)?;
                buf.write_char('(')?;
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 { buf.write_str(", ")?; }
                    arg.as_ref().borrow().stringify(ident, buf)?;
                }
                buf.write_char(')')
            },
            Node::IfThenElse { op0, op1, op2, .. } => {
                buf.write_str("if (")?;
                op0.as_ref().borrow().stringify(ident, buf)?;
                buf.write_str(")\n")?;
                buf.write_str(ident)?;
                buf.write_str("then (")?;
                let newident = ident.to_owned() + "\t";
                op1.as_ref().borrow().stringify(newident.as_str(), buf)?;
                buf.write_str(")\n")?;
                buf.write_str(ident)?;
                buf.write_str("else (")?;
                op2.as_ref().borrow().stringify(newident.as_str(), buf)?;
                buf.write_str(")")
            },
            Node::LetIn { name, value, body, .. } => {
                buf.write_str("let ")?;
                buf.write_str(name.as_ref())?;
                buf.write_str(" = ")?;
                let newident = ident.to_owned() + "\t";
                value.as_ref().borrow().stringify(newident.as_str(), buf)?;
                buf.write_str("\n")?;
                buf.write_str(ident)?;
                buf.write_str("in ")?;
                body.as_ref().borrow().stringify(newident.as_str(), buf)
            },
            Node::Lambda { args, body, .. } => {
                buf.write_str("λ(")?;
                for (i, (name, typ)) in args.iter().enumerate() {
                    if i != 0 { buf.write_str(", ")?; }
                    write!(buf, "{}: {}", name.as_ref(), typ)?;
                }
                buf.write_str(") ->\n")?;
                let newident = ident.to_owned() + "\t";
                buf.write_str(newident.as_str())?;
                body.as_ref().borrow().stringify(newident.as_str(), buf)
            },
        }
    }
}

pub struct Parser<'a> {
    lexer: &'a mut Lexer<'a>,
    consumed_sloc: SLoc,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: &'a mut Lexer<'a>) -> Parser {
        Parser { lexer, consumed_sloc: SLoc { line: 0, col: 0, file_id: 0 } }
    }

    fn expect_token(&mut self, expected: Tok) -> Result<SLoc, Error> {
        let (sloc, tok) = self.lexer.next().ok_or(Error::UnexpectedEOF)??;
        if tok == expected {
            Ok(sloc)
        } else {
            Err(Error::ExpectedToken { sloc, expected, found: tok })
        }
    }

    fn consume_if(&mut self, tok: Tok) -> bool {
        if let Some(Ok((sloc, t))) = self.lexer.peek() {
            self.consumed_sloc = sloc;
            if t == tok {
                self.lexer.next();
                return true
            }
        }
        false
    }

    fn binop_precedence(binop: BinOp) -> usize {
        match binop {
            BinOp::Mul | BinOp::Div => 100,
            BinOp::Add | BinOp::Sub => 90,
            BinOp::EQ | BinOp::NE | BinOp::LT | BinOp::LE | BinOp::GT | BinOp::GE => 80,
            BinOp::And => 70,
            BinOp::Or => 60
        }
    }

    pub fn parse_all(&mut self) -> Result<NodeRef, Error> {
        let node = self.parse()?;
        match self.lexer.next() {
            Some(Ok((sloc, tok))) => Err(Error::Parser(sloc, format!("EOF expected, found: {:?}", tok))),
            Some(Err(e)) => Err(e),
            None => Ok(node)
        }
    }

    fn parse(&mut self) -> Result<NodeRef, Error> {
        let (sloc, tok) = self.lexer.peek().ok_or(Error::UnexpectedEOF)??;
        if Tok::If == tok {
            self.lexer.next();
            let op0 = self.parse_expr1()?;
            self.expect_token(Tok::Then)?;
            let op1 = self.parse_expr1()?;
            self.expect_token(Tok::Else)?;
            let op2 = self.parse_expr1()?;
            return Ok(Rc::new(RefCell::new(Node::IfThenElse {
                sloc, typ: Type::Unresolved(None), op0, op1, op2
            })));
        }

        if tok == Tok::Let {
            self.lexer.next();
            let name = match self.lexer.next().ok_or(Error::UnexpectedEOF)?? {
                (_, Tok::Id(name)) => name,
                (sloc, found) => return Err(Error::ExpectedToken {
                    sloc, expected: Tok::Id(Rc::from("<whatever>")), found })
            };

            self.expect_token(Tok::Assign)?;
            let value = self.parse_expr1()?;

            let body = if let Some(Ok((_, Tok::Let))) = self.lexer.peek() {
                self.parse()?
            } else {
                self.expect_token(Tok::In)?;
                self.parse()?
            };
            return Ok(Rc::new(RefCell::new(Node::LetIn {
                sloc, name, typ: Type::Unresolved(None), value, body
            })));
        }

        self.parse_expr1()
    }

    fn parse_expr1(&mut self) -> Result<Rc<RefCell<Node>>, Error> {
        let mut lhs = self.parse_expr0()?;
        loop {
            if self.consume_if(Tok::Plus) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::Add, lhs, rhs, iscmp: false
                }))
            } else if self.consume_if(Tok::Minus) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::Sub, lhs, rhs, iscmp: false
                }))
            } else if self.consume_if(Tok::Star) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::Mul, lhs, rhs, iscmp: false
                }))
            } else if self.consume_if(Tok::Slash) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::Div, lhs, rhs, iscmp: false
                }))
            } else if self.consume_if(Tok::Ampersand) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::And, lhs, rhs, iscmp: false
                }))
            } else if self.consume_if(Tok::Pipe) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::Or, lhs, rhs, iscmp: false
                }))
            } else if self.consume_if(Tok::Equal) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::EQ, lhs, rhs, iscmp: true
                }))
            } else if self.consume_if(Tok::Lower) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::LT, lhs, rhs, iscmp: true
                }))
            } else if self.consume_if(Tok::LowerOrEqual) {
                let rhs = self.parse_expr0()?;
                lhs = Rc::new(RefCell::new(Node::BinOp {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), op: BinOp::LE, lhs, rhs, iscmp: true
                }))
            } else {
                break
            }
        }

        Ok(lhs)
    }

    fn parse_expr0(&mut self) -> Result<Rc<RefCell<Node>>, Error> {
        let mut expr = self.parse_final()?;
        loop {
            if self.consume_if(Tok::LParen) {
                let mut args = vec![];
                loop {
                    args.push(self.parse()?);
                    if self.consume_if(Tok::Comma) {
                        continue
                    }

                    if self.consume_if(Tok::RParen) {
                        break
                    }

                    return Err(Error::Parser(self.consumed_sloc, "Expected ',' or ')'".to_string()))
                }

                expr = Rc::new(RefCell::new(Node::Call {
                    sloc: self.consumed_sloc, typ: Type::Unresolved(None), callable: expr, args }));
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_final(&mut self) -> Result<NodeRef, Error> {
        let (sloc, tok) = self.lexer.next().ok_or(Error::UnexpectedEOF)??;
        Ok(Rc::new(RefCell::new(match tok {
            Tok::Id(name) => Node::Id { sloc, typ: Type::Unresolved(None), name },
            Tok::Boolean(value) => Node::Boolean { sloc, typ: Type::Boolean, value },
            Tok::Integer(value) => Node::Integer { sloc, typ: Type::Integer, value },
            Tok::String(value) => Node::String { sloc, typ: Type::String, value },
            Tok::Tilde => Node::Invert { sloc, typ: Type::Unresolved(None), op0: self.parse_final()? },
            Tok::LParen => {
                let expr = self.parse()?;
                self.expect_token(Tok::RParen)?;
                return Ok(expr)
            },
            Tok::Lambda => {
                self.expect_token(Tok::LParen)?;
                let mut args = vec![];
                loop {
                    let name = match self.lexer.next().ok_or(Error::UnexpectedEOF)?? {
                        (_, Tok::Id(name)) => name,
                        (sloc, t) => return Err(Error::ExpectedToken {
                            sloc, expected: Tok::Id(Rc::from("<whatever>")), found: t }),
                    };

                    self.expect_token(Tok::Colon)?;
                    let typannot = self.parse()?;
                    args.push((name, Type::Unresolved(Some(typannot))));

                    if self.consume_if(Tok::RParen) {
                        break;
                    }
                    self.expect_token(Tok::Comma)?;
                }
                self.expect_token(Tok::Arrow)?;
                let body = self.parse()?;
                Node::Lambda { sloc, typ: Type::Unresolved(None), args, body }
            },
            _ => unimplemented!()
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &'static str,
             spool: &mut std::collections::HashSet<Rc<str>>) -> Result<Rc<RefCell<Node>>, Error> {
        let mut lexer = Lexer::new(input, 0, spool);
        let mut parser = Parser::new(&mut lexer);
        parser.parse()
    }

    fn clone_node(node: &NodeRef) -> Node { (&*node.borrow()).clone() }

    #[test]
    fn lambda() {
        let mut spool = std::collections::HashSet::new();
        let node = parse("let inc = λ(x: Int) -> x + 1 in inc(41)", &mut spool).unwrap();

        assert!(match clone_node(&node) {
            Node::LetIn { name, value, body, .. } if name.as_ref() == "inc" => (
                match clone_node(&value) {
                    Node::Lambda { args, body, .. } if args.len() == 1 => (
                        match &args[0] {
                            (name, typ) if name.as_ref() == "x" => match typ {
                                Type::Unresolved(Some(id)) => match &*id.borrow() {
                                    Node::Id { name, .. } if name.as_ref() == "Int" => true,
                                    _ => false
                                },
                                _ => false,
                            },
                            _ => false
                        } &&
                        match clone_node(&body) {
                            Node::BinOp { op: BinOp::Add, lhs, rhs, iscmp: false, .. } => (
                                match clone_node(&lhs) {
                                    Node::Id { name, .. } if name.as_ref() == "x" => true,
                                    _ => false
                                } &&
                                match clone_node(&rhs) {
                                    Node::Integer { value, .. } if value == 1 => true,
                                    _ => false
                                }),
                            _ => false
                        }),
                    _ => false
                } &&
                match clone_node(&body) {
                    Node::Call { callable, args, .. } if args.len() == 1 => (
                        match clone_node(&callable) {
                            Node::Id { name, .. } if name.as_ref() == "inc" => true,
                            _ => false
                        } &&
                        match clone_node(&args[0]) {
                            Node::Integer { value, .. } if value == 41 => true,
                            _ => false
                        }),
                    _ => false
            }),
            _ => false
        });
    }
}

