use core::fmt;

use crate::operators::SyntacticOperator;

// Token types

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
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
    In,
    While,
    For,
    Loop,
    Reverse,
    Print,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
pub struct Identifier<'a> {
    pub name: &'a str,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
pub struct IntegerLiteral {
    pub value: i64,
}

#[derive(PartialEq, fmt::Debug, Clone)]
pub struct RealLiteral {
    pub value: f64,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
pub struct BoolLiteral {
    pub value: bool,
}

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
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

#[derive(PartialEq, Clone)]
pub enum TokenKind<'a> {
    Identifier(Identifier<'a>),
    Keyword(Keyword),
    IntegerLiteral(IntegerLiteral),
    RealLiteral(RealLiteral),
    BoolLiteral(BoolLiteral),
    BuiltinTypename(BuiltinTypename),
    Operator(SyntacticOperator),
    Comment(Comment<'a>),
    LeftBracket,
    RightBracket,
    LeftParenthesis,
    RightParenthesis,
    RightArrow,
    Assignment,
    RangeSymbol,
    Dot,
    Comma,
    Semicolon,
    Colon,
}

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Identifier(identifier) => write!(f, "IDENTIFIER({})", identifier.name),
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
        }
    }
}

// Token description
pub struct Extent {
    pub start: usize,
    pub end: usize,
}

impl fmt::Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { start, end } = self;
        write!(f, "{start}:{end}")
    }
}

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
        write!(f, "`{lexeme}` @ {extent} is {kind}")
    }
}
