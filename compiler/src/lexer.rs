use core::ptr;
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
        (result, copy)
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

// Assume forall(x) is_identifier_start(x) -> is_identifier_continue(x)

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
fn possible_identifier_value(lexeme: &str) -> TokenValue {
    static KNOWN_TOKENS: phf::Map<&str, TokenValue> = phf_map! {
        "var" => TokenValue::Keyword(Keyword::Var),
        "type" => TokenValue::Keyword(Keyword::Type),
        "routine" => TokenValue::Keyword(Keyword::Routine),
        "array" => TokenValue::Keyword(Keyword::Array),
        "record" => TokenValue::Keyword(Keyword::Record),
        "is" => TokenValue::Keyword(Keyword::Is),
        "end" => TokenValue::Keyword(Keyword::End),
        "if" => TokenValue::Keyword(Keyword::If),
        "then" => TokenValue::Keyword(Keyword::Then),
        "else" => TokenValue::Keyword(Keyword::Else),
        "in" => TokenValue::Keyword(Keyword::In),
        "while" => TokenValue::Keyword(Keyword::While),
        "for" => TokenValue::Keyword(Keyword::For),
        "loop" => TokenValue::Keyword(Keyword::Loop),
        "reverse" => TokenValue::Keyword(Keyword::Reverse),
        "and" => TokenValue::Operator(SyntacticOperator::And),
        "or" => TokenValue::Operator(SyntacticOperator::Or),
        "xor" => TokenValue::Operator(SyntacticOperator::Xor),
        "not" => TokenValue::Operator(SyntacticOperator::Neg),
        "true" => TokenValue::BoolLiteral(BoolLiteral { value: true }),
        "false" => TokenValue::BoolLiteral(BoolLiteral { value: false }),
        "integer" => TokenValue::BuiltinTypename(BuiltinTypename::Integer),
        "real" => TokenValue::BuiltinTypename(BuiltinTypename::Real),
        "boolean" => TokenValue::BuiltinTypename(BuiltinTypename::Boolean),
        "NaN" => TokenValue::RealLiteral(RealLiteral { value: f64::NAN }),
    };

    // TODO: add more cases (NaN, Infinity, ...)
    match KNOWN_TOKENS.get(lexeme) {
        Some(token_value) => token_value.clone(),
        None => TokenValue::Identifier(Identifier {
            name: lexeme.to_owned(),
        }),
    }
}

fn known_symbolic_tokens(start: IndexIterator<'_>) -> Option<(TokenValue, IndexIterator<'_>)> {
    static KNOWN_TOKENS: &[(&str, TokenValue)] = &[
        (":=", TokenValue::Assignment),
        ("..", TokenValue::RangeSymbol),
        ("/=", TokenValue::Operator(SyntacticOperator::Neq)),
        ("<=", TokenValue::Operator(SyntacticOperator::Le)),
        (">=", TokenValue::Operator(SyntacticOperator::Ge)),
        ("(", TokenValue::LeftParenthesis),
        (")", TokenValue::RightParenthesis),
        ("[", TokenValue::LeftBracket),
        ("]", TokenValue::RightBracket),
        (",", TokenValue::Comma),
        (".", TokenValue::Dot),
        (";", TokenValue::Semicolon),
        (":", TokenValue::Colon),
        ("+", TokenValue::Operator(SyntacticOperator::Add)),
        ("-", TokenValue::Operator(SyntacticOperator::Sub)),
        ("*", TokenValue::Operator(SyntacticOperator::Mul)),
        ("/", TokenValue::Operator(SyntacticOperator::Div)),
        ("%", TokenValue::Operator(SyntacticOperator::Mod)),
        ("=", TokenValue::Operator(SyntacticOperator::Eq)),
        ("<", TokenValue::Operator(SyntacticOperator::Lt)),
        (">", TokenValue::Operator(SyntacticOperator::Gt)),
    ];

    for (pattern, token) in KNOWN_TOKENS {
        if let Some(end) = start.stars_with(pattern) {
            return Some((token.clone(), end));
        }
    }

    None
}

fn real_literal_from_representation(float_str: &str) -> TokenValue {
    TokenValue::RealLiteral(RealLiteral {
        value: float_str.parse().unwrap(),
    })
}

fn integer_literal_from_representation(int_str: &str) -> TokenValue {
    TokenValue::IntegerLiteral(IntegerLiteral {
        value: int_str.parse().unwrap(),
    })
}

#[derive(Debug)]
pub struct LexerError {
    pub position: usize,
    pub reason: String,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    let mut begin = IndexIterator::from_beginning(source);
    let mut result = Vec::new();

    while let Some(first_char) = begin.lookup() {
        if (first_char.is_whitespace()) {
            begin = begin.skip(char::is_whitespace);
        } else if let Some(comment_start) = begin.stars_with("--") {
            let (comment, end) = begin.take_while(|ch| ch != '\n');
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                value: TokenValue::Comment(Comment { value: comment }),
            });
            begin = end;
        } else if is_identifier_start(first_char) {
            let (possible_identifier, end) = begin.take_while(is_identifier_continue);
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                value: possible_identifier_value(&possible_identifier),
            });
            begin = end;
        } else if first_char.is_ascii_digit() {
            // Either real or integer literal
            let (digits, rest) = begin.take_while(|ch| ch.is_ascii_digit());

            if let Some('.') = rest.lookup() {
                let rest = rest.skip_n(1).unwrap();
                let (_, end) = begin.take_while(|ch| ch.is_ascii_digit());
                let representation = ImmutableIterator::slice_to_string(&begin, &end);
                let token_value = real_literal_from_representation(&representation);
                result.push(Token {
                    extent: iterators_to_extent(&begin, &end),
                    lexeme: representation,
                    value: token_value,
                });
                begin = end;
            } else {
                let representation = ImmutableIterator::slice_to_string(&begin, &rest);
                let token_value = integer_literal_from_representation(&representation);
                result.push(Token {
                    extent: iterators_to_extent(&begin, &rest),
                    lexeme: representation,
                    value: token_value,
                });
                begin = rest;
            }
        } else if first_char == '.' {
            // Either member access or float literal
            match begin.next() {
                None => {
                    return Err(LexerError {
                        position: begin.index + 1,
                        reason: "Dot can not be the last character of program".to_owned(),
                    });
                }
                Some((second_char, rest)) => {
                    if second_char.is_ascii_digit() {
                        let (_, end) = rest.take_while(|ch| ch.is_ascii_digit());
                        let representation = ImmutableIterator::slice_to_string(&begin, &end);
                        let token_value = real_literal_from_representation(&representation);
                        result.push(Token {
                            extent: iterators_to_extent(&begin, &end),
                            lexeme: representation,
                            value: token_value,
                        });
                        begin = end;
                    } else {
                        let end = begin.skip_n(1).unwrap();
                        result.push(Token {
                            extent: iterators_to_extent(&begin, &end),
                            lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                            value: TokenValue::Dot,
                        });
                        begin = end;
                    }
                }
            }
        } else if let Some((token_value, end)) = known_symbolic_tokens(begin) {
            result.push(Token {
                extent: iterators_to_extent(&begin, &end),
                lexeme: ImmutableIterator::slice_to_string(&begin, &end),
                value: token_value,
            });
            begin = end;
        } else {
            return Err(LexerError {
                position: begin.index,
                reason: format!("Unexpected symbol `{first_char}`"),
            });
        }
    }
    Ok(result)
}
