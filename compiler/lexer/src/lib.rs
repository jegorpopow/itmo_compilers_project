use phf::phf_map;

use common::{Extent, Position, Real};

mod tokens;

#[cfg(test)]
mod tests;

pub use crate::tokens::{
    BuiltinTypename, Comment, Identifier, InvalidToken, Keyword, Lexeme, Literal, Operator, Token,
};

trait ImmutableIterator<'a>: Sized + Clone + From<&'a str> {
    fn slice_to_str(start: &Self, end: &Self) -> &'a str;
    fn next(&self) -> Option<(char, Self)>;

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
#[derive(Clone, Debug)]
struct IndexIterator<'a> {
    underlying: &'a str,
    index: usize,
    position: Position,
}

impl<'a> From<&'a str> for IndexIterator<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            underlying: s,
            index: 0,
            position: Position::begin(),
        }
    }
}

impl<'a> ImmutableIterator<'a> for IndexIterator<'a> {
    fn slice_to_str(start: &Self, end: &Self) -> &'a str {
        assert_eq!(
            start.underlying.as_ptr(),
            end.underlying.as_ptr(),
            "cannot slice from different underlying strings"
        );
        &start.underlying[start.index..end.index]
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

#[expect(clippy::tests_outside_test_module, reason = "better fits here")]
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
fn name_disambiguation(lexeme: &str) -> Token<'_> {
    const KNOWN_TOKENS: phf::Map<&str, Token<'static>> = phf_map! {
        "var" => Token::Keyword(Keyword::Var),
        "type" => Token::Keyword(Keyword::Type),
        "routine" => Token::Keyword(Keyword::Routine),
        "array" => Token::Keyword(Keyword::Array),
        "record" => Token::Keyword(Keyword::Record),
        "is" => Token::Keyword(Keyword::Is),
        "end" => Token::Keyword(Keyword::End),
        "if" => Token::Keyword(Keyword::If),
        "then" => Token::Keyword(Keyword::Then),
        "else" => Token::Keyword(Keyword::Else),
        "in" => Token::Keyword(Keyword::In),
        "while" => Token::Keyword(Keyword::While),
        "for" => Token::Keyword(Keyword::For),
        "loop" => Token::Keyword(Keyword::Loop),
        "reverse" => Token::Keyword(Keyword::Reverse),
        "return"  => Token::Keyword(Keyword::Return),
        "print" => Token::Keyword(Keyword::Print),
        "where" => Token::Keyword(Keyword::Where),
        "null" => Token::Keyword(Keyword::Null),
        "new" => Token::Keyword(Keyword::New),
        "constant" => Token::Keyword(Keyword::Constant),
        "and" => Token::Operator(Operator::And),
        "or" => Token::Operator(Operator::Or),
        "xor" => Token::Operator(Operator::Xor),
        "not" => Token::Operator(Operator::Not),
        "true" => Token::Literal(Literal::Bool(true)),
        "false" => Token::Literal(Literal::Bool(false)),
        "integer" => Token::BuiltinTypename(BuiltinTypename::Integer),
        "real" => Token::BuiltinTypename(BuiltinTypename::Real),
        "boolean" => Token::BuiltinTypename(BuiltinTypename::Boolean),
        "NaN" => Token::Literal(Literal::Real(Real::NAN)),
        "Inf" => Token::Literal(Literal::Real(Real::INFINITY)),
        "assert" => Token::Keyword(Keyword::Assert),
        "panic" => Token::Keyword(Keyword::Panic),
    };

    match KNOWN_TOKENS.get(lexeme) {
        Some(token_value) => token_value.clone(),
        None => Token::Identifier(Identifier { name: lexeme }),
    }
}

fn nominal_token<'a>(start: &IndexIterator<'a>) -> Option<(Token<'a>, IndexIterator<'a>)> {
    start
        .next()
        .is_some_and(|(ch, _)| is_identifier_start(ch))
        .then(|| start.take_while_map(is_identifier_continue, name_disambiguation))
}

fn comment_token<'a>(start: &IndexIterator<'a>) -> Option<(Token<'a>, IndexIterator<'a>)> {
    start.stars_with("--").map(|comment_start| {
        comment_start.take_while_map(
            |ch| ch != '\n',
            |comment| Token::Comment(Comment { value: comment }),
        )
    })
}

