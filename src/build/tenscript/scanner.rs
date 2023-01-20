use std::fmt::{Display, Formatter};
use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use crate::build::tenscript::error;
use crate::build::tenscript::scanner::ErrorKind::{FloatParseFailed, IllegalChar, IntParseFailed};
use crate::build::tenscript::scanner::Token::{*};

#[derive(Debug, Clone)]
pub enum Token {
    Atom(String),
    Identifier(String),
    Parenthesis(char),
    QuotedString(String),
    WholeNumber(usize),
    FloatingPoint(f32),
    EndOfFile,
}

#[derive(Debug, Clone, Default)]
pub struct Location {
    line: usize,
    col: usize,
}

#[derive(Debug, Clone)]
pub struct ScannedToken {
    pub(crate) tok: Token,
    loc: Location,
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Location { line, col } = self;
        write!(f, "{line}:{col}")
    }
}

#[derive(Debug, Clone)]
pub struct ScanError {
    kind: ErrorKind,
    loc: Location,
}

impl Display for ScanError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ScanError { kind, loc } = self;
        write!(f, "{kind:?} at {loc}")
    }
}

impl std::error::Error for ScanError {}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    IllegalChar { ch: char },
    UnterminatedString,
    IntParseFailed { err: ParseIntError },
    FloatParseFailed { err: ParseFloatError },
}

pub fn scan(source: &str) -> Result<Vec<ScannedToken>, error::Error> {
    Scanner::new(source).scan().map_err(error::Error::ScanError)
}

struct Scanner {
    chars: Vec<char>,
    tokens: Vec<ScannedToken>,
    index: usize,
    start: usize,
    loc: Location,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            tokens: Default::default(),
            start: 0,
            index: 0,
            loc: Default::default(),
        }
    }

    pub fn scan(mut self) -> Result<Vec<ScannedToken>, ScanError> {
        while !self.at_end() {
            self.start = self.index;
            self.scan_token().map_err(|kind| ScanError {
                kind,
                loc: self.loc.clone(),
            })?;
        }
        self.add(EndOfFile);
        Ok(self.tokens)
    }

    fn scan_token(&mut self) -> Result<(), ErrorKind> {
        match self.current() {
            '0'..='9' | '-' | '.' => self.number()?,
            'a'..='z' => self.ident(),
            ':' => self.atom(),
            '"' => self.string()?,
            '\n' => {
                self.loc.line += 1;
                self.loc.col = 0;
                self.increment();
            }
            ' ' | '\t' => self.increment(),
            ch @ ('(' | ')') => {
                self.increment();
                self.add(Parenthesis(ch));
            }
            ch => return Err(IllegalChar { ch }),
        }
        Ok(())
    }

    fn at_end(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn current(&self) -> char {
        self.chars[self.index]
    }

    fn increment(&mut self) {
        self.index += 1;
        self.loc.col += 1;
    }

    fn add(&mut self, tok: Token) {
        self.tokens.push(ScannedToken {
            tok,
            loc: self.loc.clone(),
        })
    }

    fn lexeme(&self) -> String {
        self.chars[self.start..self.index].iter().collect()
    }

    fn consume_ident_chars(&mut self) {
        let ('a'..='z' | 'A'..='Z') = self.current() else {
            return;
        };
        self.increment();
        while let 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '+' = self.current() {
            self.increment();
        }
    }

    fn number(&mut self) -> Result<(), ErrorKind> {
        let mut num_string = String::new();
        while let ch @ ('0'..='9') = self.current() {
            num_string.push(ch);
            self.increment();
        }
        if let '.' = self.current() {
            if num_string.is_empty() {
                num_string.push('0');
            }
            num_string.push('.');
            self.increment();
            while let ch @ '0'..='9' = self.current() {
                num_string.push(ch);
                self.increment();
            }
            let value = f32::from_str(&num_string)
                .map_err(|err| FloatParseFailed { err })?;
            self.add(FloatingPoint(value));
        } else {
            let value = usize::from_str(&num_string)
                .map_err(|err| IntParseFailed { err })?;
            self.add(WholeNumber(value));
        }
        Ok(())
    }

    fn atom(&mut self) {
        self.increment();
        self.consume_ident_chars();
        let mut name = self.lexeme();
        name.remove(0); // remove prefix ':'
        self.add(Atom(name));
    }

    fn ident(&mut self) {
        self.consume_ident_chars();
        let name = self.lexeme();
        self.add(Identifier(name));
    }

    fn string(&mut self) -> Result<(), ErrorKind> {
        self.increment();
        while self.current() != '"' {
            self.increment();
            if self.at_end() {
                return Err(ErrorKind::UnterminatedString);
            }
        }
        self.increment();
        let mut string = self.lexeme();
        string.remove(0);
        string.remove(string.len() - 1);
        self.add(QuotedString(string));
        Ok(())
    }
}
