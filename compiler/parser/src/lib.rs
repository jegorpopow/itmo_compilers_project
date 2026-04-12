use core::fmt;
use std::rc::Rc;

use common::{Extent, LoopOrder, Position, RawIdentifier, Real};
use lexer::{
    BuiltinTypename, Identifier as TokenIdentifier, InvalidToken, Keyword, Lexeme,
    Literal as TokenLiteral, Token,
};

mod tree;
mod types;

#[cfg(test)]
mod tests;

pub use crate::{
    tree::{
        Block, BlockElem, ConstDecl, Declaration, Expression, Literal, LvalueExpression, Operator,
        Program, RoutineBody, RoutineDecl, Statement, TypeDecl, VarDecl,
    },
    types::{ArrayDescription, FieldDescription, RecordDescription, Type},
};

pub trait TokenIterator<'a, 'b: 'a>: Clone + Copy {
    fn current(&self) -> Option<&'a Lexeme<'b>>;
    fn position(&self) -> Position;
    #[must_use]
    fn next(&self) -> Self;
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedIterator<'a, 'b: 'a> {
    underlying: &'a [Lexeme<'b>],
    index: usize,
    pos: Position,
}

impl<'a, 'b> From<&'a [Lexeme<'b>]> for IndexedIterator<'a, 'b> {
    fn from(value: &'a [Lexeme<'b>]) -> Self {
        IndexedIterator {
            underlying: value,
            index: 0,
            pos: match value.first() {
                Some(t) => t.extent.start,
                None => Position { line: 0, column: 0 },
            },
        }
    }
}

impl<'a, 'b> TokenIterator<'a, 'b> for IndexedIterator<'a, 'b> {
    fn current(&self) -> Option<&'a Lexeme<'b>> {
        self.underlying.get(self.index)
    }

    fn position(&self) -> Position {
        self.pos
    }

    fn next(&self) -> Self {
        IndexedIterator {
            underlying: self.underlying,
            index: self.index + 1,
            pos: match self.underlying.get(self.index + 1) {
                Some(t) => t.extent.start,
                None => self.underlying[self.index].extent.end,
            },
        }
    }
}

#[derive(Debug)]
pub struct ParsingError {
    pub what: String,
    pub position: Position,
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { what, position } = self;
        write!(f, "{what} @ {position}")
    }
}

impl core::error::Error for ParsingError {}

#[derive(Debug, Default)]
pub struct Parser {
    pub recovered_errors: Vec<ParsingError>,
}

type ParsingResult<'a, 'b, T> = Result<(T, IndexedIterator<'a, 'b>), ParsingError>; // For hard errors

