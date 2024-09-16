use crate::core::{Error, SLoc};

use std::rc::Rc;

pub struct Lexer<'a> {
    sloc: SLoc,
    chars: std::iter::Peekable<std::str::Chars<'a>>, // Maybe use custom (cloning) peek logic?
    buffer: String,
    string_pool: &'a mut std::collections::HashSet<Rc<str>>,
    peeked: Option<Result<(SLoc, Tok), Error>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Tok {
    Id(Rc<str>),
    Boolean(bool),
    Integer(i64),
    // TODO: A real type...
    String(Rc<str>),

    Let,
    In,
    If,
    Then,
    Else,
    Lambda,
    Forall,
    Typeof,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    Arrow,
    ThickArrow,
    Assign,
    Colon,
    Comma,
    Tilde,

    Plus,
    Minus,
    Star,
    Slash,
    Ampersand,
    Pipe,
    Equal,
    NotEqual,
    Lower,
    LowerOrEqual,
    Greater,
    GreaterOrEqual
}

impl<'a> Lexer<'a> {
    pub fn new(
        input: &'a str,
        file_id: u16,
        string_pool: &'a mut std::collections::HashSet<Rc<str>>,
    ) -> Self {
        Self {
            sloc: SLoc {
                line: 1,
                col: 0,
                file_id,
            },
            chars: input.chars().peekable(),
            buffer: String::with_capacity(64),
            string_pool,
            peeked: None,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(c) = self.chars.next() {
            if c == '\n' {
                self.sloc.line += 1;
                self.sloc.col = 1;
            } else if c == '\t' {
                self.sloc.col += 4;
            } else {
                self.sloc.col += 1;
            }

            return Some(c);
        }
        None
    }

    // peek() is provided directly be the lexer instead of
    // std::iter::Peekable because its ok to clone often.
    pub fn peek(&mut self) -> Option<Result<(SLoc, Tok), Error>> {
        if self.peeked.is_none() {
            self.peeked = self.next();
        }
        self.peeked.clone()
    }

    fn skip_whitespace(&mut self) -> Option<char> {
        'outer: while let Some(c) = self.chars.next() {
            if c == '\n' {
                self.sloc.line += 1;
                self.sloc.col = 1;
                continue;
            }

            if c.is_whitespace() {
                self.sloc.col += if c == '\t' { 4 } else { 1 };
                continue;
            }

            if c == '#' || (c == '-' && self.chars.peek().cloned() == Some('-')) {
                for c in self.chars.by_ref() {
                    if c == '\n' {
                        self.sloc.line += 1;
                        self.sloc.col = 1;
                        break;
                    }
                }
                continue;
            }

            if c == '/' && self.chars.peek().cloned() == Some('*') {
                self.chars.next();
                while let Some(c) = self.next_char() {
                    if c == '*' && self.chars.peek().cloned() == Some('/') {
                        self.chars.next();
                        continue 'outer;
                    }
                }
            }

            self.sloc.col += 1;
            return Some(c);
        }
        None
    }

    fn parse_integer(&mut self, msd: Option<char>, base: u32) -> Result<i64, Error> {
        self.buffer.clear();
        if let Some(d) = msd {
            self.buffer.push(d);
        }

        while let Some(c) = self.chars.peek() {
            if !c.is_digit(base) {
                break;
            }
            self.sloc.col += 1;
            let c = self.next_char().unwrap();
            self.buffer.push(c);
        }

        self.buffer
            .parse::<i64>()
            .map_err(|e| Error::Lexer(self.sloc, format!("illegal integer literal: {:?}", e)))
    }

    fn parse_string_into_buffer(&mut self) -> Result<(), Error> {
        loop {
            match self.next_char() {
                Some('"') => return Ok(()),
                Some('\\') => {
                    let c = self.next_char();
                    let x = match c {
                        Some('x') => {
                            let d1 = self.next_char();
                            let d2 = self.next_char();
                            if d1.is_none()
                                || !d1.unwrap().is_ascii_hexdigit()
                                || d2.is_none()
                                || !d2.unwrap().is_ascii_hexdigit()
                            {
                                return Err(Error::Lexer(
                                    self.sloc,
                                    "illegal escape sequence".to_string(),
                                ));
                            }
                            (((d1.unwrap().to_ascii_lowercase() as u8 - b'a') << 4)
                                | (d2.unwrap().to_ascii_lowercase() as u8 - b'a'))
                                as char
                        }
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some('"') => '\"',
                        Some('\\') => '\\',
                        Some(c) => {
                            return Err(Error::Lexer(
                                self.sloc,
                                format!("unknown escape character: {:?}", c),
                            ))
                        }
                        None => return Err(Error::UnexpectedEOF),
                    };
                    self.buffer.push(x);
                }
                Some(c) => self.buffer.push(c),
                None => return Err(Error::UnexpectedEOF),
            }
        }
    }

    fn get_buffer_as_string(&mut self) -> Rc<str> {
        if let Some(s) = self.string_pool.get(self.buffer.as_str()) {
            s.clone()
        } else {
            let s = Rc::from(self.buffer.clone());
            self.string_pool.insert(Rc::clone(&s));
            s
        }
    }
}

impl<'a> std::iter::Iterator for Lexer<'a> {
    type Item = Result<(SLoc, Tok), Error>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.peeked.is_some() {
            return self.peeked.take();
        }

        let c = match self.skip_whitespace() {
            Some(c) => c,
            None => return None,
        };

