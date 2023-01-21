use std::fmt::{Debug, Display, Formatter};

use crate::build::tenscript::error::Error;
use crate::build::tenscript::expression::ErrorKind::{ConsumeFailed, MatchExhausted};
use crate::build::tenscript::scanner;
use crate::build::tenscript::scanner::{ScannedToken, Token};
use crate::build::tenscript::scanner::Token::{*};

#[derive(Clone)]
pub enum Expression {
    Atom(String),
    FloatingPoint(f32),
    Identifier(String),
    List(Vec<Expression>),
    QuotedString(String),
    WholeNumber(usize),
}

impl Debug for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{self}'")
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::List(terms) => {
                f.write_str("(")?;
                for (i, term) in terms.iter().enumerate() {
                    Display::fmt(term, f)?;
                    if i < terms.len() - 1 {
                        f.write_str(" ")?;
                    }
                }
                f.write_str(")")?;
                Ok(())
            }
            Expression::Identifier(name) => write!(f, "{name}"),
            Expression::Atom(value) => write!(f, ":{value}"),
            Expression::QuotedString(value) => write!(f, "\"{value}\""),
            Expression::FloatingPoint(value) => write!(f, "{value}"),
            Expression::WholeNumber(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    kind: ErrorKind,
    token: ScannedToken,
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    MatchExhausted,
    ConsumeFailed { expected: &'static str },
}

pub fn parse(source: &str) -> Result<Expression, Error> {
    let lines: Vec<&str> = source.split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.starts_with(';'))
        .collect();
    let tokens = scanner::scan(lines.join("").as_str())?;
    parse_tokens(tokens)
}

pub fn parse_tokens(tokens: Vec<ScannedToken>) -> Result<Expression, Error> {
    Parser::new(tokens).parse().map_err(Error::ExpressionParseError)
}

struct Parser {
    tokens: Vec<ScannedToken>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<ScannedToken>) -> Self {
        Self { tokens, index: 0 }
    }

    pub fn parse(mut self) -> Result<Expression, ParseError> {
        self.expression().map_err(|kind| ParseError {
            kind,
            token: self.current_scanned().clone(),
        })
    }

    fn current_scanned(&self) -> &ScannedToken {
        &self.tokens[self.index]
    }

    fn current(&self) -> &Token {
        &self.current_scanned().tok
    }

    fn increment(&mut self) {
        self.index += 1;
    }

    fn expression(&mut self) -> Result<Expression, ErrorKind> {
        let token = self.current().clone();
        self.increment();
        match token {
            Atom(value) => Ok(Expression::Atom(value)),
            FloatingPoint(value) => Ok(Expression::FloatingPoint(value)),
            Identifier(name) => Ok(Expression::Identifier(name)),
            Parenthesis('(') => self.list(),
            QuotedString(value) => Ok(Expression::QuotedString(value)),
            WholeNumber(value) => Ok(Expression::WholeNumber(value)),
            _ => Err(MatchExhausted),
        }
    }

    fn list(&mut self) -> Result<Expression, ErrorKind> {
        let mut terms = Vec::new();
        while !matches!(self.current(), Parenthesis(')') | EndOfFile) {
            let term = self.expression()?;
            terms.push(term);
        }
        let Parenthesis(')') = self.current() else {
            return Err(ConsumeFailed { expected: "right bracket" });
        };
        self.increment();
        Ok(Expression::List(terms))
    }
}
