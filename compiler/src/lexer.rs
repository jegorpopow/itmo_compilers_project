use core::{num, ptr};
use std::collections::HashMap;

use phf::phf_map;

use crate::operators::SyntacticOperator;
use crate::tokens::*;

#[cfg(test)]
mod tests;

trait ImmutableIterator<'a>: Sized + Clone {
    fn from_index(string: &'a str, n: usize) -> Self;
    fn slice_to_string(start: &Self, end: &Self) -> String;
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

        for i in 0..n {
            if let Some((_, next)) = copy.next() {
                copy = next;
            } else {
                return None;
            }
        }

        Some(copy)
    }

    fn take_while(&self, predicate: impl Fn(char) -> bool) -> (String, Self) {
        let mut copy = self.clone();
        let mut result = String::new();

        while let Some((ch, rest)) = copy.next() {
            if !predicate(ch) {
                break;
            }
            result.push(ch);
            copy = rest;
        }
        (Self::slice_to_string(self, &copy), copy)
    }

    fn stars_with(&self, value: &str) -> Option<Self> {
        let mut expected = value.chars();
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
#[derive(Clone, Copy)]
struct IndexIterator<'a> {
    underlying: &'a str,
    index: usize,
}

impl<'a> ImmutableIterator<'a> for IndexIterator<'a> {
    fn from_index(string: &'a str, n: usize) -> IndexIterator<'a> {
        IndexIterator {
            underlying: string,
            index: string.char_indices().nth(n).map(|(idx, _)| idx).unwrap(),
        }
    }

    fn slice_to_string(start: &IndexIterator<'_>, end: &IndexIterator<'_>) -> String {
        assert_eq!(start.underlying.as_ptr(), end.underlying.as_ptr());
        start.underlying[start.index..end.index].to_owned()
    }

    fn is_end(&self) -> bool {
        self.index >= self.underlying.len()
    }

    fn next(&self) -> Option<(char, IndexIterator<'a>)> {
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

#[test]
fn identifier_start_is_identifier_continue() {
    for c in char::MIN..=char::MAX {
        assert!(
            is_identifier_start(c) <= is_identifier_continue(c),
            "Identifier start should be identifier continue: {c:?}"
        )
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '\''
}

fn iterators_to_extent(start: &IndexIterator<'_>, end: &IndexIterator<'_>) -> Extent {
    Extent {
        start: start.index,
        end: end.index,
    }
}

/// Processes all the identifier-like lexemes (identifiers, keywords, bool literals and some operators)
fn possible_identifier_value(lexeme: &str) -> TokenKind {
    static KNOWN_TOKENS: phf::Map<&str, TokenKind> = phf_map! {
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
        None => TokenKind::Identifier(Identifier {
            name: lexeme.to_owned(),
        }),
    }
}

fn known_symbolic_tokens(start: IndexIterator<'_>) -> Option<(TokenKind, IndexIterator<'_>)> {
    static KNOWN_TOKENS: &[(&str, TokenKind)] = &[
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

fn real_literal_from_representation(float_str: &str) -> TokenKind {
    // TODO: process parsing error
    TokenKind::RealLiteral(RealLiteral {
        value: float_str.parse().unwrap(),
    })
}

fn integer_literal_from_representation(int_str: &str) -> TokenKind {
    // TODO: process parsing error
    TokenKind::IntegerLiteral(IntegerLiteral {
        value: int_str.parse().unwrap(),
    })
}

fn should_expect_sign(prev: bool, token: &TokenKind) -> bool {
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

fn numerical_tokens(
    expect_sign: bool,
    begin: IndexIterator<'_>,
) -> Option<(TokenKind, IndexIterator<'_>)> {
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
            let representation = ImmutableIterator::slice_to_string(&begin, &rest);
            let token_value = real_literal_from_representation(&representation);
            Some((token_value, rest))
        }
    } else if before_point.is_empty() {
        None
    } else {
        let representation = ImmutableIterator::slice_to_string(&begin, &start);
        let token_value = integer_literal_from_representation(&representation);
        Some((token_value, start))
    }
}

#[derive(Debug)]
pub struct LexerError {
    pub position: usize,
    pub reason: String,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    let mut begin = IndexIterator::from_beginning(source);
    let mut result = Vec::new();
    let mut expects_sign = true;

    while let Some(first_char) = begin.lookup() {
        if (first_char.is_whitespace()) {
            begin = begin.skip(char::is_whitespace);
        } else if let Some(comment_start) = begin.stars_with("--") {
            let (comment, end) = comment_start.take_while(|ch| ch != '\n');
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                kind: TokenKind::Comment(Comment { value: comment }),
            });
            begin = end;
        } else if is_identifier_start(first_char) {
            let (possible_identifier, end) = begin.take_while(is_identifier_continue);
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                kind: possible_identifier_value(&possible_identifier),
            });
            begin = end;
        } else if let Some((numerical_token, end)) = numerical_tokens(expects_sign, begin) {
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                kind: numerical_token,
            });
            begin = end;
        } else if let Some((token_value, end)) = known_symbolic_tokens(begin) {
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                kind: token_value,
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