        Some(match c {
            '(' => Ok((self.sloc, Tok::LParen)),
            ')' => Ok((self.sloc, Tok::RParen)),
            '[' => Ok((self.sloc, Tok::LBracket)),
            ']' => Ok((self.sloc, Tok::RBracket)),
            '{' => Ok((self.sloc, Tok::LBrace)),
            '}' => Ok((self.sloc, Tok::RBrace)),
            ',' => Ok((self.sloc, Tok::Comma)),
            ':' => Ok((self.sloc, Tok::Colon)),

            '+' => Ok((self.sloc, Tok::Plus)),
            '*' => Ok((self.sloc, Tok::Star)),
            '/' => Ok((self.sloc, Tok::Slash)),
            '&' => Ok((self.sloc, Tok::Ampersand)),
            '|' => Ok((self.sloc, Tok::Pipe)),
            '~' => Ok((self.sloc, Tok::Tilde)),
            '-' => match self.chars.peek() {
                Some('>') => {
                    self.next_char();
                    Ok((self.sloc, Tok::Arrow))
                }
                _ => Ok((self.sloc, Tok::Minus)),
            },
            '=' => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Ok((self.sloc, Tok::Equal))
                }
                Some('>') => {
                    self.next_char();
                    Ok((self.sloc, Tok::ThickArrow))
                }
                _ => Ok((self.sloc, Tok::Assign)),
            },
            '!' => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Ok((self.sloc, Tok::NotEqual))
                },
                _ => todo!()
            },
            '<' => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Ok((self.sloc, Tok::LowerOrEqual))
                }
                _ => Ok((self.sloc, Tok::Lower)),
            },
            '>' => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Ok((self.sloc, Tok::GreaterOrEqual))
                }
                _ => Ok((self.sloc, Tok::Greater))
            },

            '\\' | 'λ' => Ok((self.sloc, Tok::Lambda)),
            '∀' => Ok((self.sloc, Tok::Forall)),
            '⊤' => Ok((self.sloc, Tok::Boolean(true))),
            '⊥' => Ok((self.sloc, Tok::Boolean(false))),
            '→' => Ok((self.sloc, Tok::Arrow)),

            '0' => match self.chars.peek().cloned() {
                Some('b') => self
                    .parse_integer(None, 2)
                    .map(|i| (self.sloc, Tok::Integer(i))),
                Some('o') => self
                    .parse_integer(None, 8)
                    .map(|i| (self.sloc, Tok::Integer(i))),
                Some('x') => self
                    .parse_integer(None, 16)
                    .map(|i| (self.sloc, Tok::Integer(i))),
                Some(c) if !c.is_alphanumeric() => Ok((self.sloc, Tok::Integer(0))),
                None => Ok((self.sloc, Tok::Integer(0))),
                Some(c) => Err(Error::Lexer(
                    self.sloc,
                    format!("illegal character following a 0: {:?}", c),
                )),
            },
            '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => self
                .parse_integer(Some(c), 10)
                .map(|i| (self.sloc, Tok::Integer(i))),

            '"' => {
                self.buffer.clear();
                if let Err(e) = self.parse_string_into_buffer() {
                    return Some(Err(e));
                }
                Ok((self.sloc, Tok::String(self.get_buffer_as_string())))
            }

            '`' => {
                self.buffer.clear();
                loop {
                    let c = match self.next_char() {
                        None => {
                            return Some(Err(Error::Lexer(
                                self.sloc,
                                "Unterminated string litteral".to_string(),
                            )))
                        }
                        Some('`') => break,
                        Some(c) => c,
                    };
                    self.buffer.push(c);
                }
                Ok((self.sloc, Tok::String(self.get_buffer_as_string())))
            }

            c if c.is_alphabetic() || c == '_' => {
                self.buffer.clear();
                self.buffer.push(c);
                // This loop will be entered a lot and can surely be optimized...
                while let Some(c) = self.chars.peek() {
                    if !c.is_alphanumeric() && *c != '_' {
                        break;
                    }
                    let c = self.next_char().unwrap();
                    self.buffer.push(c);
                }

                match self.buffer.as_str() {
                    "lambda" => Ok((self.sloc, Tok::Lambda)),
                    "forall" => Ok((self.sloc, Tok::Forall)),
                    "true" => Ok((self.sloc, Tok::Boolean(true))),
                    "false" => Ok((self.sloc, Tok::Boolean(false))),
                    "let" => Ok((self.sloc, Tok::Let)),
                    "in" => Ok((self.sloc, Tok::In)),
                    "if" => Ok((self.sloc, Tok::If)),
                    "then" => Ok((self.sloc, Tok::Then)),
                    "else" => Ok((self.sloc, Tok::Else)),
                    "typeof" => Ok((self.sloc, Tok::Typeof)),
                    _ => Ok((self.sloc, Tok::Id(self.get_buffer_as_string()))),
                }
            }

            _ => todo!(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::assert_matches::assert_matches;

    #[test]
    fn lexing() {
        let input = "let x = { 42 } in λ(y: Str) -> \"Hello, World!\"";
        let mut string_pool = std::collections::HashSet::<Rc<str>>::new();
        let mut lexer = Lexer::new(input, 0, &mut string_pool);

        assert_matches!(lexer.next(), Some(Ok((_, Tok::Let))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Id(id)))) if id.as_ref() == "x");
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Assign))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::LBrace))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Integer(42)))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::RBrace))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::In))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Lambda))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::LParen))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Id(id)))) if id.as_ref() == "y");
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Colon))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Id(id)))) if id.as_ref() == "Str");
        assert_matches!(lexer.next(), Some(Ok((_, Tok::RParen))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::Arrow))));
        assert_matches!(lexer.next(), Some(Ok((_, Tok::String(str)))) if str.as_ref() == "Hello, World!");
        assert_matches!(lexer.next(), None);
    }
}