fn symbolic_token<'a>(start: &IndexIterator<'a>) -> Option<(Token<'a>, IndexIterator<'a>)> {
    const KNOWN_TOKENS: &[(&str, Token<'static>)] = &[
        (":=", Token::Assignment),
        ("::", Token::Cast),
        ("..", Token::RangeSymbol),
        ("/=", Token::Operator(Operator::Ne)),
        ("<=", Token::Operator(Operator::Le)),
        (">=", Token::Operator(Operator::Ge)),
        ("=>", Token::RightArrow),
        ("(", Token::LeftParenthesis),
        (")", Token::RightParenthesis),
        ("[", Token::LeftBracket),
        ("]", Token::RightBracket),
        (",", Token::Comma),
        (".", Token::Dot),
        (";", Token::Semicolon),
        (":", Token::Colon),
        ("+", Token::Operator(Operator::Plus)),
        ("-", Token::Operator(Operator::Minus)),
        ("*", Token::Operator(Operator::Mul)),
        ("/", Token::Operator(Operator::Div)),
        ("%", Token::Operator(Operator::Mod)),
        ("=", Token::Operator(Operator::Eq)),
        ("<", Token::Operator(Operator::Lt)),
        (">", Token::Operator(Operator::Gt)),
    ];

    KNOWN_TOKENS
        .iter()
        .find_map(|(pattern, token)| start.stars_with(pattern).map(|end| (token.to_owned(), end)))
}

fn real_literal_from_representation(s: &str) -> Token<'_> {
    match s.parse() {
        Ok(value) => Token::Literal(Literal::Real(value)),
        Err(e) => Token::Invalid(InvalidToken::MalformedReal(e)),
    }
}

fn integer_literal_from_representation(s: &str) -> Token<'_> {
    match s.parse() {
        Ok(value) => Token::Literal(Literal::Integer(value)),
        Err(e) => Token::Invalid(InvalidToken::MalformedInteger(e)),
    }
}

fn numeric_token<'a>(
    allow_sign: bool,
    begin: &IndexIterator<'a>,
) -> Option<(Token<'a>, IndexIterator<'a>)> {
    let start_digits = if allow_sign && let Some(('-' | '+', it)) = begin.next() {
        it
    } else {
        begin.clone()
    };

    let (whole_part, tail) = start_digits.take_while(|ch| ch.is_ascii_digit());

    if let Some(('.', start_frac)) = tail.next() {
        let (frac_part, rest) = start_frac.take_while(|ch| ch.is_ascii_digit());

        if !frac_part.is_empty() {
            Some((
                real_literal_from_representation(ImmutableIterator::slice_to_str(begin, &rest)),
                rest,
            ))
        } else if !whole_part.is_empty() {
            Some((
                integer_literal_from_representation(ImmutableIterator::slice_to_str(begin, &tail)),
                tail,
            ))
        } else {
            None
        }
    } else if !whole_part.is_empty() {
        Some({
            (
                integer_literal_from_representation(ImmutableIterator::slice_to_str(begin, &tail)),
                tail,
            )
        })
    } else {
        None
    }
}

#[derive(Debug)]
pub struct Lexer<'src> {
    pos: IndexIterator<'src>,
    allow_sign: bool,
}

impl<'src> From<&'src str> for Lexer<'src> {
    fn from(src: &'src str) -> Self {
        Self {
            pos: IndexIterator::from(src),
            allow_sign: true,
        }
    }
}

impl Lexer<'_> {
    fn update_allow_sign(&mut self, token: &Token<'_>) {
        self.allow_sign = match token {
            Token::Comment(_) => return,

            Token::Assignment
            | Token::LeftParenthesis
            | Token::RightBracket
            | Token::Operator(_)
            | Token::Semicolon
            | Token::RangeSymbol
            | Token::Comma
            | Token::RightArrow
            | Token::Keyword(_) => true,

            Token::Identifier(_)
            | Token::Literal(_)
            | Token::BuiltinTypename(_)
            | Token::LeftBracket
            | Token::RightParenthesis
            | Token::Dot
            | Token::Invalid(_)
            | Token::Cast
            | Token::Colon => false,
        }
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Lexeme<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        let begin = self.pos.skip(char::is_whitespace);
        let (first_char, rest) = begin.next()?;
        let (kind, end) = comment_token(&begin)
            .or_else(|| nominal_token(&begin))
            .or_else(|| numeric_token(self.allow_sign, &begin))
            .or_else(|| symbolic_token(&begin))
            .unwrap_or((Token::Invalid(InvalidToken::Unexpected(first_char)), rest));
        self.update_allow_sign(&kind);
        let token = Lexeme {
            extent: iterators_to_extent(&begin, &end),
            text: ImmutableIterator::slice_to_str(&begin, &end),
            token: kind,
        };
        self.pos = end;
        Some(token)
    }
}
