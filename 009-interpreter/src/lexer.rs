use std::iter::{Iterator, Peekable};
use std::str::Chars;

// Tuple of (file-id, line, col)
pub type Pos = (u16, u16, u16);
pub const NULLPOS: Pos = (0, 0, 0);

#[derive(Clone, Debug, PartialEq)]
pub enum Tok<'input> {
    // Literalls:
    Int(Pos, i64),
    Real(Pos, f64),
    Bool(Pos, bool),
    Str(Pos, &'input str),

    // Generall Tokens:
    Id(Pos, &'input str),
    Colon(Pos),
    Comma(Pos),
    Assign(Pos),
    ThinArrow(Pos),
    ThickArrow(Pos),
    LeftParen(Pos),
    RightParen(Pos),
    LeftCurly(Pos),
    RightCurly(Pos),
    LeftSquare(Pos),
    RightSquare(Pos),

    // Operators:
    Plus(Pos),
    Minus(Pos),
    Star(Pos),
    Div(Pos),
    Equal(Pos),
    NotEqual(Pos),
    Lower(Pos),
    Greater(Pos),
    LowerOrEqual(Pos),
    GreaterOrEqual(Pos),

    // Keywords:
    And(Pos),
    Or(Pos),
    Let(Pos),
    In(Pos),
    If(Pos),
    Then(Pos),
    Else(Pos),

    // Special:
    Error(Pos, String),
    Nil
}

pub struct Lexer<'input> {
    input: &'input str,
    chars: Peekable<Chars<'input>>,
    fileid: u16,
    line: u16,
    lastline: usize,
    pos: usize
}

