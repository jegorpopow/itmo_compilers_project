use core::{
    error::Error,
    fmt,
    num::{ParseFloatError, ParseIntError},
};

use common::Position;

use crate::parser::{Expected, TokenKind};

impl From<TokenKind> for &'static str {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Identifier => "an identifier",
            TokenKind::Literal => "a literal",
            TokenKind::Var => "`var`",
            TokenKind::Type => "`type`",
            TokenKind::Routine => "`routine`",
            TokenKind::Array => "`array`",
            TokenKind::Record => "`record`",
            TokenKind::Is => "`is`",
            TokenKind::End => "`end`",
            TokenKind::If => "`if`",
            TokenKind::Then => "`then`",
            TokenKind::Else => "`else`",
            TokenKind::Return => "`return`",
            TokenKind::In => "`in`",
            TokenKind::While => "`while`",
            TokenKind::For => "`for`",
            TokenKind::Loop => "`loop`",
            TokenKind::Reverse => "`reverse`",
            TokenKind::Print => "`print`",
            TokenKind::New => "`new`",
            TokenKind::Null => "`null`",
            TokenKind::Where => "`where`",
            TokenKind::Assert => "`assert`",
            TokenKind::Panic => "`panic`",
            TokenKind::Constant => "`constant`",
            TokenKind::Integer => "`integer`",
            TokenKind::Real => "`real`",
            TokenKind::Boolean => "`boolean`",
            TokenKind::LeftBracket => "`[`",
            TokenKind::RightBracket => "`]`",
            TokenKind::LeftParenthesis => "`(`",
            TokenKind::RightParenthesis => "`)`",
            TokenKind::RightArrow => "`=>`",
            TokenKind::Cast => "`::`",
            TokenKind::Assignment => "`:=`",
            TokenKind::RangeSymbol => "`..`",
            TokenKind::Dot => "`.`",
            TokenKind::Comma => "`,`",
            TokenKind::Colon => "`:`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Mul => "`*`",
            TokenKind::Div => "`/`",
            TokenKind::Mod => "`%`",
            TokenKind::Eq => "`=`",
            TokenKind::Ne => "`/=`",
            TokenKind::Lt => "`<`",
            TokenKind::Le => "`<=`",
            TokenKind::Gt => "`>`",
            TokenKind::Ge => "`>=`",
            TokenKind::And => "`and`",
            TokenKind::Or => "`or`",
            TokenKind::Xor => "`xor`",
            TokenKind::Not => "`not`",
            TokenKind::Semicolon => "`;`",
            TokenKind::EOF => "EOF",
            TokenKind::Unexpected => "an unexpected character",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).into())
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct ParsingError<K> {
    pub position: Position,
    pub kind: K,
    pub previous: Option<Box<ParsingError<Recoverable>>>,
}

impl<K: fmt::Display> fmt::Display for ParsingError<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            position,
            previous,
        } = self;
        write!(f, "at {position}: {kind}")?;
        if let Some(previous) = previous {
            write!(f, ";\nthis might be caused by an error {previous}")?
        }
        Ok(())
    }
}

impl<K: Error> Error for ParsingError<K> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.previous.as_deref() {
            Some(e) => Some(e),
            None => self.kind.source(),
        }
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub enum Recoverable {
    NotEOF(TokenKind),
    MalformedReal(ParseFloatError),
    MalformedInteger(ParseIntError),
}

impl fmt::Display for Recoverable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotEOF(found) => {
                debug_assert_ne!(
                    found,
                    &TokenKind::EOF,
                    "Complaining about finding EOF instead of EOF?"
                );
                write!(f, "expected EOF, but found {found}")
            }
            Self::MalformedReal(error) => write!(f, "malformed real literal: {error}"),
            Self::MalformedInteger(error) => write!(f, "malformed integer literal: {error}"),
        }
    }
}

impl Error for Recoverable {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NotEOF(_) => None,
            Self::MalformedReal(e) => Some(e),
            Self::MalformedInteger(e) => Some(e),
        }
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub enum Fatal {
    UnexpectedToken {
        found: TokenKind,
        expected: Expected,
    },
}

impl fmt::Display for Fatal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken { found, expected } => {
                write!(f, "expected ")?;

                let or_after = expected.len().checked_sub(2);
                for (i, &expected) in expected.iter().enumerate() {
                    write!(
                        f,
                        "{expected}{} ",
                        if or_after == Some(i) { " or" } else { "," }
                    )?
                }
                write!(f, "but found {found}")
            }
        }
    }
}

impl Error for Fatal {}

#[derive(Debug)]
pub enum FinalError<T> {
    ParsedWithError {
        last: ParsingError<Recoverable>,
        parsed: T,
    },
    Failed(ParsingError<Fatal>),
}

impl<T: fmt::Debug> fmt::Display for FinalError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(fatal) => write!(f, "Parsing error {fatal}."),
            Self::ParsedWithError { last, parsed } => {
                write!(
                    f,
                    "Parsing error {last}.\nHowever, managed to parse:\n{parsed:#?}"
                )
            }
        }
    }
}

impl<T> Error for FinalError<T>
where
    T: fmt::Debug,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::ParsedWithError { last, parsed: _ } => last,
            Self::Failed(e) => e,
        })
    }
}
pub type FinalResult<T> = Result<T, FinalError<T>>;
