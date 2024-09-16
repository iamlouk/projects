use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Display, Pointer};
use std::path::PathBuf;
use std::rc::Rc;
use crate::common::*;

#[allow(dead_code)]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Tok {
    /* Special: */
    EndOfFile,
    Expand(Rc<Vec<(SLoc, Tok)>>),

    /* Literals: */
    Id(Rc<str>),
    String(Rc<str>),
    IntLit(i64),
    RealLit(f64),
    CharLit(char),

    /* Keywords: */
    Alignas,
    Alignof,
    Auto,
    Bool,
    Break,
    Case,
    Char,
    Const,
    Constexpr,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Extern,
    False,
    Float,
    For,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Nullptr,
    Register,
    Restrict,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    StaticAssert,
    Struct,
    Switch,
    ThreadLocal,
    True,
    Typedef,
    Typeof,
    Union,
    Unsigned,
    Void,
    Volatile,
    While,
    Attribute,
    /* Non-C: */
    Export,
    Fn,

    /* Operators */
    Assign,
    AssignAdd,
    AssignSub,
    AssignMul,
    AssignDiv,
    AssignMod,
    AssignBitAnd,
    AssignBitOr,
    AssignBitXOr,
    AssignLeftShift,
    AssignRightShift,
    PlusPlus,
    MinusMinus,
    Plus,
    Minus,
    Star,
    Divide,
    Modulo,
    BitwiseNot,
    Ampersand,
    BitwiseOr,
    BitwiseXOr,
    ShiftLeft,
    ShiftRight,
    LogicalNot,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Smaller,
    Bigger,
    SmallerOrEqual,
    BiggerOrEqual,

    /* Others */
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBraces,
    RBraces,
    Arrow,
    Dot,
    Comma,
    QuestionMark,
    Colon,
    SemiColon,
}

impl Tok {
    pub fn is_eof(&self) -> bool { *self == Tok::EndOfFile }
}

impl Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Tok::*;
        f.write_str(match self {
            EndOfFile => return Ok(()),
            Expand(toks) => {
                for tok in toks.iter() {
                    tok.fmt(f)?;
                }
                return Ok(())
            },
            Id(id) => return write!(f, "{}", &**id),
            String(str) => return write!(f, "{:?}", &**str),
            IntLit(x) => return write!(f, "{:x}", x),
            RealLit(x) => return write!(f, "{}", x),
            CharLit(x) => match *x {
                '\n' => "'\\n'",
                '\r' => "'\\r'",
                '\t' => "'\\t'",
                '\0' => "'\\0'",
                x => return write!(f, "{:?}", x)
            },
            Alignas => "_Alignas",
            Alignof => "_Alignof",
            Auto => "auto",
            Bool => "bool",
            Break => "break",
            Case => "case",
            Char => "char",
            Const => "const",
            Constexpr => "constexpr",
            Continue => "continue",
            Default => "default",
            Do => "do",
            Double => "double",
            Else => "else",
            Enum => "enum",
            Extern => "extern",
            False => "false",
            Float => "float",
            For => "for",
            Goto => "goto",
            If => "if",
            Inline => "inline",
            Int => "int",
            Long => "long",
            Nullptr => "nullptr",
            Register => "register",
            Restrict => "restrict",
            Return => "return",
            Short => "short",
            Signed => "signed",
            Sizeof => "sizeof",
            Static => "static",
            StaticAssert => "_Static_assert",
            Struct => "struct",
            Switch => "switch",
            ThreadLocal => "_Thread_local",
            True => "true",
            Typedef => "typedef",
            Typeof => "typeof",
            Union => "union",
            Unsigned => "unsigned",
            Void => "void",
            Volatile => "volatile",
            While => "while",
            Attribute => "__attribute__",
            Export => "export",
            Fn => "fn",
            Assign => "=",
            AssignAdd => "+=",
            AssignSub => "-=",
            AssignMul => "*=",
            AssignDiv => "/=",
            AssignMod => "%=",
            AssignBitAnd => "&=",
            AssignBitOr => "|=",
            AssignBitXOr => "^=",
            AssignLeftShift => "<<=",
            AssignRightShift => ">>=",
            PlusPlus => "++",
            MinusMinus => "--",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Divide => "/",
            Modulo => "%",
            BitwiseNot => "~",
            Ampersand => "&",
            BitwiseOr => "|",
            BitwiseXOr => "^",
            ShiftLeft => "<<",
            ShiftRight => ">>",
            LogicalNot => "!",
            LogicalAnd => "&&",
            LogicalOr => "||",
            Equal => "==",
            NotEqual => "!=",
            Smaller => "<",
            Bigger => ">",
            SmallerOrEqual => "<=",
            BiggerOrEqual => ">=",
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            LBraces => "{",
            RBraces => "}",
            Arrow => "->",
            Dot => ".",
            Comma => ",",
            QuestionMark => "?",
            Colon => ":",
            SemiColon => ";",
        })
    }
}

