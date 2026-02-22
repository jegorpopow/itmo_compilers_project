use std::fmt;

use crate::operators::SyntacticOperator;

// Token types

#[derive(PartialEq, Eq, Hash, fmt::Debug, Clone)]
pub enum Keyword {
    Var,
    Type,
    Routine,
    Array,
    Record,
    Integer,
    Real,
    Boolean,
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
pub struct Identifier {
    pub name: String,
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
pub struct Comment {
    pub value: String,
}

impl Comment {
    fn shortened(&self) -> String {
        if self.value.len() <= 40 {
            self.value.to_owned()
        } else {
            self.value[0..40].to_owned() + " ..."
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum TokenValue {
    Identifier(Identifier),
    Keyword(Keyword),
    IntegerLiteral(IntegerLiteral),
    RealLiteral(RealLiteral),
    BoolLiteral(BoolLiteral),
    Operator(SyntacticOperator),
    Comment(Comment),
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

impl fmt::Display for TokenValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenValue::Identifier(identifier) => write!(f, "IDENTIFIER({})", identifier.name),
            TokenValue::Keyword(keyword) => write!(f, "KEYWORD({:?})", keyword),
            TokenValue::IntegerLiteral(integer_literal) => {
                write!(f, "INTEGER LITERAL({})", integer_literal.value)
            }
            TokenValue::RealLiteral(real_literal) => {
                write!(f, "REAL LITERAL({})", real_literal.value)
            }
            TokenValue::BoolLiteral(bool_literal) => {
                write!(f, "BOOLEAN LITERAL({})", bool_literal.value)
            }
            TokenValue::Operator(operator) => write!(f, "OPERATOR({:?})", operator),
            TokenValue::Comment(comment) => write!(f, "COMMENT({})", comment.shortened()),
            TokenValue::LeftBracket => write!(f, "LEFT BRACKET"),
            TokenValue::RightBracket => write!(f, "RIGHT BRACKET"),
            TokenValue::LeftParenthesis => write!(f, "LEFT PARENTHESIS"),
            TokenValue::RightParenthesis => write!(f, "RIGHT PARENTHESIS"),
            TokenValue::RightArrow => write!(f, "FUNCTION ARROW"),
            TokenValue::Assignment => write!(f, "ASSIGNMENT OPERATOR"),
            TokenValue::RangeSymbol => write!(f, "RANGE"),
            TokenValue::Dot => write!(f, "DOT"),
            TokenValue::Comma => write!(f, "COMMA"),
            TokenValue::Semicolon => write!(f, "SEMICOLON"),
            TokenValue::Colon => write!(f, "COLON"),
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
        write!(f, "{}:{}", self.start, self.end)
    }
}

pub struct Token {
    pub extent: Extent,
    pub lexeme: String,
    pub value: TokenValue,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` @ {} is {}", self.lexeme, self.extent, self.value)
    }
}

pub fn dump_tokens(tokens: &Vec<Token>) -> String {
    tokens
        .iter()
        .map(|token| token.to_string())
        .collect::<Vec<String>>()
        .join("\n")
}
