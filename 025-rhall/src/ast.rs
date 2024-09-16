use std::fmt;
use std::{cell::RefCell, rc::Rc};

use crate::core::{Error, SLoc, Type, Value};
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

pub type NodeRef = Rc<RefCell<Node>>;
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
    Invert {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: NodeRef,
    },
    BinOp {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op: BinOp,
        lhs: NodeRef,
        rhs: NodeRef,
        iscmp: bool,
    },
    Call {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        callable: NodeRef,
        args: Vec<NodeRef>,
    },
    IfThenElse {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        op0: NodeRef,
        op1: NodeRef,
        op2: NodeRef,
    },
    LetIn {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        name: Rc<str>,
        value: NodeRef,
        body: NodeRef,
    },
    Lambda {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        args: Vec<(Rc<str>, Option<Rc<Type>>, NodeRef)>,
        body: NodeRef, // <- Technically/Theoretically, this should be the only
                       // field where a Rc<RefCell<Node>> is really needed.
                       // All other AST-children should also be able to just
                       // be Box<Node>.
    },
    Forall {
        sloc: SLoc,
        typ: Option<Rc<Type>>,
        argtypes: Vec<(Rc<str>, Option<Rc<Type>>, NodeRef)>,
        rettyp: NodeRef // <- Well, here as well...!
    }
}

impl Node {
    #[allow(unused)]
    pub fn get_type(&self) -> Rc<Type> {
        let typ = match self {
            Node::Id { typ, .. } => typ,
            Node::Integer { typ, .. } => typ,
            Node::Boolean { typ, .. } => typ,
            Node::String { typ, .. } => typ,
            Node::Invert { typ, .. } => typ,
            Node::BinOp { typ, .. } => typ,
            Node::Call { typ, .. } => typ,
            Node::IfThenElse { typ, .. } => typ,
            Node::LetIn { typ, .. } => typ,
            Node::Lambda { typ, .. } => typ,
            Node::Forall { typ, .. } => typ,
        };
        typ.clone().unwrap()
    }

