use core::fmt;

use phf::phf_map;

use crate::operators::SyntacticOperator;
use crate::tokens::*;

#[cfg(test)]
mod tests;

trait ImmutableIterator<'a>: Sized + Clone {
    fn from_index(string: &'a str, n: usize) -> Self;
    fn slice_to_str(start: &Self, end: &Self) -> &'a str;
    fn is_end(&self) -> bool;
    fn next(&self) -> Option<(char, Self)>;

    fn from_beginning(string: &'a str) -> Self {
        Self::from_index(string, 0)
    }

    fn lookup(&self) -> Option<char> {
        self.next().map(|(ch, _)| ch)
    }

    fn skip(&self, predicate: impl Fn(char) -> bool) -> Self {
        let mut copy = self.clone();

        while let Some((ch, rest)) = copy.next() {
            if !predicate(ch) {
                break;
            }
            copy = rest;
        }
        copy
    }

    fn skip_n(&self, n: usize) -> Option<Self> {
        let mut copy = self.clone();

        for _ in 0..n {
            if let Some((_, next)) = copy.next() {
                copy = next;
            } else {
                return None;
            }
        }

        Some(copy)
    }

    fn take_while(&self, predicate: impl Fn(char) -> bool) -> (&'a str, Self) {
        let mut copy = self.clone();
        let mut result = String::new();

        while let Some((ch, rest)) = copy.next() {
            if !predicate(ch) {
                break;
            }
            result.push(ch);
            copy = rest;
        }

        (Self::slice_to_str(self, &copy), copy)
    }

    fn take_while_map<T>(
        &self,
        predicate: impl Fn(char) -> bool,
        map: impl FnOnce(&'a str) -> T,
    ) -> (T, Self) {
        let (s, it) = self.take_while(predicate);
        (map(s), it)
    }

    fn stars_with(&self, value: &str) -> Option<Self> {
        let expected = value.chars();
        let mut copy = self.clone();

        for expected_char in expected {
            if let Some((ch, rest)) = copy.next() {
                if ch != expected_char {
                    return None;
                }
                copy = rest;
            } else {
                return None;
            }
        }

        Some(copy)
    }
}

// TODO: rewrite with Chars<'a> and its .clone() method
#[derive(Clone)]
struct IndexIterator<'a> {
    underlying: &'a str,
    index: usize,
}

impl<'a> ImmutableIterator<'a> for IndexIterator<'a> {
    fn from_index(string: &'a str, n: usize) -> Self {
        IndexIterator {
            underlying: string,
            index: string.char_indices().nth(n).map(|(idx, _)| idx).unwrap(),
        }
    }

    fn slice_to_str(start: &Self, end: &Self) -> &'a str {
        assert_eq!(start.underlying.as_ptr(), end.underlying.as_ptr());
        &start.underlying[start.index..end.index]
    }

    fn is_end(&self) -> bool {
        self.index >= self.underlying.len()
    }

    fn next(&self) -> Option<(char, Self)> {
        self.underlying[self.index..].chars().next().map(|ch| {
            (
                ch,
                IndexIterator {
                    underlying: self.underlying,
                    index: self.index + ch.len_utf8(),
                },
            )
        })
    }
}

#[allow(clippy::tests_outside_test_module)]
#[test]
fn identifier_start_is_identifier_continue() {
    for c in char::MIN..=char::MAX {
        assert!(
            is_identifier_start(c) <= is_identifier_continue(c),
            "Identifier start should be identifier continue: {c:?}"
        )
    }
}

/// Unicode does not like `'` or `_`. We do.
const fn extra_ident_char(c: char) -> bool {
    matches!(c, '\'' | '_')
}

fn is_identifier_start(c: char) -> bool {
    ::unicode_ident::is_xid_start(c) | extra_ident_char(c)
}

fn is_identifier_continue(c: char) -> bool {
    ::unicode_ident::is_xid_continue(c) | extra_ident_char(c)
}

fn iterators_to_extent(start: &IndexIterator<'_>, end: &IndexIterator<'_>) -> Extent {
    Extent {
        start: start.index,
        end: end.index,
    }
}

