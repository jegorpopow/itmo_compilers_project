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
    position: Position,
}

impl<'a> ImmutableIterator<'a> for IndexIterator<'a> {
    fn from_index(string: &'a str, n: usize) -> Self {
        IndexIterator {
            underlying: string,
            index: string.char_indices().nth(n).map(|(idx, _)| idx).unwrap(),
            position: Position::begin(),
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
                    position: self.position.advance(ch == '\n'),
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
        start: start.position,
        end: end.position,
    }
}

/// Processes all the identifier-like lexemes (identifiers, keywords, bool literals and some operators)
fn name_disambigation(lexeme: &str) -> TokenKind<'_> {
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

fn nominal_tokens<'a>(begin: &IndexIterator<'a>) -> Option<(TokenKind<'a>, IndexIterator<'a>)> {
    begin
        .lookup()
        .is_some_and(is_identifier_start)
        .then(|| begin.take_while_map(is_identifier_continue, name_disambigation))
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

fn real_literal_from_representation(s: &str) -> TokenKind<'_> {
    match s.parse::<f64>() {
        Ok(value) => TokenKind::RealLiteral(RealLiteral { value }),
        Err(e) => TokenKind::Invalid(InvalidToken {
            problem: format!("Malformed float {s:?}: {e}"),
        }),
    }
}

fn integer_literal_from_representation(s: &str) -> TokenKind<'_> {
    match s.parse::<i64>() {
        Ok(value) => TokenKind::IntegerLiteral(IntegerLiteral { value }),
        Err(e) => TokenKind::Invalid(InvalidToken {
            problem: format!("Malformed integer {s:?}: {e}"),
        }),
    }
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
        | TokenKind::Invalid(_)
        | TokenKind::Colon => false,
    }
}

fn numerical_tokens<'a>(
    expect_sign: bool,
    begin: &IndexIterator<'a>,
) -> Option<(TokenKind<'a>, IndexIterator<'a>)> {
    let start_digits = if expect_sign && let Some(('-' | '+', it)) = begin.next() {
        it
    } else {
        begin.clone()
    };

    let (whole_part, tail) = start_digits.take_while(|ch| ch.is_ascii_digit());

    if let Some(('.', start_frac)) = tail.next() {
        let (frac_part, rest) = start_frac.take_while(|ch| ch.is_ascii_digit());

        (!frac_part.is_empty())
            .then(|| {
                (
                    real_literal_from_representation(ImmutableIterator::slice_to_str(begin, &rest)),
                    rest,
                )
            })
            .or_else(|| {
                (!whole_part.is_empty()).then(|| {
                    (
                        integer_literal_from_representation(ImmutableIterator::slice_to_str(
                            begin, &tail,
                        )),
                        tail,
                    )
                })
            })
    } else {
        (!whole_part.is_empty()).then(|| {
            (
                integer_literal_from_representation(ImmutableIterator::slice_to_str(begin, &tail)),
                tail,
            )
        })
    }
}

pub fn tokenize(source: &str) -> Vec<Token<'_>> {
    let mut begin = IndexIterator::from_beginning(source);
    let mut result = Vec::new();
    let mut expects_sign = true;

    while let Some(first_char) = begin.lookup() {
        if first_char.is_whitespace() {
            begin = begin.skip(char::is_whitespace);
            continue;
        }

        let (kind, end) = begin
            .stars_with("--")
            .map(|comment_start| {
                comment_start.take_while_map(
                    |ch| ch != '\n',
                    |comment| TokenKind::Comment(Comment { value: comment }),
                )
            })
            .or_else(|| nominal_tokens(&begin))
            .or_else(|| numerical_tokens(expects_sign, &begin))
            .or_else(|| known_symbolic_tokens(&begin))
            .unwrap_or((
                TokenKind::Invalid(InvalidToken {
                    problem: format!("Unexpected symbol `{first_char}`"),
                }),
                begin.skip_n(1).unwrap(),
            ));

        println!("{kind}, {}", end.index);

        expects_sign = should_expect_sign(expects_sign, &kind);
        result.push(Token {
            extent: iterators_to_extent(&begin, &end),
            lexeme: ImmutableIterator::slice_to_str(&begin, &end),
            kind,
        });
        begin = end;
    }

    result
}