    pub fn typecheck(&mut self, env: &mut Env) -> Result<Rc<Type>, Error> {
        match self {
            Node::Id { typ: Some(t), .. } => Ok(t.clone()),
            Node::Id { sloc, typ, name } => {
                let t = match env.lookup(name).ok_or(Error::ExpectedType(*sloc))? {
                    Value::Type(_, t) => t,
                    _ => panic!()
                };
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::Integer { typ: Some(t), .. } => Ok(t.clone()),
            Node::Integer { typ, .. } => {
                let t = env.int_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::Boolean { typ: Some(t), .. } => Ok(t.clone()),
            Node::Boolean { typ, .. } => {
                let t = env.bool_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::String { typ: Some(t), .. } => Ok(t.clone()),
            Node::String { typ, .. } => {
                let t = env.str_type.clone();
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::Invert { typ: Some(t), .. } => Ok(t.clone()),
            Node::Invert { sloc, typ, op0 } => {
                let t = op0.borrow_mut().typecheck(env)?;
                if *t.as_ref() != Type::Boolean && *t.as_ref() != Type::Integer {
                    return Err(Error::TypeError(*sloc,
                            format!("invalid type for '~' operator: {}", t)))
                }
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::BinOp { typ: Some(t), .. } => Ok(t.clone()),
            Node::BinOp { sloc, typ, op, lhs, rhs, iscmp } => {
                let lhsty = lhs.borrow_mut().typecheck(env)?;
                let rhsty = rhs.borrow_mut().typecheck(env)?;
                if *lhsty != *rhsty {
                    return Err(Error::TypeError(*sloc,
                            format!("binary operator with different operand types: {} and {}",
                                lhsty.as_ref(), rhsty.as_ref())))
                }

                let t = match (op, iscmp, lhsty.as_ref()) {
                    (_, true, Type::Integer) => env.bool_type.clone(),
                    (BinOp::And | BinOp::Or, _, Type::Boolean) => lhsty,
                    (BinOp::And | BinOp::Or, _, _) => return Err(Error::TypeError(*sloc,
                            format!("'&' and '|' only work for booleans"))),
                    (_, false, Type::Integer) => lhsty,
                    _ => unimplemented!()
                };
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::Call { typ: Some(t), .. } => Ok(t.clone()),
            Node::Call { sloc, typ, callable, args } => {
                let callablety = callable.borrow_mut().typecheck(env)?;
                if let Type::Lambda(argtypes, rettyp) = callablety.as_ref() {
                    if args.len() != argtypes.len() {
                        return Err(Error::Uncallable(callable.clone()))
                    }

                    for (arg, (_, argtyp)) in args.iter().zip(argtypes.iter()) {
                        let t = arg.borrow_mut().typecheck(env)?;
                        if t.as_ref() != argtyp.as_ref() {
                            return Err(Error::TypeError(*sloc,
                                    format!("expected: {}, found: {}", argtyp, t)));
                        }
                    }

                    *typ = Some(rettyp.clone());
                    return Ok(rettyp.clone())
                }

                Err(Error::Uncallable(callable.clone()))
            },
            Node::IfThenElse { typ: Some(t), .. } => Ok(t.clone()),
            Node::IfThenElse { sloc, typ, op0, op1, op2 } => {
                let op0ty = op0.borrow_mut().typecheck(env)?;
                if op0ty.as_ref() != &Type::Boolean {
                    return Err(Error::TypeError(*sloc,
                            format!("condition of if-then-else needs to be a boolean, not: {}",
                                op0ty.as_ref())))
                }

                let op1ty = op1.borrow_mut().typecheck(env)?;
                let op2ty = op2.borrow_mut().typecheck(env)?;

                if op1ty.as_ref() != op2ty.as_ref() {
                    return Err(Error::TypeError(*sloc,
                            format!("branches of if-then-else needs to be of same type, not: {} and {}",
                                op1ty.as_ref(), op2ty.as_ref())))

                }

                *typ = Some(op1ty);
                Ok(op2ty)
            },
            Node::LetIn { typ: Some(t), .. } => Ok(t.clone()),
            Node::LetIn { typ, name, value, body, .. } => {
                let valty = value.borrow_mut().typecheck(env)?;
                env.push(name, Value::Type(None, valty));
                let t = body.borrow_mut().typecheck(env)?;
                env.pop(1);
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::Lambda { typ: Some(t), .. } => Ok(t.clone()),
            Node::Lambda { typ, args, body, .. } => {
                let mut argtypes = Vec::with_capacity(args.len());
                for (name, argtyp, rawtyp) in args.iter_mut() {
                    assert!(typ.is_none());
                    let t = rawtyp.borrow_mut().typecheck(env)?;
                    *argtyp = Some(t.clone());
                    env.push(name, Value::Type(None, t.clone()));
                    argtypes.push((name.clone(), t));
                }

                let rettyp = body.borrow_mut().typecheck(env)?;
                let t = Rc::new(Type::Lambda(argtypes, rettyp));
                env.pop(args.len());
                *typ = Some(t.clone());
                Ok(t)
            },
            Node::Forall { typ: Some(t), .. } => Ok(t.clone()),
            Node::Forall { sloc, typ, argtypes, rettyp } => {
                _ = (sloc, typ, argtypes, rettyp);
                todo!()
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
            Node::Invert { op0, .. } => write!(f, "(~{})", op0.borrow()),
            Node::BinOp { op, lhs, rhs, .. } =>
                write!(f, "({} {} {})",
                    lhs.borrow(),
                    match op { BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
                               BinOp::Div => "*", BinOp::And => "&", BinOp::Or => "|",
                               BinOp::EQ => "==", BinOp::NE => "!=", BinOp::LT => "<",
                               BinOp::LE => "<=", BinOp::GT => ">", BinOp::GE => "<=" },
                    rhs.borrow()),
            Node::Call { callable, args, .. } => {
                write!(f, "{}(", callable.as_ref().borrow())?;
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg.borrow())?;
                }
                write!(f, ")")
            },
            Node::IfThenElse { op0, op1, op2, .. } =>
                write!(f, "if ({}) then ({}) else ({})", op0.borrow(), op1.borrow(), op2.borrow()),
            Node::LetIn { name, value, body, .. } =>
                write!(f, "let {} = {} in {}", name.as_ref(), value.borrow(), body.borrow()),
            Node::Lambda { args, body, .. } => {
                write!(f, "λ(")?;
                for (i, (name, typ, rawtyp)) in args.iter().enumerate() {
                    if i != 0 { write!(f, ", ")?; }
                    if let Some(t) = typ {
                        write!(f, "{}: {}", name.as_ref(), t.as_ref())?;
                    } else {
                        write!(f, "{}: {}", name.as_ref(), rawtyp.borrow())?;
                    }
                }
                write!(f, ") -> {}", body.borrow())
            },
            Node::Forall { argtypes, rettyp, .. } => {
                write!(f, "∀(")?;
                for (i, (name, typ, rawtyp)) in argtypes.iter().enumerate() {
                    if i != 0 { write!(f, ", ")?; }
                    if let Some(t) = typ {
                        write!(f, "{}: {}", name.as_ref(), t.as_ref())?;
                    } else {
                        write!(f, "{}: {}", name.as_ref(), rawtyp.borrow())?;
                    }
                }
                write!(f, ") -> {}", rettyp.borrow())
            },
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, _other: &Self) -> bool { false }
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
            (sloc, t) => {
                Err(Error::ExpectedToken {
                    sloc,
                    expected: Tok::Id(Rc::from("<whatever>")),
                    found: t,
                })
            }
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

    pub fn parse_all(&mut self) -> Result<NodeRef, Error> {
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

    fn parse(&mut self) -> Result<NodeRef, Error> {
        let (sloc, tok) = self.lexer.peek().ok_or(Error::UnexpectedEOF)??;
        if tok == Tok::If {
            self.lexer.next();
            let op0 = self.parse_expr1()?;
            self.expect_token(Tok::Then)?;
            let op1 = self.parse_expr1()?;
            self.expect_token(Tok::Else)?;
            let op2 = self.parse_expr1()?;
            return Ok(Rc::new(RefCell::new(Node::IfThenElse {
                sloc,
                typ: None,
                op0,
                op1,
                op2,
            })));
        }

        if tok == Tok::Let {
            self.lexer.next();
            let (_, name) = self.expect_id()?;
            self.expect_token(Tok::Assign)?;
            let value = self.parse_expr1()?;

            let body = if let Some(Ok((_, Tok::Let))) = self.lexer.peek() {
                self.parse()?
            } else {
                self.expect_token(Tok::In)?;
                self.parse()?
            };
            return Ok(Rc::new(RefCell::new(Node::LetIn {
                sloc,
                name,
                typ: None,
                value,
                body,
            })));
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
            _ => None
        }
    }

    fn parse_binop(&mut self, min_prec: usize) -> Result<Rc<RefCell<Node>>, Error> {
        let mut lhs = self.parse_expr0()?;
        while let Some(Ok((sloc, tok))) = self.lexer.peek() {
            let (binop, prec, leftassoc, iscmp) = match Self::binop_precedence(&tok) {
                Some(t) if t.1 >= min_prec => t,
                _ => break
            };

            self.lexer.next().unwrap().unwrap();

            let next_min_prev = if leftassoc { prec + 1 } else { prec };
            let rhs = self.parse_binop(next_min_prev)?;
            lhs = Rc::new(RefCell::new(Node::BinOp {
                sloc,
                typ: None,
                op: binop,
                lhs,
                rhs,
                iscmp
            }));
        }
        Ok(lhs)
    }

    fn parse_expr1(&mut self) -> Result<Rc<RefCell<Node>>, Error> {
        self.parse_binop(0)
    }

    fn parse_expr0(&mut self) -> Result<Rc<RefCell<Node>>, Error> {
        let mut expr = self.parse_final()?;
        loop {
            if self.consume_if(Tok::LParen) {
                let mut args = vec![];
                loop {
                    args.push(self.parse()?);
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

                expr = Rc::new(RefCell::new(Node::Call {
                    sloc: self.consumed_sloc,
                    typ: None,
                    callable: expr,
                    args,
                }));
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_final(&mut self) -> Result<NodeRef, Error> {
        let (sloc, tok) = self.lexer.next().ok_or(Error::UnexpectedEOF)??;
        Ok(Rc::new(RefCell::new(match tok {
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
                    body,
                }
            },
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
                Node::Forall { sloc, typ: None, argtypes, rettyp }
            },
            _ => unimplemented!(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(
        input: &'static str,
        spool: &mut std::collections::HashSet<Rc<str>>,
    ) -> Result<Rc<RefCell<Node>>, Error> {
        let mut lexer = Lexer::new(input, 0, spool);
        let mut parser = Parser::new(&mut lexer);
        parser.parse()
    }

    fn clone_node(node: &NodeRef) -> Node {
        (&*node.borrow()).clone()
    }

    #[test]
    fn lambda() {
        let mut spool = std::collections::HashSet::new();
        let node = parse("let inc = λ(x: Int) -> x + 1 in inc(41)", &mut spool).unwrap();

        assert!(match clone_node(&node) {
            Node::LetIn {
                name, value, body, ..
            } if name.as_ref() == "inc" =>
                (match clone_node(&value) {
                    Node::Lambda { args, body, .. } if args.len() == 1 =>
                        (match &args[0] {
                            (name, None, typ) if name.as_ref() == "x" => match &*typ.borrow() {
                                Node::Id { name, .. } if name.as_ref() == "Int" => true,
                                _ => false,
                            },
                            _ => false,
                        } && match clone_node(&body) {
                            Node::BinOp {
                                op: BinOp::Add,
                                lhs,
                                rhs,
                                iscmp: false,
                                ..
                            } =>
                                (match clone_node(&lhs) {
                                    Node::Id { name, .. } if name.as_ref() == "x" => true,
                                    _ => false,
                                } && match clone_node(&rhs) {
                                    Node::Integer { value, .. } if value == 1 => true,
                                    _ => false,
                                }),
                            _ => false,
                        }),
                    _ => false,
                } && match clone_node(&body) {
                    Node::Call { callable, args, .. } if args.len() == 1 =>
                        (match clone_node(&callable) {
                            Node::Id { name, .. } if name.as_ref() == "inc" => true,
                            _ => false,
                        } && match clone_node(&args[0]) {
                            Node::Integer { value, .. } if value == 41 => true,
                            _ => false,
                        }),
                    _ => false,
                }),
            _ => false,
        });
    }
}