/// Processes all the identifier-like lexemes (identifiers, keywords, bool literals and some operators)
fn possible_identifier_value(lexeme: &str) -> TokenKind<'_> {
    static KNOWN_TOKENS: phf::Map<&str, TokenKind<'static>> = phf_map! {
        "var" => TokenKind::Keyword(Keyword::Var),
        "type" => TokenKind::Keyword(Keyword::Type),
        "routine" => TokenKind::Keyword(Keyword::Routine),
        "array" => TokenKind::Keyword(Keyword::Array),
        "record" => TokenKind::Keyword(Keyword::Record),
        "is" => TokenKind::Keyword(Keyword::Is),
        "end" => TokenKind::Keyword(Keyword::End),
        "if" => TokenKind::Keyword(Keyword::If),
        "then" => TokenKind::Keyword(Keyword::Then),
        "else" => TokenKind::Keyword(Keyword::Else),
        "in" => TokenKind::Keyword(Keyword::In),
        "while" => TokenKind::Keyword(Keyword::While),
        "for" => TokenKind::Keyword(Keyword::For),
        "loop" => TokenKind::Keyword(Keyword::Loop),
        "reverse" => TokenKind::Keyword(Keyword::Reverse),
        "and" => TokenKind::Operator(SyntacticOperator::And),
        "or" => TokenKind::Operator(SyntacticOperator::Or),
        "xor" => TokenKind::Operator(SyntacticOperator::Xor),
        "not" => TokenKind::Operator(SyntacticOperator::Neg),
        "true" => TokenKind::BoolLiteral(BoolLiteral { value: true }),
        "false" => TokenKind::BoolLiteral(BoolLiteral { value: false }),
        "integer" => TokenKind::BuiltinTypename(BuiltinTypename::Integer),
        "real" => TokenKind::BuiltinTypename(BuiltinTypename::Real),
        "boolean" => TokenKind::BuiltinTypename(BuiltinTypename::Boolean),
        "NaN" => TokenKind::RealLiteral(RealLiteral { value: f64::NAN }),
    };

    // TODO: add more cases (NaN, Infinity, ...)
    match KNOWN_TOKENS.get(lexeme) {
        Some(token_value) => token_value.clone(),
        None => TokenKind::Identifier(Identifier { name: lexeme }),
    }
}

fn known_symbolic_tokens<'a>(
    start: &IndexIterator<'a>,
) -> Option<(TokenKind<'a>, IndexIterator<'a>)> {
    static KNOWN_TOKENS: &[(&str, TokenKind<'static>)] = &[
        (":=", TokenKind::Assignment),
        ("..", TokenKind::RangeSymbol),
        ("/=", TokenKind::Operator(SyntacticOperator::Neq)),
        ("<=", TokenKind::Operator(SyntacticOperator::Le)),
        (">=", TokenKind::Operator(SyntacticOperator::Ge)),
        ("(", TokenKind::LeftParenthesis),
        (")", TokenKind::RightParenthesis),
        ("[", TokenKind::LeftBracket),
        ("]", TokenKind::RightBracket),
        (",", TokenKind::Comma),
        (".", TokenKind::Dot),
        (";", TokenKind::Semicolon),
        (":", TokenKind::Colon),
        ("+", TokenKind::Operator(SyntacticOperator::Add)),
        ("-", TokenKind::Operator(SyntacticOperator::Sub)),
        ("*", TokenKind::Operator(SyntacticOperator::Mul)),
        ("/", TokenKind::Operator(SyntacticOperator::Div)),
        ("%", TokenKind::Operator(SyntacticOperator::Mod)),
        ("=", TokenKind::Operator(SyntacticOperator::Eq)),
        ("<", TokenKind::Operator(SyntacticOperator::Lt)),
        (">", TokenKind::Operator(SyntacticOperator::Gt)),
    ];

    for (pattern, token) in KNOWN_TOKENS {
        if let Some(end) = start.stars_with(pattern) {
            return Some((token.clone(), end));
        }
    }

    None
}

#[derive(Debug)]
pub struct LexerError {
    pub position: usize,
    pub reason: String,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { position, reason } = self;
        write!(f, "Lexing error byte offset {position}: {reason}")
    }
}

impl core::error::Error for LexerError {}

type LexerResult<T> = Result<T, LexerError>;