impl<'input> Lexer<'input> {
    pub fn new(fileid: u16, input: &'input str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            fileid,
            line: 1,
            lastline: 0,
            pos: 0
        }
    }

    fn getpos(&self) -> Pos {
        (self.fileid, self.line, (self.pos - self.lastline) as u16)
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Tok<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let c: char;
        loop { // Skip whitespace...
            self.pos += 1;
            c = match self.chars.next() {
                Some(' ') => continue,
                Some('\t') => continue,
                Some('\n') => {
                    self.lastline = self.pos;
                    self.line += 1;
                    continue;
                },
                Some('#') => {
                    loop {
                        self.pos += 1;
                        match self.chars.next() {
                            Some('\n') => break,
                            Some(_) => continue,
                            None => return None
                        }
                    }
                    self.lastline = self.pos;
                    self.line += 1;
                    continue
                },
                Some(c) => c,
                None => return None
            };
            break
        }

        let pos = self.getpos();
        match c {
            '+' => Some(Tok::Plus(pos)),
            '*' => Some(Tok::Star(pos)),
            ':' => Some(Tok::Colon(pos)),
            ',' => Some(Tok::Comma(pos)),
            '(' => Some(Tok::LeftParen(pos)),
            ')' => Some(Tok::RightParen(pos)),
            '{' => Some(Tok::LeftCurly(pos)),
            '}' => Some(Tok::RightCurly(pos)),
            '[' => Some(Tok::LeftSquare(pos)),
            ']' => Some(Tok::RightSquare(pos)),

            '=' => match self.chars.peek() {
                Some(&'=') => { self.pos += 1; self.chars.next(); Some(Tok::Equal(pos)) },
                Some(&'>') => { self.pos += 1; self.chars.next(); Some(Tok::ThickArrow(pos)) },
                _ => Some(Tok::Assign(pos))
            },
            '/' => match self.chars.peek() {
                Some(&'=') => { self.pos += 1; self.chars.next(); Some(Tok::NotEqual(pos)) },
                _ => Some(Tok::Div(pos))
            }
            '-' => match self.chars.peek() {
                Some(&'>') => { self.pos += 1; self.chars.next(); Some(Tok::ThinArrow(pos)) },
                _ => Some(Tok::Minus(pos))
            },
            '<' => match self.chars.peek() {
                Some(&'=') => { self.pos += 1; self.chars.next(); Some(Tok::LowerOrEqual(pos)) },
                _ => Some(Tok::Lower(pos))
            },
            '>' => match self.chars.peek() {
                Some(&'=') => { self.pos += 1; self.chars.next(); Some(Tok::GreaterOrEqual(pos)) },
                _ => Some(Tok::Greater(pos))
            },

            '0'..='9' => {
                let mut contains_dot = false;
                let mut start = self.pos - 1;
                let base = match self.chars.peek() {
                    Some(&'x') => { start = self.pos + 1; self.chars.next(); 16 },
                    Some(&'b') => { start = self.pos + 1; self.chars.next(); 2 },
                    _ => 10
                };

                loop {
                    match self.chars.peek() {
                        Some(c) => match *c {
                            '0'..='9' | 'a'..='f' => {
                                self.pos += 1;
                                self.chars.next();
                            },
                            '.' => {
                                self.pos += 1;
                                self.chars.next();
                                contains_dot = true;
                            },
                            _ => break
                        },
                        None => break,
                    }
                }

                let text = &self.input[start..self.pos];
                match contains_dot {
                    true => match text.parse::<f64>() {
                        Ok(x) => Some(Tok::Real(pos, x)),
                        Err(e) => Some(Tok::Error(pos, format!("invalid real literall: {}", e))),
                    },
                    false => match i64::from_str_radix(text, base) {
                        Ok(x) => Some(Tok::Int(pos, x)),
                        Err(e) => Some(Tok::Error(pos, format!("invalid integer literall: {}", e))),
                    }
                }
            },

            '"' => {
                let start = self.pos;
                loop {
                    self.pos += 1;
                    match self.chars.next() {
                        Some('"') => break,
                        Some('\\') => unimplemented!(),
                        Some(_) => continue,
                        None => return Some(Tok::Error(self.getpos(), format!("unexpexted EOF in string literall")))
                    }
                }

                Some(Tok::Str(pos, &self.input[start..(self.pos - 1)]))
            },

            'a'..='z' | 'A'..='Z' => {
                let start = self.pos - 1;
                loop {
                    match self.chars.peek() {
                        Some(c) => match *c {
                            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                                self.pos += 1;
                                self.chars.next();
                            },
                            _ => break
                        },
                        None => break,
                    }
                }

                match &self.input[start..self.pos] {
                    "and" => Some(Tok::And(pos)),
                    "or" => Some(Tok::Or(pos)),
                    "let" => Some(Tok::Let(pos)),
                    "in" => Some(Tok::In(pos)),
                    "if" => Some(Tok::If(pos)),
                    "then" => Some(Tok::Then(pos)),
                    "else" => Some(Tok::Else(pos)),
                    "true" => Some(Tok::Bool(pos, true)),
                    "false" => Some(Tok::Bool(pos, false)),
                    id => Some(Tok::Id(pos, id))
                }
            },
            _ => Some(Tok::Error(self.getpos(), format!("unexpected character: {:?}", c)))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_INPUT: &'static str = "hi
    # hallo welt
    (42)
    let x={\"das ist ein test\"}
    ->=>hallo
    ";

    #[test]
    fn lexer() {
        let mut lexer = Lexer::new(0, TEST_INPUT);
        assert_eq!(lexer.next().unwrap(), Tok::Id((0, 1, 1), "hi"));
        assert_eq!(lexer.next().unwrap(), Tok::LeftParen((0, 3, 5)));
        assert_eq!(lexer.next().unwrap(), Tok::Int((0, 3, 6), 42));
        assert_eq!(lexer.next().unwrap(), Tok::RightParen((0, 3, 8)));
        assert_eq!(lexer.next().unwrap(), Tok::Let((0, 4, 5)));
        assert_eq!(lexer.next().unwrap(), Tok::Id((0, 4, 9), "x"));
        assert_eq!(lexer.next().unwrap(), Tok::Assign((0, 4, 10)));
        assert_eq!(lexer.next().unwrap(), Tok::LeftCurly((0, 4, 11)));
        assert_eq!(lexer.next().unwrap(), Tok::Str((0, 4, 12), "das ist ein test"));
        assert_eq!(lexer.next().unwrap(), Tok::RightCurly((0, 4, 30)));
        assert_eq!(lexer.next().unwrap(), Tok::ThinArrow((0, 5, 5)));
        assert_eq!(lexer.next().unwrap(), Tok::ThickArrow((0, 5, 7)));
        assert_eq!(lexer.next().unwrap(), Tok::Id((0, 5, 9), "hallo"));
        assert_eq!(lexer.next(), None);
    }
}
