use core::num::{ParseFloatError, ParseIntError};
use core::{error, fmt};

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

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone, Copy)]
pub struct IntegerLiteral {
    pub value: Integer,
}

#[derive(PartialEq, fmt::Debug, Clone, Copy)]
pub struct RealLiteral {
    pub value: Real,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone, Copy)]
pub struct BoolLiteral {
    pub value: bool,
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

#[derive(PartialEq, Eq, fmt::Debug, Clone)]
pub enum InvalidToken {
    Unexpected(char),
    MalformedInteger(ParseIntError),
    MalformedReal(ParseFloatError),
}

impl error::Error for InvalidToken {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unexpected(_) => None,
            Self::MalformedInteger(e) => Some(e),
            Self::MalformedReal(e) => Some(e),
        }
    }
}

impl fmt::Display for InvalidToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected(c) => write!(f, "Unexpected character: {c:?}"),
            Self::MalformedInteger(e) => write!(f, "Malformed integer literal: {e}"),
            Self::MalformedReal(e) => write!(f, "Malformed real literal: {e}"),
        }
    }
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
pub enum TokenKind<'a> {
    Identifier(Identifier<'a>),
    Keyword(Keyword),
    IntegerLiteral(IntegerLiteral),
    RealLiteral(RealLiteral),
    BoolLiteral(BoolLiteral),
    BuiltinTypename(BuiltinTypename),
    Operator(Operator),
    Comment(Comment<'a>),
    Invalid(InvalidToken),
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

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Identifier(Identifier { name }) => write!(f, "IDENTIFIER({name})"),
            TokenKind::Keyword(keyword) => write!(f, "KEYWORD({keyword:?})"),
            TokenKind::IntegerLiteral(IntegerLiteral { value }) => {
                write!(f, "INTEGER LITERAL({value})")
            }
            TokenKind::RealLiteral(RealLiteral { value }) => {
                write!(f, "REAL LITERAL({value})")
            }
            TokenKind::BoolLiteral(BoolLiteral { value }) => {
                write!(f, "BOOLEAN LITERAL({value})")
            }
            TokenKind::BuiltinTypename(builtin_typename) => {
                write!(f, "TYPENAME({builtin_typename:?})")
            }
            TokenKind::Operator(operator) => write!(f, "OPERATOR({operator:?})"),
            TokenKind::Comment(comment) => write!(f, "COMMENT({comment})"),
            TokenKind::Invalid(problem) => write!(f, "INVALID({problem})"),
            TokenKind::LeftBracket => write!(f, "LEFT BRACKET"),
            TokenKind::RightBracket => write!(f, "RIGHT BRACKET"),
            TokenKind::LeftParenthesis => write!(f, "LEFT PARENTHESIS"),
            TokenKind::RightParenthesis => write!(f, "RIGHT PARENTHESIS"),
            TokenKind::RightArrow => write!(f, "FUNCTION ARROW"),
            TokenKind::Assignment => write!(f, "ASSIGNMENT OPERATOR"),
            TokenKind::RangeSymbol => write!(f, "RANGE"),
            TokenKind::Dot => write!(f, "DOT"),
            TokenKind::Comma => write!(f, "COMMA"),
            TokenKind::Semicolon => write!(f, "SEMICOLON"),
            TokenKind::Colon => write!(f, "COLON"),
            TokenKind::Cast => write!(f, "CAST"),
        }
    }
}

// Token description
#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    pub extent: Extent,
    pub lexeme: &'a str,
    pub kind: TokenKind<'a>,
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            extent,
            lexeme,
            kind,
        } = self;
        write!(f, "{lexeme:?} @ {extent} is {kind}")
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
