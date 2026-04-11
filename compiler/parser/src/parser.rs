//! Design largely inspired by `rustc_parse`.

use core::{fmt, mem};
use std::collections::BTreeSet;

use common::Position;
use lexer::{BuiltinTypename, Keyword, Lexeme, Operator, Token};

use crate::{ParserError, ParsingError, ParsingResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Like [`lexer::Token`], but flat and without data.
/// TODO: this can be turned into a bitflag
///
/// Idea stolen from rustc.
pub(crate) enum TokenKind {
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
    Invalid,
    EOF,
}

impl From<TokenKind> for &'static str {
    fn from(kind: TokenKind) -> Self {
        match kind {
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
            TokenKind::Identifier => "an identifier",
            TokenKind::Literal => "a literal",
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
            TokenKind::Invalid => "an invalid token",
            TokenKind::EOF => "EOF",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).into())
    }
}

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
            Token::Invalid(_) => Self::Invalid,
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
    expected: BTreeSet<TokenKind>,
    recovered: Vec<ParsingError>,
}

impl<'src, I: Iterator<Item = Lexeme<'src>>> From<I> for Parser<'src, I> {
    fn from(lexer: I) -> Self {
        Self {
            state: lexer.into(),
            expected: BTreeSet::new(),
            recovered: vec![],
        }
    }
}

fn format_expected(out: &mut String, expected: &BTreeSet<TokenKind>) {
    let or_after = expected.len().checked_sub(2);
    for (i, &expected) in expected.iter().enumerate() {
        out.push_str(expected.into());
        out.push_str(if or_after == Some(i) { " or " } else { ", " });
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
            !is_present || first_check || kind == TokenKind::EOF,
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

    fn error(&self) -> ParsingError {
        debug_assert!(
            !self.expected.contains(&self.state.current_kind()),
            "Error message requested, but the next token is expected"
        );

        let mut what = "Expected ".to_string();
        format_expected(&mut what, &self.expected);
        what.push_str("but found ");

        let position = self.state.pos();
        let found = self.state.current();

        what.push_str(match found {
            Some(lexeme) => TokenKind::from(&lexeme.token).into(),
            None => "EOF",
        });

        ParsingError { what, position }
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

    pub(crate) fn eat_lexeme(&mut self, kind: impl Into<TokenKind>) -> ParsingResult<Lexeme<'src>> {
        self.try_eat_lexeme(kind.into()).ok_or_else(|| self.error())
    }

    pub(crate) fn eat(&mut self, kind: impl Into<TokenKind>) -> ParsingResult<()> {
        self.eat_lexeme(kind).map(drop::<Lexeme<'src>>)
    }

    pub fn finish<T>(mut self, parsed: T) -> crate::ParserResult<T> {
        if !self.eof() {
            Err(self.error())?
        }
        let Self { recovered, .. } = self;
        if recovered.is_empty() {
            Ok(parsed)
        } else {
            Err(ParserError::Recoverable {
                errors: recovered,
                parsed,
            })
        }
    }
}
