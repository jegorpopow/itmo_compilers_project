use core::fmt;
use core::num::{ParseFloatError, ParseIntError};

use common::{Extent, Integer, Real};

// Token types

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone, Copy)]
pub enum Keyword {
    Var,
    Type,
    Routine,
    Array,
    Record,
    Is,
    End,
    If,
    Then,
    Else,
    Return,
    In,
    While,
    For,
    Loop,
    Reverse,
    Print,
    New,
    Null,
    Where,
    Assert,
    Panic,
    Constant,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
pub struct Identifier<'a> {
    pub name: &'a str,
}

#[derive(PartialEq, fmt::Debug, Clone)]
pub enum Literal {
    Real(Result<Real, ParseFloatError>),
    Integer(Result<Integer, ParseIntError>),
    Bool(bool),
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone, Copy)]
pub enum BuiltinTypename {
    Integer,
    Real,
    Boolean,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
pub struct Comment<'a> {
    pub value: &'a str,
}

impl fmt::Display for Comment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const MAX_LEN: usize = 40;

        let Self { value } = self;
        let comment: &str = value;
        if comment.len() <= MAX_LEN {
            write!(f, "{comment}")
        } else {
            write!(f, "{} …", &comment[..comment.floor_char_boundary(MAX_LEN)])
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Token<'a> {
    Identifier(Identifier<'a>),
    Keyword(Keyword),
    Literal(Literal),
    BuiltinTypename(BuiltinTypename),
    Operator(Operator),
    Comment(Comment<'a>),
    Unexpected(char),
    LeftBracket,
    RightBracket,
    LeftParenthesis,
    RightParenthesis,
    /// `=>`
    RightArrow,
    /// `::`
    Cast,
    /// `:=`
    Assignment,
    /// `..`
    RangeSymbol,
    Dot,
    Comma,
    Semicolon,
    Colon,
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(Identifier { name }) => write!(f, "IDENTIFIER({name})"),
            Token::Keyword(keyword) => write!(f, "KEYWORD({keyword:?})"),
            Token::Literal(Literal::Integer(Ok(value))) => {
                write!(f, "INTEGER LITERAL({value})")
            }
            Token::Literal(Literal::Real(Ok(value))) => {
                write!(f, "REAL LITERAL({value})")
            }
            Token::Literal(Literal::Bool(value)) => {
                write!(f, "BOOLEAN LITERAL({value})")
            }
            Token::BuiltinTypename(builtin_typename) => {
                write!(f, "TYPENAME({builtin_typename:?})")
            }
            Token::Operator(operator) => write!(f, "OPERATOR({operator:?})"),
            Token::Comment(comment) => write!(f, "COMMENT({comment})"),
            Token::Literal(Literal::Integer(Err(e))) => {
                write!(f, "INVALID(Malformed integer literal: {e})")
            }
            Token::Literal(Literal::Real(Err(e))) => {
                write!(f, "INVALID(Malformed real literal: {e})")
            }
            Token::Unexpected(c) => write!(f, "INVALID(Unexpected character: {c:?})"),
            Token::LeftBracket => write!(f, "LEFT BRACKET"),
            Token::RightBracket => write!(f, "RIGHT BRACKET"),
            Token::LeftParenthesis => write!(f, "LEFT PARENTHESIS"),
            Token::RightParenthesis => write!(f, "RIGHT PARENTHESIS"),
            Token::RightArrow => write!(f, "FUNCTION ARROW"),
            Token::Assignment => write!(f, "ASSIGNMENT OPERATOR"),
            Token::RangeSymbol => write!(f, "RANGE"),
            Token::Dot => write!(f, "DOT"),
            Token::Comma => write!(f, "COMMA"),
            Token::Semicolon => write!(f, "SEMICOLON"),
            Token::Colon => write!(f, "COLON"),
            Token::Cast => write!(f, "CAST"),
        }
    }
}

// Token description
#[derive(Debug, Clone, PartialEq)]
pub struct Lexeme<'a> {
    pub extent: Extent,
    pub text: &'a str,
    pub token: Token<'a>,
}

impl fmt::Display for Lexeme<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            extent,
            text,
            token,
        } = self;
        write!(f, "{text:?} @ {extent} is {token}")
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum Operator {
    Plus,  // Either binary or unary one
    Minus, // Either binary or unary one
    Mul,
    Div,
    Mod,
    /// `=`
    Eq,
    /// `/=`
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `and`
    And,
    /// `or`
    Or,
    /// `xor`
    Xor,
    /// `not`
    Not,
}
