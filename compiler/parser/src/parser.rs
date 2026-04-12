//! Design largely inspired by `rustc_parse`.

use core::mem;

use common::Position;
use culpa::{throw, throws};
use lexer::{BuiltinTypename, Keyword, Lexeme, Operator, Token};

use crate::{Fatal, FinalError, FinalResult, ParsingError, Recoverable};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Like [`lexer::Token`], but flat and without data.
///
/// Idea stolen from rustc.
pub enum TokenKind {
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
    Identifier,
    Literal,
    Integer,
    Real,
    Boolean,
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
    Unexpected,
    EOF,
}

// TODO: this can be turned into a bitset
pub type Expected = std::collections::BTreeSet<TokenKind>;

#[expect(clippy::error_impl_error, reason = "for #[throws]")]
pub(crate) type Error = ParsingError<Fatal>;

impl From<Operator> for TokenKind {
    fn from(op: Operator) -> Self {
        match op {
            Operator::Plus => Self::Plus,
            Operator::Minus => Self::Minus,
            Operator::Mul => Self::Mul,
            Operator::Div => Self::Div,
            Operator::Mod => Self::Mod,
            Operator::Eq => Self::Eq,
            Operator::Ne => Self::Ne,
            Operator::Lt => Self::Lt,
            Operator::Le => Self::Le,
            Operator::Gt => Self::Gt,
            Operator::Ge => Self::Ge,
            Operator::And => Self::And,
            Operator::Or => Self::Or,
            Operator::Xor => Self::Xor,
            Operator::Not => Self::Not,
        }
    }
}

impl From<Keyword> for TokenKind {
    fn from(keyword: Keyword) -> Self {
        match keyword {
            Keyword::Var => Self::Var,
            Keyword::Type => Self::Type,
            Keyword::Routine => Self::Routine,
            Keyword::Array => Self::Array,
            Keyword::Record => Self::Record,
            Keyword::Is => Self::Is,
            Keyword::End => Self::End,
            Keyword::If => Self::If,
            Keyword::Then => Self::Then,
            Keyword::Else => Self::Else,
            Keyword::Return => Self::Return,
            Keyword::In => Self::In,
            Keyword::While => Self::While,
            Keyword::For => Self::For,
            Keyword::Loop => Self::Loop,
            Keyword::Reverse => Self::Reverse,
            Keyword::Print => Self::Print,
            Keyword::New => Self::New,
            Keyword::Null => Self::Null,
            Keyword::Where => Self::Where,
            Keyword::Assert => Self::Assert,
            Keyword::Panic => Self::Panic,
            Keyword::Constant => Self::Constant,
        }
    }
}

impl From<BuiltinTypename> for TokenKind {
    fn from(ty: BuiltinTypename) -> Self {
        match ty {
            BuiltinTypename::Integer => Self::Integer,
            BuiltinTypename::Real => Self::Real,
            BuiltinTypename::Boolean => Self::Boolean,
        }
    }
}

impl From<&Token<'_>> for TokenKind {
    fn from(token: &Token<'_>) -> Self {
        match token {
            Token::Identifier(_) => Self::Identifier,
            &Token::Keyword(keyword) => keyword.into(),
            Token::Literal(_) => Self::Literal,
            &Token::BuiltinTypename(ty) => ty.into(),
            &Token::Operator(op) => op.into(),
            Token::Comment(_) => unreachable!("We skip comments"),
            Token::Unexpected(_) => Self::Unexpected,
            Token::LeftBracket => Self::LeftBracket,
            Token::RightBracket => Self::RightBracket,
            Token::LeftParenthesis => Self::LeftParenthesis,
            Token::RightParenthesis => Self::RightParenthesis,
            Token::RightArrow => Self::RightArrow,
            Token::Cast => Self::Cast,
            Token::Assignment => Self::Assignment,
            Token::RangeSymbol => Self::RangeSymbol,
            Token::Dot => Self::Dot,
            Token::Comma => Self::Comma,
            Token::Semicolon => Self::Semicolon,
            Token::Colon => Self::Colon,
        }
    }
}

impl From<Token<'_>> for TokenKind {
    fn from(token: Token<'_>) -> Self {
        Self::from(&token)
    }
}

// FIXME(GrigorenkoPV): recover on invalid tokens
fn next_lexeme<'src>(
    lexer: &mut impl Iterator<Item = Lexeme<'src>>,
) -> Result<Lexeme<'src>, Position> {
    let mut eof_pos = Position::begin();
    for lexeme in lexer {
        eof_pos = lexeme.extent.end;
        if let Token::Comment(_) = lexeme.token {
            continue;
        }
        return Ok(lexeme);
    }
    Err(eof_pos)
}

#[derive(Debug)]
enum State<'src, Lexer> {
    InProgress { current: Lexeme<'src>, lexer: Lexer },
    EOF(Position),
}