struct State {
    string_pool: HashSet<Rc<str>>,
    defines: HashMap<Rc<str>, Rc<Vec<(SLoc, Tok)>>>,
    buf: String,
}

impl State {
    fn get_buf(&mut self) -> Rc<str> {
        if let Some(rc) = self.string_pool.get(self.buf.as_str()) {
            rc.clone()
        } else {
            let rc: Rc<str> = Rc::from(self.buf.as_str());
            self.string_pool.insert(rc.clone());
            rc
        }
    }
}

struct File<'input> {
    path: PathBuf,
    sloc: SLoc,
    input: &'input [u8],
    pos: usize,
}

impl<'input> File<'input> {
    fn next_char(&mut self) -> Option<char> {
        self.sloc.col += 1;
        self.pos += 1;
        self.input.get(self.pos - 1).map(|b| *b as char)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' | b'\r' => {
                    self.pos += 1;
                    self.sloc.col += 1;
                    continue;
                }
                b'\n' => {
                    self.pos += 1;
                    self.sloc.line += 1;
                    self.sloc.col = 1;
                    continue;
                }
                b'/' if self.input.get(self.pos + 1).cloned() == Some(b'/') => {
                    self.pos += 2;
                    while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                    continue;
                }
                b'/' if self.input.get(self.pos + 1).cloned() == Some(b'*') => {
                    self.pos += 2;
                    self.sloc.col += 2;
                    while self.pos < self.input.len() {
                        let c = self.input[self.pos];
                        self.pos += 1;
                        self.sloc.col += 1;
                        if c == b'\n' {
                            self.sloc.line += 1;
                            self.sloc.col = 1;
                        }
                        if c == b'*' && self.input.get(self.pos).cloned() == Some(b'/') {
                            self.pos += 1;
                            self.sloc.col += 1;
                            break
                        }
                    }
                    continue;
                }
                _ => return
            }
        }
    }

    fn next(&mut self, state: &mut State, raw: bool) -> Result<(SLoc, Tok), Error> {
        self.skip_whitespace();
        let sloc = self.sloc.clone();
        if self.pos >= self.input.len() {
            return Ok((sloc, Tok::EndOfFile))
        }

        let c = self.input[self.pos] as char;
        self.pos += 1;
        match c {
            '{' => Ok((sloc, Tok::LBraces)),
            '}' => Ok((sloc, Tok::RBraces)),
            '(' => Ok((sloc, Tok::LParen)),
            ')' => Ok((sloc, Tok::RParen)),
            '[' => Ok((sloc, Tok::LBracket)),
            ']' => Ok((sloc, Tok::RBracket)),
            ':' => Ok((sloc, Tok::Colon)),
            ',' => Ok((sloc, Tok::Comma)),
            ';' => Ok((sloc, Tok::SemiColon)),
            '?' => Ok((sloc, Tok::QuestionMark)),
            '.' => Ok((sloc, Tok::Dot)),
            '*' => Ok((sloc, Tok::Star)),
            '/' => Ok((sloc, Tok::Divide)),
            '~' => Ok((sloc, Tok::BitwiseNot)),
            '^' => Ok((sloc, Tok::BitwiseXOr)),
            '%' => Ok((sloc, Tok::Modulo)),
            '|' => match self.input.get(self.pos).cloned() {
                Some(b'|') => {
                    self.next_char();
                    Ok((sloc, Tok::LogicalOr))
                }
                _ => Ok((sloc, Tok::BitwiseOr)),
            },
            '&' => match self.input.get(self.pos).cloned() {
                Some(b'&') => {
                    self.next_char();
                    Ok((sloc, Tok::LogicalAnd))
                }
                _ => Ok((sloc, Tok::Ampersand)),
            },
            '+' => match self.input.get(self.pos).cloned() {
                Some(b'+') => {
                    self.next_char();
                    Ok((sloc, Tok::PlusPlus))
                }
                Some(b'=') => {
                    self.next_char();
                    Ok((sloc, Tok::AssignAdd))
                }
                _ => Ok((sloc, Tok::Plus)),
            },
            '-' => match self.input.get(self.pos).cloned() {
                Some(b'-') => {
                    self.next_char();
                    Ok((sloc, Tok::MinusMinus))
                }
                Some(b'>') => {
                    self.next_char();
                    Ok((sloc, Tok::Arrow))
                }
                Some(b'=') => {
                    self.next_char();
                    Ok((sloc, Tok::AssignSub))
                }
                _ => Ok((sloc, Tok::Minus)),
            },
            '!' => match self.input.get(self.pos).cloned() {
                Some(b'=') => {
                    self.next_char();
                    Ok((sloc, Tok::NotEqual))
                }
                _ => Ok((sloc, Tok::LogicalNot)),
            },
            '=' => match self.input.get(self.pos).cloned() {
                Some(b'=') => {
                    self.next_char();
                    Ok((sloc, Tok::Equal))
                }
                _ => Ok((sloc, Tok::Assign)),
            },
            '<' => match self.input.get(self.pos).cloned() {
                Some(b'=') => {
                    self.next_char();
                    Ok((sloc, Tok::SmallerOrEqual))
                }
                Some(b'<') => {
                    self.next_char();
                    Ok((sloc, Tok::ShiftLeft))
                }
                _ => Ok((sloc, Tok::Smaller)),
            },
            '>' => match self.input.get(self.pos).cloned() {
                Some(b'=') => {
                    self.next_char();
                    Ok((sloc, Tok::BiggerOrEqual))
                }
                Some(b'>') => {
                    self.next_char();
                    Ok((sloc, Tok::ShiftRight))
                }
                _ => Ok((sloc, Tok::Bigger)),
            },

            '\'' => match (self.next_char(), self.next_char()) {
                (Some(c), Some('\'')) if c != '\\' => Ok((sloc, Tok::CharLit(c))),
                (Some('\\'), Some(c)) => {
                    let c = match c {
                        '\\' => '\\',
                        'n' => '\n',
                        't' => '\t',
                        '0' => '\0',
                        _ => return Err(Error::InvalidTok(sloc, "invalid character litteral")),
                    };
                    if self.next_char() != Some('\'') {
                        Err(Error::InvalidTok(sloc, "invalid character litteral"))
                    } else {
                        Ok((sloc, Tok::CharLit(c)))
                    }
                }
                _ => Err(Error::InvalidTok(sloc, "invalid character litteral")),
            },

            '"' => {
                state.buf.clear();
                while let Some(mut c) = self.next_char() {
                    if c == '"' {
                        let res = state.get_buf();
                        return Ok((sloc, Tok::String(res)));
                    }

                    if c == '\\' {
                        c = match self.next_char() {
                            Some('\\') => '\\',
                            Some('\n') => '\n',
                            Some('\t') => '\t',
                            _ => return Err(Error::InvalidTok(sloc, "unknown escaped character in string")),
                        }
                    }

                    state.buf.push(c);
                }

                Err(Error::InvalidTok(sloc, "unterminated string litteral"))
            }

            '0'..='9' => {
                state.buf.clear();
                let radix = match (c, self.input.get(self.pos).cloned()) {
                    ('0', Some(b'x')) => {
                        self.next_char();
                        16
                    }
                    ('0', Some(b'o')) => {
                        self.next_char();
                        8
                    }
                    ('0', Some(b'b')) => {
                        self.next_char();
                        2
                    }
                    (c, _) => {
                        state.buf.push(c);
                        10
                    }
                };

                while let Some(c) = self.input.get(self.pos).cloned() {
                    if c != b'_' && c != b'.' && !c.is_ascii_alphanumeric() {
                        break;
                    }
                    state.buf.push(c as char);
                    self.next_char();
                }

                match i64::from_str_radix(state.buf.as_str(), radix) {
                    Ok(num) => Ok((sloc, Tok::IntLit(num))),
                    Err(e) => Err(Error::InvalidInt(sloc, e)),
                }
            }

            '_' | 'a'..='z' | 'A'..='Z' => {
                state.buf.clear();
                state.buf.push(c);
                while let Some(c) = self.input.get(self.pos).cloned() {
                    if !matches!(c, b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9') {
                        break;
                    }
                    state.buf.push(c as char);
                    self.next_char();
                }

                if raw {
                    return Ok((sloc, Tok::Id(state.get_buf())))
                }

                match state.buf.as_str() {
                    "alignas" => return Ok((sloc, Tok::Alignas)),
                    "alignof" => return Ok((sloc, Tok::Alignof)),
                    "auto" => return Ok((sloc, Tok::Auto)),
                    "bool" => return Ok((sloc, Tok::Bool)),
                    "break" => return Ok((sloc, Tok::Break)),
                    "case" => return Ok((sloc, Tok::Case)),
                    "char" => return Ok((sloc, Tok::Char)),
                    "const" => return Ok((sloc, Tok::Const)),
                    "constexpr" => return Ok((sloc, Tok::Constexpr)),
                    "continue" => return Ok((sloc, Tok::Continue)),
                    "default" => return Ok((sloc, Tok::Default)),
                    "do" => return Ok((sloc, Tok::Do)),
                    "double" => return Ok((sloc, Tok::Double)),
                    "else" => return Ok((sloc, Tok::Else)),
                    "enum" => return Ok((sloc, Tok::Enum)),
                    "extern" => return Ok((sloc, Tok::Extern)),
                    "false" => return Ok((sloc, Tok::False)),
                    "float" => return Ok((sloc, Tok::Float)),
                    "for" => return Ok((sloc, Tok::For)),
                    "goto" => return Ok((sloc, Tok::Goto)),
                    "if" => return Ok((sloc, Tok::If)),
                    "inline" => return Ok((sloc, Tok::Inline)),
                    "int" => return Ok((sloc, Tok::Int)),
                    "long" => return Ok((sloc, Tok::Long)),
                    "nullptr" => return Ok((sloc, Tok::Nullptr)),
                    "register" => return Ok((sloc, Tok::Register)),
                    "restrict" => return Ok((sloc, Tok::Restrict)),
                    "return" => return Ok((sloc, Tok::Return)),
                    "short" => return Ok((sloc, Tok::Short)),
                    "signed" => return Ok((sloc, Tok::Signed)),
                    "sizeof" => return Ok((sloc, Tok::Sizeof)),
                    "static" => return Ok((sloc, Tok::Static)),
                    "static_assert" => return Ok((sloc, Tok::StaticAssert)),
                    "struct" => return Ok((sloc, Tok::Struct)),
                    "switch" => return Ok((sloc, Tok::Switch)),
                    "thread_local" => return Ok((sloc, Tok::ThreadLocal)),
                    "true" => return Ok((sloc, Tok::True)),
                    "typedef" => return Ok((sloc, Tok::Typedef)),
                    "typeof" => return Ok((sloc, Tok::Typeof)),
                    "union" => return Ok((sloc, Tok::Union)),
                    "unsigned" => return Ok((sloc, Tok::Unsigned)),
                    "void" => return Ok((sloc, Tok::Void)),
                    "volatile" => return Ok((sloc, Tok::Volatile)),
                    "while" => return Ok((sloc, Tok::While)),
                    "__attribute__" => return Ok((sloc, Tok::Attribute)),
                    "fn" => return Ok((sloc, Tok::Fn)),
                    "export" => return Ok((sloc, Tok::Export)),
                    _ => {}
                };

                if let Some(m) = state.defines.get(state.buf.as_str()) {
                    return Ok((sloc, Tok::Expand(m.clone())))
                }

                let id = state.get_buf();
                Ok((sloc, Tok::Id(id)))
            }

            '#' => self.directive(sloc, state),

            c => Err(Error::Lex(sloc, format!("unexpected character: {:?}", c))),
        }
    }

    fn directive(&mut self, sloc: SLoc, state: &mut State) -> Result<(SLoc, Tok), Error> {
        let dir = match self.next(state, true)? {
            (_, Tok::Id(id)) => id,
            (sloc, t) => return Err(Error::PreProcessor(sloc, t, "unexpected token following '#'"))
        };

        if &*dir == "include" {
            let path = match self.next(state, true)? {
                (_, Tok::String(path)) => path,
                (sloc, t) => return Err(Error::PreProcessor(sloc, t, "expected string litteral after '#include'"))
            };
            let mut filepath = self.path.clone();
            filepath.pop();
            filepath.push(&*path);
            let input: Vec<u8> = std::fs::read(&filepath).map_err(|e| Error::IO(sloc.clone(), e))?;
            let mut file = File {
                sloc: SLoc::new(&filepath, 1, 1),
                path: filepath,
                input: &input,
                pos: 0
            };
            let mut toks = Vec::new();
            loop {
                match file.next(state, false)? {
                    (_, Tok::EndOfFile) => break,
                    (sloc, tok) => toks.push((sloc, tok))
                }
            }
            return Ok((sloc, Tok::Expand(Rc::new(toks))));
        }

        if &*dir == "define" {
            let line = self.sloc.line;
            let name = match self.next(state, true)? {
                (_, Tok::Id(id)) => id,
                (sloc, t) => return Err(Error::PreProcessor(sloc, t, "unexpected token following '#define'"))
            };

            let mut toks = Vec::new();
            loop {
                self.skip_whitespace();
                if line != self.sloc.line {
                    break;
                }
                match self.next(state, false)? {
                    (_, Tok::EndOfFile) => break,
                    (sloc, tok) => toks.push((sloc, tok))
                }
            }
            state.defines.insert(name, Rc::new(toks));
            return self.next(state, false);
        }

        if &*dir == "undef" {
            let name = match self.next(state, true)? {
                (_, Tok::Id(id)) => id,
                (sloc, t) => return Err(Error::PreProcessor(sloc, t, "unexpected token following '#define'"))
            };
            state.defines.remove(name.as_ref());
            return self.next(state, false);
        }

        Err(Error::PreProcessor(sloc, Tok::Id(dir), "unexpected token following '#'"))
    }
}