fn real_literal_from_representation(s: &str, position: usize) -> LexerResult<TokenKind<'_>> {
    Ok(TokenKind::RealLiteral(RealLiteral {
        value: s.parse().map_err(|e| LexerError {
            position,
            reason: format!("Malformed float {s:?}: {e}"),
        })?,
    }))
}

fn integer_literal_from_representation(s: &str, position: usize) -> LexerResult<TokenKind<'_>> {
    Ok(TokenKind::IntegerLiteral(IntegerLiteral {
        value: s.parse().map_err(|e| LexerError {
            position,
            reason: format!("Malformed int {s:?}: {e}"),
        })?,
    }))
}

fn should_expect_sign(prev: bool, token: &TokenKind<'_>) -> bool {
    match token {
        TokenKind::Comment(_) => prev,

        TokenKind::Assignment
        | TokenKind::LeftParenthesis
        | TokenKind::RightBracket
        | TokenKind::Operator(_)
        | TokenKind::Semicolon
        | TokenKind::RangeSymbol
        | TokenKind::Comma
        | TokenKind::RightArrow
        | TokenKind::Keyword(_) => true,

        TokenKind::Identifier(_)
        | TokenKind::IntegerLiteral(_)
        | TokenKind::RealLiteral(_)
        | TokenKind::BoolLiteral(_)
        | TokenKind::BuiltinTypename(_)
        | TokenKind::LeftBracket
        | TokenKind::RightParenthesis
        | TokenKind::Dot
        | TokenKind::Colon => false,
    }
}

fn numerical_tokens<'a>(
    expect_sign: bool,
    begin: &IndexIterator<'a>,
) -> Option<LexerResult<(TokenKind<'a>, IndexIterator<'a>)>> {
    let mut start = begin.clone();

    // Numerical literal may start with a sign
    if expect_sign && start.lookup().is_some_and(|ch| ch == '-' || ch == '+') {
        start = start.skip_n(1).unwrap();
    }

    let (before_point, start) = start.take_while(|ch| ch.is_ascii_digit());

    if start.lookup().is_some_and(|ch| ch == '.') {
        let start = start.skip_n(1).unwrap();
        let (after_point, rest) = start.take_while(|ch| ch.is_ascii_digit());
        if after_point.is_empty() {
            None
        } else {
            Some(
                real_literal_from_representation(
                    ImmutableIterator::slice_to_str(begin, &rest),
                    begin.index,
                )
                .map(|token_value| (token_value, rest)),
            )
        }
    } else if before_point.is_empty() {
        None
    } else {
        Some(
            integer_literal_from_representation(
                ImmutableIterator::slice_to_str(begin, &start),
                begin.index,
            )
            .map(|token_value| (token_value, start)),
        )
    }
}

pub fn tokenize(source: &str) -> LexerResult<Vec<Token<'_>>> {
    let mut begin = IndexIterator::from_beginning(source);
    let mut result = Vec::new();
    let mut expects_sign = true;

    while let Some(first_char) = begin.lookup() {
        if first_char.is_whitespace() {
            begin = begin.skip(char::is_whitespace);
        } else if let Some(comment_start) = begin.stars_with("--") {
            let (kind, end) = comment_start.take_while_map(
                |ch| ch != '\n',
                |value| TokenKind::Comment(Comment { value }),
            );
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_str(&begin, &end),
                kind,
            });
            begin = end;
        } else if is_identifier_start(first_char) {
            let (kind, end) =
                begin.take_while_map(is_identifier_continue, possible_identifier_value);
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_str(&begin, &end),
                kind,
            });
            begin = end;
        } else if let Some(res) = numerical_tokens(expects_sign, &begin) {
            let (kind, end) = res?;
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_str(&begin, &end),
                kind,
            });
            begin = end;
        } else if let Some((kind, end)) = known_symbolic_tokens(&begin) {
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_str(&begin, &end),
                kind,
            });
            begin = end;
        } else {
            return Err(LexerError {
                position: begin.index,
                reason: format!("Unexpected symbol `{first_char}`"),
            });
        }

        expects_sign = should_expect_sign(expects_sign, &result.last().unwrap().kind);
    }

    Ok(result)
}