impl<'src, I: Iterator<Item = Lexeme<'src>>> From<I> for State<'src, I> {
    fn from(mut lexer: I) -> Self {
        match next_lexeme(&mut lexer) {
            Ok(current) => Self::InProgress { current, lexer },
            Err(pos) => Self::EOF(pos),
        }
    }
}

impl<'src, I> State<'src, I> {
    #[must_use]
    fn current(&self) -> Option<&Lexeme<'src>> {
        match self {
            Self::InProgress { current, lexer: _ } => Some(current),
            Self::EOF(_) => None,
        }
    }

    #[must_use]
    fn current_kind(&self) -> TokenKind {
        match self.current() {
            Some(lexeme) => TokenKind::from(&lexeme.token),
            None => TokenKind::EOF,
        }
    }

    #[must_use]
    fn pos(&self) -> Position {
        match self {
            Self::InProgress { current, lexer: _ } => current.extent.start,
            Self::EOF(pos) => *pos,
        }
    }

    #[must_use]
    fn advance(&mut self) -> Option<Lexeme<'src>>
    where
        I: Iterator<Item = Lexeme<'src>>,
    {
        let prev = mem::replace(self, Self::EOF(self.pos()));
        match prev {
            Self::EOF(_) => None,
            Self::InProgress { current, lexer } => {
                *self = lexer.into();
                Some(current)
            }
        }
    }
}

#[derive(Debug)]
pub struct Parser<'src, Lexer> {
    state: State<'src, Lexer>,
    expected: Expected,
    recovered: Option<ParsingError<Recoverable>>,
}

impl<'src, I: Iterator<Item = Lexeme<'src>>> From<I> for Parser<'src, I> {
    fn from(lexer: I) -> Self {
        Self {
            state: lexer.into(),
            expected: Expected::new(),
            recovered: None,
        }
    }
}

impl<'src, I: Iterator<Item = Lexeme<'src>>> Parser<'src, I> {
    #[must_use]
    pub fn new(lexer: I) -> Self {
        lexer.into()
    }

    #[must_use]
    #[track_caller]
    pub(crate) fn check(&mut self, kind: impl Into<TokenKind>) -> bool {
        let kind = kind.into();
        let first_check: bool = self.expected.insert(kind);
        let is_present = self.state.current_kind() == kind;
        debug_assert!(
            !is_present || first_check,
            "Duplicate `check()` call for {kind:?} @ {}\n{:?}",
            self.state.pos(),
            self.expected
        );
        is_present
    }

    #[must_use]
    pub(crate) fn eof(&mut self) -> bool {
        self.check(TokenKind::EOF)
    }

    pub(crate) fn try_eat_lexeme(&mut self, kind: impl Into<TokenKind>) -> Option<Lexeme<'src>> {
        let kind = kind.into();
        assert_ne!(kind, TokenKind::EOF, "Cannot eat EOF");
        if self.state.current_kind() == kind {
            self.expected.clear();
            self.state.advance()
        } else {
            let _: bool = self.expected.insert(kind);
            None
        }
    }

    pub(crate) fn try_eat(&mut self, kind: impl Into<TokenKind>) -> bool {
        self.try_eat_lexeme(kind.into()).is_some()
    }

    #[throws]
    pub(crate) fn eat_lexeme(&mut self, kind: impl Into<TokenKind>) -> Lexeme<'src> {
        match self.try_eat_lexeme(kind.into()) {
            Some(l) => l,
            None => {
                let found = self.state.current_kind();
                let expected = self.expected.clone();
                debug_assert!(
                    !expected.contains(&found),
                    "Error message requested, but the next token is expected"
                );
                throw!(ParsingError {
                    position: self.state.pos(),
                    kind: Fatal::UnexpectedToken { found, expected },
                    previous: self.recovered.take().map(Box::new),
                })
            }
        }
    }

    #[throws]
    pub(crate) fn eat(&mut self, kind: impl Into<TokenKind>) {
        let _: Lexeme<'src> = self.eat_lexeme(kind)?;
    }

    pub(crate) fn recovered(&mut self, error: Recoverable, position: Position) {
        let previous = self.recovered.take().map(Box::new);
        self.recovered = Some(ParsingError {
            position,
            kind: error,
            previous,
        });
    }

    pub fn finish<T>(mut self, parsed: Result<T, ParsingError<Fatal>>) -> FinalResult<T> {
        if let Some(lexeme) = self.state.advance() {
            self.recovered(
                Recoverable::NotEOF(lexeme.token.into()),
                lexeme.extent.start,
            )
        }

        match parsed {
            Err(fatal) => Err(FinalError::Failed(fatal)),
            Ok(parsed) => match self.recovered {
                None => Ok(parsed),
                Some(last) => Err(FinalError::ParsedWithError { last, parsed }),
            },
        }
    }
}