pub struct Lexer<'lexer> {
    file: File<'lexer>,
    peeked: VecDeque<(SLoc, Tok)>,
    expanding: VecDeque<(SLoc, Tok)>,
    state: State
}

impl<'lexer> Lexer<'lexer> {
    pub fn new(filepath: &std::path::Path, input: &'lexer [u8]) -> Self {
        Self {
            file: File {
                path: filepath.to_owned(),
                sloc: SLoc {
                    file: Rc::from(filepath),
                    line: 1,
                    col: 1
                },
                input,
                pos: 0
            },
            peeked: VecDeque::new(),
            expanding: VecDeque::new(),
            state: State {
                string_pool: HashSet::new(),
                defines: HashMap::new(),
                buf: String::with_capacity(64)
            }
        }
    }

    pub fn peek(&mut self) -> Result<(SLoc, Tok), Error> {
        if let Some(res) = self.peeked.front() {
            return Ok(res.clone())
        }
        let res = self.next()?;
        self.peeked.push_front(res.clone());
        Ok(res)
    }

    pub fn next(&mut self) -> Result<(SLoc, Tok), Error> {
        if let Some(res) = self.peeked.pop_front() {
            return Ok(res)
        }

        let tok = match self.expanding.pop_front() {
            Some(t) => t,
            None => self.file.next(&mut self.state, false)?
        };
        if let (_, Tok::Expand(toks)) = tok.clone() {
            for t in toks.iter().rev() {
                self.expanding.push_front(t.clone());
            }
            return self.next()
        }
        Ok(tok)
    }