trait ParsingFunction<'a, 'b: 'a, T>:
    FnMut(&mut Parser, IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, T>
{
}

impl<'a, 'b: 'a, T, F> ParsingFunction<'a, 'b, T> for F where
    F: FnMut(&mut Parser, IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, T>
{
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn report_recovered(&mut self, reason: String, position: Position) {
        self.recovered_errors.push(ParsingError {
            what: reason,
            position,
        });
    }

    fn skip_unused<'a, 'b: 'a>(
        &mut self,
        mut i: IndexedIterator<'a, 'b>,
    ) -> IndexedIterator<'a, 'b> {
        while let Some(token) = i.current() {
            match token {
                Lexeme {
                    token: Token::Invalid(t @ InvalidToken::Unexpected(_)),
                    extent: Extent { start, .. },
                    ..
                } => {
                    self.report_recovered(t.to_string(), *start);
                }
                Lexeme {
                    token: Token::Comment(_),
                    ..
                } => {}
                _ => break,
            }

            i = i.next();
        }
        i
    }

    fn next<'a, 'b: 'a>(&mut self, i: IndexedIterator<'a, 'b>) -> IndexedIterator<'a, 'b> {
        self.skip_unused(i.next())
    }

    // Parsing combinators

    /// Parses with `parser` until it fails
    #[expect(
        clippy::unnecessary_wraps,
        reason = "Better composition with other APIs"
    )]
    fn parse_many<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Vec<T>> {
        let mut result = Vec::new();

        while let Ok((parsed, next)) = parser(self, i) {
            result.push(parsed);
            i = next;
        }

        Ok((result, i))
    }

    /// Parses with `parser` till the end
    fn parse_all<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Vec<T>> {
        let mut result = Vec::new();

        while i.current().is_some() {
            let (parsed, next) = parser(self, i)?;
            result.push(parsed);
            i = next;
        }

        Ok((result, i))
    }

    /// Parses one of two alternatives
    fn parse_one_of<'a, 'b: 'a, T>(
        &mut self,
        mut l: impl ParsingFunction<'a, 'b, T>,
        mut r: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        l(self, i).or_else(|first_err| {
            r(self, i).map_err(|second_err| ParsingError {
                what: format!(
                    "Following parses failed\n{}\n{}",
                    first_err.what, second_err.what
                ),
                position: first_err.position,
            })
        })
    }

    /// Parses with `parser`, parses with `right` and drops latter value
    fn parse_before<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut right: impl ParsingFunction<'a, 'b, ()>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        let (res, next) = parser(self, i)?;
        let ((), next) = right(self, next)?;
        Ok((res, next))
    }

    ///  parses with `left`, parses with `parser` and drops former value
    fn parse_after<'a, 'b: 'a, T>(
        &mut self,
        mut left: impl ParsingFunction<'a, 'b, ()>,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        let ((), next) = left(self, i)?;
        parser(self, next)
    }

    /// parse_after and parse_before combined
    fn parse_between<'a, 'b: 'a, T>(
        &mut self,
        mut left: impl ParsingFunction<'a, 'b, ()>,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut right: impl ParsingFunction<'a, 'b, ()>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        let ((), next) = left(self, i)?;
        let (res, next) = parser(self, next)?;
        let ((), next) = right(self, next)?;
        Ok((res, next))
    }

    /// Parses with `parser`, successfully return None on failure
    fn try_parse<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Option<T>> {
        parser(self, i)
            .map(|(val, i)| (Some(val), i))
            .or_else(|_| Ok((None, i)))
    }

    /// Parse a list of `parser` values, separated by `sep`
    fn parse_many_sep_by<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut sep: impl ParsingFunction<'a, 'b, ()>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Vec<T>> {
        let mut result = Vec::new();

        let Ok((first_parsed, rest)) = parser(self, i) else {
            return Ok((Vec::new(), i));
        };

        result.push(first_parsed);

        let mut next = rest;

        while let Ok(((), rest)) = sep(self, next) {
            next = rest;
            let (elem, rest) = parser(self, next)?;
            next = rest;
            result.push(elem);
        }

        Ok((result, next))
    }

    // / Parsers `l`, `rep`, `r`, drops the values in centre
    fn parse_def<'a, 'b: 'a, T, U>(
        &mut self,
        mut l: impl ParsingFunction<'a, 'b, T>,
        mut rep: impl ParsingFunction<'a, 'b, ()>,
        mut r: impl ParsingFunction<'a, 'b, U>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, (T, U)> {
        let (lhs, next) = l(self, i)?;
        let ((), next) = rep(self, next)?;
        let (rhs, end) = r(self, next)?;
        Ok(((lhs, rhs), end))
    }

    // Parsers for primitives

    fn parse_identifier<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, RawIdentifier> {
        match i.current() {
            Some(&Lexeme {
                token: Token::Identifier(TokenIdentifier { name }),
                ..
            }) => Ok((
                RawIdentifier {
                    name: name.to_owned(),
                },
                self.next(i),
            )),
            Some(token) => Err(ParsingError {
                what: format!("Identifier expected, {token} found"),
                position: i.position(),
            }),
            None => Err(ParsingError {
                what: "Identifier expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    fn parse_literal<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Literal> {
        match i.current() {
            Some(&Lexeme {
                token: Token::Literal(TokenLiteral::Real(value)),
                text,
                ..
            }) => Ok((
                Literal::Real {
                    repr: text.to_owned(),
                    value,
                },
                self.next(i),
            )),

            Some(&Lexeme {
                token: Token::Invalid(ref t @ InvalidToken::MalformedReal(_)),
                extent: Extent { start, .. },
                text,
            }) => {
                self.recovered_errors.push(ParsingError {
                    position: start,
                    what: t.to_string(),
                });

                Ok((
                    Literal::Real {
                        repr: text.to_owned(),
                        value: Real::NAN,
                    },
                    self.next(i),
                ))
            }

            Some(&Lexeme {
                token: Token::Literal(TokenLiteral::Integer(value)),
                text,
                ..
            }) => Ok((
                Literal::Integer {
                    repr: text.to_owned(),
                    value,
                },
                self.next(i),
            )),

            Some(&Lexeme {
                token: Token::Invalid(ref t @ InvalidToken::MalformedInteger(_)),
                extent: Extent { start, .. },
                text,
            }) => {
                self.recovered_errors.push(ParsingError {
                    position: start,
                    what: t.to_string(),
                });
                Ok((
                    Literal::Integer {
                        repr: text.to_owned(),
                        value: 0,
                    },
                    self.next(i),
                ))
            }

            Some(&Lexeme {
                token: Token::Literal(TokenLiteral::Bool(value)),
                ..
            }) => Ok((Literal::Bool { value }, self.next(i))),

            Some(token) => Err(ParsingError {
                what: format!("Literal expected, {token} found"),
                position: i.position(),
            }),
            None => Err(ParsingError {
                what: "Literal expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    fn parse_known_kind<'a, 'b: 'a>(
        &mut self,
        expected_kind: &Token<'a>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        match i.current() {
            Some(Lexeme { token: kind, .. }) if kind == expected_kind => Ok(((), self.next(i))),
            Some(token) => Err(ParsingError {
                what: format!("Token of kind {expected_kind} expected, token {token} found"),
                position: token.extent.start,
            }),
            None => Err(ParsingError {
                what: format!("Token of kind {expected_kind} expected, token EOF found"),
                position: i.position(),
            }),
        }
    }

    fn parse_keyword<'a, 'b: 'a>(
        &mut self,
        keyword: Keyword,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::Keyword(keyword), i)
    }

    fn parse_semicolon<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::Semicolon, i)
    }

    fn parse_assn<'a, 'b: 'a>(&mut self, i: IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::Assignment, i)
    }

    fn parse_comma<'a, 'b: 'a>(&mut self, i: IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::Comma, i)
    }

    fn parse_colon<'a, 'b: 'a>(&mut self, i: IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::Colon, i)
    }

    fn parse_kw_end<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_keyword(Keyword::End, i)
    }

    fn parse_kw_loop<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_keyword(Keyword::Loop, i)
    }

    fn parse_kw_where<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_keyword(Keyword::Where, i)
    }

    fn parse_kw_is<'a, 'b: 'a>(&mut self, i: IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, ()> {
        self.parse_keyword(Keyword::Is, i)
    }

    fn parse_left_bracket<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::LeftBracket, i)
    }

    fn parse_right_bracket<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::RightBracket, i)
    }

    fn parse_in_brackets<'a, 'b: 'a, T>(
        &mut self,
        parser: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        self.parse_between(
            Self::parse_left_bracket,
            parser,
            Self::parse_right_bracket,
            i,
        )
    }

    fn parse_left_parenthesis<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::LeftParenthesis, i)
    }

    fn parse_right_parenthesis<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&Token::RightParenthesis, i)
    }

    fn parse_in_parentheses<'a, 'b: 'a, T>(
        &mut self,
        parser: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        self.parse_between(
            Self::parse_left_parenthesis,
            parser,
            Self::parse_right_parenthesis,
            i,
        )
    }

    fn parse_field_description<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, FieldDescription> {
        let ((), next) = self.parse_keyword(Keyword::Var, i)?;
        let (ident, next) = self.parse_identifier(next)?;
        let ((), next) = self.parse_kw_is(next)?;
        let (t, next) = self.parse_type(next)?;

        Ok((FieldDescription { name: ident, t }, next))
    }

    fn parse_record_desc<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, RecordDescription> {
        let ((), next) = self.parse_keyword(Keyword::Record, i)?;
        let (fields, next) = self.parse_many(
            |ctx, i| ctx.parse_before(Self::parse_field_description, Self::parse_semicolon, i),
            next,
        )?;

        let ((), next) = self.parse_keyword(Keyword::End, next)?;
        Ok((RecordDescription { fields }, next))
    }

    fn parse_array_desc<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ArrayDescription> {
        let ((), next) = self.parse_keyword(Keyword::Array, i)?;
        let (length, next) =
            self.parse_in_brackets(|ctx, i| ctx.try_parse(Self::parse_expr, i), next)?;
        self.parse_type(next).map(|(element_type, next)| {
            (
                ArrayDescription {
                    length,
                    t: element_type.into(),
                },
                next,
            )
        })
    }

    fn parse_type<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Type> {
        self.parse_identifier(i)
            .map(|(ident, next)| (Type::Alias(ident), next))
            .or_else(|_| {
                self.parse_known_kind(&Token::BuiltinTypename(BuiltinTypename::Real), i)
                    .map(|((), next)| (Type::Real, next))
            })
            .or_else(|_| {
                self.parse_known_kind(&Token::BuiltinTypename(BuiltinTypename::Integer), i)
                    .map(|((), next)| (Type::Int, next))
            })
            .or_else(|_| {
                self.parse_known_kind(&Token::BuiltinTypename(BuiltinTypename::Boolean), i)
                    .map(|((), next)| (Type::Bool, next))
            })
            .or_else(|_| {
                self.parse_array_desc(i)
                    .map(|(arr_desc, i)| (Type::Array(arr_desc), i))
            })
            .or_else(|_| {
                self.parse_record_desc(i)
                    .map(|(rec_desc, i)| (Type::Record(rec_desc), i))
            })
            .map_err(|_err| ParsingError {
                what: "Type expected, but not found".to_owned(),
                position: i.position(),
            })
    }

    fn parse_operators<'a, 'b: 'a>(
        &mut self,
        mut ops: impl Iterator<Item = &'static [Operator]> + Clone,
        mut atom: impl ParsingFunction<'a, 'b, Rc<Expression>> + Clone,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        match ops.next() {
            Some(operators) => {
                let (mut head, mut next) = self.parse_operators(ops.clone(), atom.clone(), i)?;
                while let Some(Lexeme {
                    token: Token::Operator(op),
                    ..
                }) = next.current()
                    && operators.contains(op)
                {
                    next = self.next(next);
                    let (rhs, rest) = self.parse_operators(ops.clone(), atom.clone(), next)?;
                    next = rest;
                    head = Rc::new(Expression::BinOp {
                        op: *op,
                        lhs: head,
                        rhs,
                    });
                }

                Ok((head, next))
            }
            None => atom(self, i),
        }
    }

    fn parse_lvalue_expr<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<LvalueExpression>> {
        let (ident, next) = self.parse_identifier(i)?;
        let mut next = next;

        let mut result = Rc::new(LvalueExpression::Identifier(ident));

        loop {
            match next.current() {
                Some(Lexeme {
                    token: Token::Dot, ..
                }) => {
                    next = self.next(next);
                    let (field_name, rest) = self.parse_identifier(next)?;
                    result = Rc::new(LvalueExpression::Member {
                        lhs: result,
                        member_name: field_name,
                    });
                    next = rest;
                }
                Some(Lexeme {
                    token: Token::LeftBracket,
                    ..
                }) => {
                    let (index, rest) = self.parse_in_brackets(Self::parse_expr, next)?;
                    result = Rc::new(LvalueExpression::Index { lhs: result, index });
                    next = rest;
                }
                _ => break,
            }
        }

        Ok((result, next))
    }

    fn parse_unop<'a, 'b: 'a>(
        &mut self,
        op: Operator,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        self.parse_after(
            |ctx, i| ctx.parse_known_kind(&Token::Operator(op), i),
            Self::parse_atom,
            i,
        )
        .map(|(atom, i)| (Rc::new(Expression::UnOp { op, operand: atom }), i))
    }

    fn parse_new<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        self.parse_after(
            |ctx, i| ctx.parse_keyword(Keyword::New, i),
            |ctx, i| {
                let (array_length, i) =
                    ctx.try_parse(|ctx, i| ctx.parse_in_brackets(Self::parse_expr, i), i)?;
                let (t, next) = ctx.parse_type(i)?;
                let (fields, next) = ctx.try_parse(
                    |ctx, i| {
                        ctx.parse_between(
                            Self::parse_kw_where,
                            |ctx, i| {
                                ctx.parse_many(
                                    |ctx, i| {
                                        ctx.parse_before(
                                            |ctx, i| {
                                                ctx.parse_def(
                                                    Self::parse_identifier,
                                                    Self::parse_kw_is,
                                                    Self::parse_expr,
                                                    i,
                                                )
                                            },
                                            Self::parse_semicolon,
                                            i,
                                        )
                                    },
                                    i,
                                )
                            },
                            Self::parse_kw_end,
                            i,
                        )
                    },
                    next,
                )?;

                Ok((
                    Rc::new(Expression::New {
                        t,
                        fields,
                        array_length,
                    }),
                    next,
                ))
            },
            i,
        )
    }

    fn parse_call<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        let (callee, next) = self.parse_identifier(i)?;
        let (args, next) = self.parse_in_parentheses(
            |ctx, i| ctx.parse_many_sep_by(Self::parse_expr, Self::parse_comma, i),
            next,
        )?;

        Ok((Rc::new(Expression::Call { callee, args }), next))
    }

    fn parse_atom<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        // TODO: replace `.or_else()`-es with `match i.current()` or `if/else` for better error handling
        let (mut head, mut next) = self
            .parse_call(i)
            .or_else(|_| {
                self.parse_keyword(Keyword::Null, i)
                    .map(|((), next)| (Rc::new(Expression::Null), next))
            })
            .or_else(|_| {
                self.parse_literal(i)
                    .map(|(literal, next)| (Rc::new(Expression::Literal(literal)), next))
            })
            .or_else(|_| self.parse_unop(Operator::Not, i))
            .or_else(|_| self.parse_unop(Operator::Minus, i))
            .or_else(|_| self.parse_new(i))
            .or_else(|_| self.parse_in_parentheses(Self::parse_expr, i))
            .or_else(|_| {
                self.parse_lvalue_expr(i)
                    .map(|(lvalue, next)| (Rc::new(Expression::LvalueToRvalue(lvalue)), next))
            })?;

        while let Some(Lexeme {
            token: Token::Cast, ..
        }) = next.current()
        {
            next = self.next(next);
            let (t, rest) = self.parse_type(next)?;
            head = Rc::new(Expression::Cast {
                operand: head,
                target: t,
            });
            next = rest;
        }

        Ok((head, next))
    }

    pub fn parse_expr<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        const OPERATORS_PRECEDENCE_TABLE: &[&[Operator]] = &[
            &[Operator::And, Operator::Or, Operator::Xor],
            &[
                Operator::Lt,
                Operator::Le,
                Operator::Ne,
                Operator::Eq,
                Operator::Gt,
                Operator::Ge,
            ],
            &[Operator::Plus, Operator::Minus],
            &[Operator::Mul, Operator::Div, Operator::Mod],
        ];

        self.parse_operators(
            OPERATORS_PRECEDENCE_TABLE.iter().copied(),
            Self::parse_atom,
            i,
        )
    }

    pub fn parse_var_decl<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, VarDecl> {
        let ((), next) = self.parse_keyword(Keyword::Var, i)?;
        let (name, next) = self.parse_identifier(next)?;
        let (t, next) = self.try_parse(
            |ctx, i| ctx.parse_after(Self::parse_colon, Self::parse_type, i),
            next,
        )?;
        let (initialiser, next) = self.try_parse(
            |ctx, i| ctx.parse_after(Self::parse_kw_is, Self::parse_expr, i),
            next,
        )?;
        let ((), next) = self.parse_semicolon(next)?;

        Ok((
            VarDecl {
                name,
                t,
                initialiser,
            },
            next,
        ))
    }

    pub fn parse_const_decl<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ConstDecl> {
        let ((), next) = self.parse_keyword(Keyword::Constant, i)?;
        let (name, next) = self.parse_identifier(next)?;
        let (t, next) = self.try_parse(
            |ctx, i: IndexedIterator<'_, '_>| {
                ctx.parse_after(Self::parse_colon, Self::parse_type, i)
            },
            next,
        )?;
        let (initialiser, next) = self.parse_after(Self::parse_kw_is, Self::parse_expr, next)?;
        let ((), next) = self.parse_semicolon(next)?;

        Ok((
            ConstDecl {
                name,
                t,
                initialiser,
            },
            next,
        ))
    }

    pub fn parse_type_decl<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, TypeDecl> {
        let ((), next) = self.parse_keyword(Keyword::Type, i)?;
        let (name, next) = self.parse_identifier(next)?;
        let (t, next) = self.parse_after(Self::parse_kw_is, Self::parse_type, next)?;
        let ((), next) = self.parse_semicolon(next)?;

        Ok((TypeDecl { name, t }, next))
    }

    fn parse_assignment<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Statement> {
        let (lhs, next) = self.parse_lvalue_expr(i)?;
        let ((), next) = self.parse_assn(next)?;
        let (rhs, next) = self.parse_expr(next)?;
        let ((), next) = self.parse_semicolon(next)?;

        Ok((Statement::Assignment { lhs, rhs }, next))
    }
    fn parse_if<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Statement> {
        let next = self.next(i);
        let (condition, next) = self.parse_expr(next)?;
        let ((), next) = self.parse_keyword(Keyword::Then, next)?;
        let (on_true, next) = self.parse_block(next)?;
        let (on_false, next) = self.try_parse(
            |ctx, i| {
                ctx.parse_after(
                    |ctx, i| ctx.parse_keyword(Keyword::Else, i),
                    Self::parse_block,
                    i,
                )
            },
            next,
        )?;

        let ((), next) = self.parse_kw_end(next)?;

        Ok((
            Statement::If {
                condition,
                on_true,
                on_false,
            },
            next,
        ))
    }

    pub fn parse_statement<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Statement> {
        match i.current() {
            Some(Lexeme {
                token: Token::Keyword(Keyword::If),
                ..
            }) => self.parse_if(i),

            Some(Lexeme {
                token: Token::Keyword(Keyword::While),
                ..
            }) => {
                let next = self.next(i);
                let (condition, next) = self.parse_expr(next)?;

                let (body, next) = self.parse_between(
                    Self::parse_kw_loop,
                    Self::parse_block,
                    Self::parse_kw_end,
                    next,
                )?;

                Ok((Statement::While { condition, body }, next))
            }

            Some(Lexeme {
                token: Token::Keyword(Keyword::Print),
                ..
            }) => {
                let next = self.next(i);
                let (expr, next) = self.parse_expr(next)?;
                let ((), next) = self.parse_semicolon(next)?;

                Ok((Statement::Print { value: expr }, next))
            }

            Some(Lexeme {
                token: Token::Keyword(Keyword::Return),
                ..
            }) => {
                let next = self.next(i);
                let (expr, next) = self.parse_expr(next)?;
                let ((), next) = self.parse_semicolon(next)?;

                Ok((Statement::Return { value: expr }, next))
            }
            Some(&Lexeme {
                token: Token::Keyword(Keyword::Panic),
                extent: Extent { start: pos, end: _ },
                text: _,
            }) => {
                let next = self.next(i);
                let ((), next) = self.parse_semicolon(next)?;
                Ok((Statement::Panic { pos }, next))
            }

            Some(&Lexeme {
                token: Token::Keyword(Keyword::Assert),
                extent: Extent { start: pos, end: _ },
                text: _,
            }) => {
                let next = self.next(i);
                let (value, next) = self.parse_expr(next)?;
                let ((), next) = self.parse_semicolon(next)?;
                Ok((Statement::Assert { value, pos }, next))
            }

            Some(Lexeme {
                token: Token::Keyword(Keyword::For),
                ..
            }) => {
                let next = self.next(i);
                let (counter, next) = self.parse_identifier(next)?;

                let ((), next) = self.parse_keyword(Keyword::In, next)?;

                let (from, next) = self.parse_expr(next)?;

                let ((), next) = self.parse_known_kind(&Token::RangeSymbol, next)?;
                let (to, next) = self.try_parse(Self::parse_expr, next)?;
                let (order, next) =
                    self.try_parse(|ctx, i| ctx.parse_keyword(Keyword::Reverse, i), next)?;

                let order = order.map_or(LoopOrder::Direct, |()| LoopOrder::Reversed);

                let (body, next) = self.parse_between(
                    Self::parse_kw_loop,
                    Self::parse_block,
                    Self::parse_kw_end,
                    next,
                )?;

                Ok((
                    Statement::For {
                        counter,
                        from,
                        to,
                        order,
                        body,
                    },
                    next,
                ))
            }

            Some(Lexeme {
                token: Token::Identifier(_),
                ..
            }) => self.parse_one_of(
                Self::parse_assignment,
                |ctx, i| {
                    ctx.parse_before(Self::parse_call, Self::parse_semicolon, i)
                        .map(|(call_expr, next)| (Statement::Expr(call_expr), next))
                },
                i,
            ),

            Some(token) => Err(ParsingError {
                what: format!("Statement expected, {token} found"),
                position: i.position(),
            }),

            None => Err(ParsingError {
                what: "Statement expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    pub fn parse_block_elem<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, BlockElem> {
        match i.current() {
            Some(Lexeme {
                token: Token::Keyword(Keyword::Var),
                ..
            }) => self
                .parse_var_decl(i)
                .map(|(decl, next)| (BlockElem::VarDecl(decl), next)),

            Some(Lexeme {
                token: Token::Keyword(Keyword::Constant),
                ..
            }) => self
                .parse_const_decl(i)
                .map(|(decl, next)| (BlockElem::ConstDecl(decl), next)),

            Some(Lexeme {
                token: Token::Keyword(Keyword::Type),
                ..
            }) => self
                .parse_type_decl(i)
                .map(|(decl, next)| (BlockElem::TypeDecl(decl), next)),

            Some(_) => self
                .parse_statement(i)
                .map(|(stmt, next)| (BlockElem::Stmt(stmt), next)),

            None => Err(ParsingError {
                what: "Statement or declaration expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    pub fn parse_block<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Block> {
        self.parse_many(Self::parse_block_elem, i)
            .map(|(elems, next)| (Block(elems), next))
    }

    pub fn parse_routine_decl<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, RoutineDecl> {
        let ((), next) = self.parse_keyword(Keyword::Routine, i)?;
        let (name, next) = self.parse_identifier(next)?;

        let (args, next) = self.parse_in_parentheses(
            |ctx, i| {
                ctx.parse_many_sep_by(
                    |ctx, i| {
                        ctx.parse_def(
                            Self::parse_identifier,
                            Self::parse_colon,
                            Self::parse_type,
                            i,
                        )
                    },
                    Self::parse_comma,
                    i,
                )
            },
            next,
        )?;

        let (return_type, next) = self.try_parse(
            |ctx, i| ctx.parse_after(Self::parse_colon, Self::parse_type, i),
            next,
        )?;

        match next.current() {
            Some(Lexeme {
                token: Token::RightArrow,
                ..
            }) => {
                let next = self.next(next);
                let (expr, next) =
                    self.parse_before(Self::parse_expr, Self::parse_semicolon, next)?;
                Ok((
                    RoutineDecl {
                        name,
                        arguments: args,
                        return_type,
                        body: Some(RoutineBody::Expression(expr)),
                    },
                    next,
                ))
            }

            Some(Lexeme {
                token: Token::Keyword(Keyword::Is),
                ..
            }) => {
                let (body, next) = self.parse_between(
                    Self::parse_kw_is,
                    Self::parse_block,
                    Self::parse_kw_end,
                    next,
                )?;

                Ok((
                    RoutineDecl {
                        name,
                        arguments: args,
                        return_type,
                        body: Some(RoutineBody::Block(body)),
                    },
                    next,
                ))
            }

            Some(Lexeme {
                token: Token::Semicolon,
                ..
            }) => Ok((
                RoutineDecl {
                    name,
                    arguments: args,
                    return_type,
                    body: None,
                },
                self.next(next),
            )),
            Some(t) => Err(ParsingError {
                what: format!("Function body or `;` expected, {t} found"),
                position: next.position(),
            }),
            None => Err(ParsingError {
                what: "Function body or `;` expected, EOF found".to_owned(),
                position: next.position(),
            }),
        }
    }

    pub fn parse_declaration<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Declaration> {
        match i.current() {
            Some(Lexeme {
                token: Token::Keyword(Keyword::Var),
                ..
            }) => self
                .parse_var_decl(i)
                .map(|(decl, next)| (Declaration::Var(decl), next)),

            Some(Lexeme {
                token: Token::Keyword(Keyword::Constant),
                ..
            }) => self
                .parse_const_decl(i)
                .map(|(decl, next)| (Declaration::Const(decl), next)),

            Some(Lexeme {
                token: Token::Keyword(Keyword::Type),
                ..
            }) => self
                .parse_type_decl(i)
                .map(|(decl, next)| (Declaration::Type(decl), next)),

            Some(Lexeme {
                token: Token::Keyword(Keyword::Routine),
                ..
            }) => self
                .parse_routine_decl(i)
                .map(|(decl, next)| (Declaration::Routine(decl), next)),

            Some(t) => Err(ParsingError {
                what: format!("Declaration expected, {t} found"),
                position: i.position(),
            }),
            None => Err(ParsingError {
                what: "Declaration expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    pub fn finish<T>(self, parsed: T) -> ParserResult<T> {
        let Self { recovered_errors } = self;
        if recovered_errors.is_empty() {
            Ok(parsed)
        } else {
            Err(ParserError::Recoverable {
                errors: recovered_errors,
                parsed,
            })
        }
    }
}

#[derive(Debug)]
pub enum ParserError<T> {
    Unrecoverable {
        error: ParsingError,
    },
    Recoverable {
        errors: Vec<ParsingError>,
        parsed: T,
    },
}

pub type ParserResult<T> = Result<T, ParserError<T>>;

impl<T> From<ParsingError> for ParserError<T> {
    fn from(error: ParsingError) -> Self {
        Self::Unrecoverable { error }
    }
}

impl<T: fmt::Debug> fmt::Display for ParserError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unrecoverable { error } => write!(f, "unrecoverable error: {error}"),
            Self::Recoverable { errors, parsed } => {
                writeln!(f, "Some recoverable errors occurred during parsing:")?;
                for err in errors {
                    writeln!(f, "- {err}")?
                }
                writeln!(f, "But managed to parse: {parsed:#?}")
            }
        }
    }
}

impl<T: fmt::Debug> core::error::Error for ParserError<T> {}

pub fn parse_program(tokens: &[Lexeme<'_>]) -> ParserResult<Program> {
    let mut parser = Parser::new();
    let start = parser.skip_unused(IndexedIterator::from(tokens));
    let (decls, tail) = parser.parse_all(Parser::parse_declaration, start)?;
    debug_assert_eq!(tail.current(), None, "EOF not reached during parsing");
    parser.finish(Program(decls))
}