    pub fn expect_token(&mut self, tok: Tok, msg: &'static str) -> Result<(), Error> {
        match self.next()? {
            (_, t) if t == tok => Ok(()),
            (sloc, t) => Err(Error::UnexpectedTok(sloc,
                format!("{}: expected: {:?}, found: {:?}", msg, tok, t)))
        }
    }

    pub fn expect_id(&mut self, msg: &'static str) -> Result<(SLoc, Rc<str>), Error> {
        match self.next()? {
            (sloc, Tok::Id(s)) => Ok((sloc, s)),
            (sloc, t) => Err(Error::UnexpectedTok(sloc,
                format!("{}: expected a identifier, found: {:?}", msg, t))),
        }
    }

    pub fn consume_if_next(&mut self, tok: Tok) -> Result<bool, Error> {
        if self.peek()?.1 == tok {
            self.next()?;
            return Ok(true)
        }
        Ok(false)
    }

    pub fn unread(&mut self, sloc: SLoc, tok: Tok) {
        self.peeked.push_front((sloc, tok));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::assert_matches::assert_matches;

    fn lex(input: &str) -> Vec<Tok> {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut toks = Vec::new();
        loop {
            match lex.next().unwrap() {
                (_, Tok::EndOfFile) => break,
                (_, tok) => toks.push(tok)
            }
        }
        toks
    }

    fn preprocess(input: &str) -> String {
        let toks = lex(input);
        let mut buf = String::new();
        use std::fmt::Write;
        write!(buf, "{}", toks[0]).unwrap();
        for tok in toks.iter().skip(1) { write!(buf, " {}", tok).unwrap(); }
        buf
    }

    #[test]
    fn basic() {
        let toks = lex("[foo]+/*comment*/(\"bar\")--42%0x1234");
        println!("tokens: {:?}", toks);
        assert_eq!(toks.len(), 11);
        assert_matches!(toks[0], Tok::LBracket);
        assert_matches!(toks[1], Tok::Id(ref id) if id.as_ref() == "foo");
        assert_matches!(toks[2], Tok::RBracket);
        assert_matches!(toks[3], Tok::Plus);
        assert_matches!(toks[4], Tok::LParen);
        assert_matches!(toks[5], Tok::String(ref s) if s.as_ref() == "bar");
        assert_matches!(toks[6], Tok::RParen);
        assert_matches!(toks[7], Tok::MinusMinus);
        assert_matches!(toks[8], Tok::IntLit(42));
        assert_matches!(toks[9], Tok::Modulo);
        assert_matches!(toks[10], Tok::IntLit(0x1234));
    }

    #[test]
    fn preprocess_basic() {
        let pp = preprocess("
                // hello
                static
                #define foo 1
                1/*blah blah blah*/2
                foo
                #undef foo
                foo
                // world
            ");
        println!("preprocessed: {:?}", pp);
        assert_eq!(pp.as_str(), "static 1 2 1 foo");
    }

    #[test]
    fn define_simple() {
        let toks = lex("
                foo
                #define foo 1
                foo
                #undef foo
                foo
            ");
        println!("tokens: {:?}", toks);
        assert_eq!(toks.len(), 3);
        assert_matches!(toks[0], Tok::Id(ref id) if id.as_ref() == "foo");
        assert_matches!(toks[1], Tok::IntLit(1));
        assert_matches!(toks[2], Tok::Id(ref id) if id.as_ref() == "foo");
    }

    static FIBS_EXAMPLE: &'static str = "
        export fn fibs(n: unsigned): unsigned =
            if n < 2 { n } else { fibs(n - 2) + fibs(n - 1) };";

    #[test]
    fn fibs() {
        let toks = lex(FIBS_EXAMPLE);
        println!("tokens: {:?}", toks);
        assert_eq!(toks.len(), 35);
        assert_matches!(toks[0], Tok::Export);
        assert_matches!(toks[1], Tok::Fn);
        assert_matches!(toks[2], Tok::Id(ref id) if id.as_ref() == "fibs");
        assert_matches!(toks[3], Tok::LParen);
        assert_matches!(toks[4], Tok::Id(ref id) if id.as_ref() == "n");
        assert_matches!(toks[5], Tok::Colon);
        assert_matches!(toks[6], Tok::Unsigned);
        assert_matches!(toks[7], Tok::RParen);
        assert_matches!(toks[8], Tok::Colon);
        assert_matches!(toks[9], Tok::Unsigned);
        assert_matches!(toks[10], Tok::Assign);
        assert_matches!(toks[11], Tok::If);
        assert_matches!(toks[12], Tok::Id(ref id) if id.as_ref() == "n");
        assert_matches!(toks[13], Tok::Smaller);
        assert_matches!(toks[14], Tok::IntLit(2));
        assert_matches!(toks[15], Tok::LBraces);
        assert_matches!(toks[16], Tok::Id(ref id) if id.as_ref() == "n");
        assert_matches!(toks[17], Tok::RBraces);
        assert_matches!(toks[18], Tok::Else);
        assert_matches!(toks[19], Tok::LBraces);
        assert_matches!(toks[20], Tok::Id(ref id) if id.as_ref() == "fibs");
        assert_matches!(toks[21], Tok::LParen);
        assert_matches!(toks[22], Tok::Id(ref id) if id.as_ref() == "n");
        assert_matches!(toks[23], Tok::Minus);
        assert_matches!(toks[24], Tok::IntLit(2));
        assert_matches!(toks[25], Tok::RParen);
        assert_matches!(toks[26], Tok::Plus);
        assert_matches!(toks[27], Tok::Id(ref id) if id.as_ref() == "fibs");
        assert_matches!(toks[28], Tok::LParen);
        assert_matches!(toks[29], Tok::Id(ref id) if id.as_ref() == "n");
        assert_matches!(toks[30], Tok::Minus);
        assert_matches!(toks[31], Tok::IntLit(1));
        assert_matches!(toks[32], Tok::RParen);
        assert_matches!(toks[33], Tok::RBraces);
        assert_matches!(toks[34], Tok::SemiColon);
    }
}
