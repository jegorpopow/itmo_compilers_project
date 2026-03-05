#![expect(dead_code, reason = "WIP")]
use std::rc::Rc;

use crate::identifier::RawIdentifier;
use crate::operators::SyntacticOperator;
use crate::parse_tree::tree::{
    BoolLiteral, Expression, IntegerLiteral, LvalueExpression, RealLiteral,
};
use crate::parse_tree::types::{ArrayDescription, FieldDescription, RecordDescription, Type};
use crate::source_positions::{Extent, Position};
use crate::tokens::{
    self, BoolLiteral as TokenBoolLiteral, Identifier, IntegerLiteral as TokenIntegerLiteral,
    InvalidToken, RealLiteral as TokenRealLiteral, Token, TokenIssue, TokenKind,
};

trait TokenIterator<'a, 'b: 'a>: Clone + Copy {
    fn current(&self) -> Option<&'a Token<'b>>;
    fn position(&self) -> Position;
    fn next(&self) -> Self;
    fn has_value(&self) -> bool {
        return self.current().is_some();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedIterator<'a, 'b: 'a> {
    underlying: &'a [Token<'b>],
    index: usize,
    pos: Position,
}

impl<'a, 'b> From<&'a [Token<'b>]> for IndexedIterator<'a, 'b> {
    fn from(value: &'a [Token<'b>]) -> Self {
        IndexedIterator {
            underlying: value,
            index: 0,
            pos: if value.len() == 0 {
                Position { line: 0, column: 0 }
            } else {
                value[0].extent.start
            },
        }
    }
}

impl<'a, 'b> TokenIterator<'a, 'b> for IndexedIterator<'a, 'b> {
    fn current(&self) -> Option<&'a Token<'b>> {
        self.underlying.get(self.index)
    }

    fn position(&self) -> Position {
        self.pos
    }

    fn next(&self) -> Self {
        IndexedIterator {
            underlying: self.underlying,
            index: self.index + 1,
            pos: if self.underlying.len() <= self.index + 1 {
                self.underlying[self.index].extent.end
            } else {
                self.underlying[self.index + 1].extent.start
            },
        }
    }
}

#[derive(Debug)]
pub struct ParsingError {
    pub what: String,
    pub position: Position,
}

#[derive(Debug)]
pub struct Parser {
    pub recovered_errors: Vec<ParsingError>,
}

pub type ParsingResult<'a, 'b, T> = Result<(T, IndexedIterator<'a, 'b>), ParsingError>; // For hard errors
pub type PureParsingResult<T> = Result<T, ParsingError>;

trait ParsingFunction<'a, 'b: 'a, T>:
    FnMut(&mut Parser, IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, T>
{
}

impl<'a, 'b: 'a, T, F> ParsingFunction<'a, 'b, T> for F where
    F: FnMut(&mut Parser, IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, T>
{
}

type OperatorsPrecedense = Vec<SyntacticOperator>;

impl Parser {
    pub fn new() -> Self {
        Parser {
            recovered_errors: vec![],
        }
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
                Token {
                    kind:
                        TokenKind::Invalid(InvalidToken {
                            problem,
                            code: TokenIssue::Unexpected,
                        }),
                    extent: Extent { start, .. },
                    ..
                } => {
                    self.report_recovered(problem.clone(), start.clone());
                }
                Token {
                    kind: TokenKind::Comment(_),
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

    fn parse_identifier<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, RawIdentifier> {
        match i.current() {
            Some(token) => {
                if let Token {
                    kind: TokenKind::Identifier(Identifier { name }),
                    ..
                } = token
                {
                    Ok((
                        RawIdentifier {
                            name: (*name).to_owned(),
                        },
                        self.next(i),
                    ))
                } else {
                    Err(ParsingError {
                        what: format!("Identifier expected, {} found", token),
                        position: i.position(),
                    })
                }
            }
            None => Err(ParsingError {
                what: "Identifier expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    fn parse_real_literal<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, RealLiteral> {
        match i.current() {
            Some(token) => {
                if let Token {
                    kind: TokenKind::RealLiteral(TokenRealLiteral { value }),
                    ..
                } = token
                {
                    Ok((
                        RealLiteral {
                            repr: token.lexeme.to_owned(),
                            value: *value,
                        },
                        self.next(i),
                    ))
                } else if let Token {
                    kind:
                        TokenKind::Invalid(InvalidToken {
                            problem,
                            code: TokenIssue::MalformedReal,
                        }),
                    extent: Extent { start, .. },
                    ..
                } = token
                {
                    self.recovered_errors.push(ParsingError {
                        position: start.clone(),
                        what: problem.clone(),
                    });

                    Ok((
                        RealLiteral {
                            repr: token.lexeme.to_owned(),
                            value: f64::NAN,
                        },
                        self.next(i),
                    ))
                } else {
                    Err(ParsingError {
                        what: format!("Real literal expected, {} found", token),
                        position: i.position(),
                    })
                }
            }
            None => Err(ParsingError {
                what: "Real literal expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    fn parse_integer_literal<'a, 'b>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, IntegerLiteral> {
        match i.current() {
            Some(token) => {
                if let Token {
                    kind: TokenKind::IntegerLiteral(TokenIntegerLiteral { value }),
                    ..
                } = token
                {
                    Ok((
                        IntegerLiteral {
                            repr: token.lexeme.to_owned(),
                            value: *value,
                        },
                        self.next(i),
                    ))
                } else if let Token {
                    kind:
                        TokenKind::Invalid(InvalidToken {
                            problem,
                            code: TokenIssue::MalformedReal,
                        }),
                    extent: Extent { start, .. },
                    ..
                } = token
                {
                    self.recovered_errors.push(ParsingError {
                        position: start.clone(),
                        what: problem.clone(),
                    });

                    Ok((
                        IntegerLiteral {
                            repr: token.lexeme.to_owned(),
                            value: 0,
                        },
                        self.next(i),
                    ))
                } else {
                    Err(ParsingError {
                        what: format!("Integer literal expected, {} found", token),
                        position: i.position(),
                    })
                }
            }
            None => Err(ParsingError {
                what: "Integer literal expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    fn parse_bool_literal<'a, 'b>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, BoolLiteral> {
        match i.current() {
            Some(token) => {
                if let Token {
                    kind: TokenKind::BoolLiteral(TokenBoolLiteral { value }),
                    ..
                } = token
                {
                    Ok((
                        if *value {
                            BoolLiteral::True
                        } else {
                            BoolLiteral::False
                        },
                        self.next(i),
                    ))
                } else {
                    Err(ParsingError {
                        what: format!("Bool literal expected, {} found", token),
                        position: i.position(),
                    })
                }
            }
            None => Err(ParsingError {
                what: "Bool literal expected, EOF found".to_owned(),
                position: i.position(),
            }),
        }
    }

    fn parse_known_kind<'a, 'b: 'a>(
        &mut self,
        expected_kind: &TokenKind<'a>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        match i.current() {
            Some(token) => {
                if token.kind == *expected_kind {
                    Ok(((), self.next(i)))
                } else {
                    Err(ParsingError {
                        what: format!(
                            "Token of kind {expected_kind} expected, token {token} found"
                        ),
                        position: token.extent.start,
                    })
                }
            }
            None => Err(ParsingError {
                what: format!("Token of kind {expected_kind} expected, token EOF found"),
                position: i.position(),
            }),
        }
    }

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

    fn parse_one_of<'a, 'b: 'a, T>(
        &mut self,
        mut l: impl ParsingFunction<'a, 'b, T>,
        mut r: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        match l(self, i) {
            Ok(val) => Ok(val),
            Err(first_err) => match r(self, i) {
                res @ Ok(_) => res,
                Err(second_err) => Err(ParsingError {
                    what: format!(
                        "Following parses failed\n{}\n{}",
                        first_err.what, second_err.what
                    ),
                    position: first_err.position,
                }),
            },
        }
    }

    fn parse_some<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Vec<T>> {
        let mut result = Vec::new();
        let first = parser(self, i);

        match first {
            Ok((first_parsed, next)) => {
                i = next;
                result.push(first_parsed);

                while let Ok((parsed, next)) = parser(self, i) {
                    result.push(parsed);
                    i = next;
                }

                Ok((result, i))
            }
            Err(e) => Err(e),
        }
    }

    fn parse_before<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut right: impl ParsingFunction<'a, 'b, ()>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        let (res, next) = parser(self, i)?;
        let (_, next) = right(self, next)?;
        Ok((res, next))
    }

    fn parse_after<'a, 'b: 'a, T>(
        &mut self,
        mut left: impl ParsingFunction<'a, 'b, ()>,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        let (_, next) = left(self, i)?;
        parser(self, next)
    }

    fn parse_between<'a, 'b: 'a, T>(
        &mut self,
        mut left: impl ParsingFunction<'a, 'b, ()>,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut right: impl ParsingFunction<'a, 'b, ()>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, T> {
        let (_, next) = left(self, i)?;
        let (res, next) = parser(self, next)?;
        let (_, next) = right(self, next)?;
        Ok((res, next))
    }

    fn try_parse<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Option<T>> {
        parser(self, i)
            .map(|(val, i)| (Some(val), i))
            .or_else(|_| Ok((None, i)))
    }

    fn parse_many_sep_by<'a, 'b: 'a, T>(
        &mut self,
        mut parser: impl ParsingFunction<'a, 'b, T>,
        mut sep: impl ParsingFunction<'a, 'b, ()>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Vec<T>> {
        let mut result = Vec::new();
        let first = parser(self, i);

        match first {
            Ok((first_parsed, rest)) => {
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
            Err(_) => Ok((Vec::new(), i)),
        }
    }

    fn parse_semicolon<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::Semicolon, i)
    }

    fn parse_comma<'a, 'b: 'a>(&mut self, i: IndexedIterator<'a, 'b>) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::Comma, i)
    }

    fn parse_kw_end<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::End), i)
    }

    fn parse_kw_where<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::Where), i)
    }

    fn parse_left_bracket<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::LeftBracket, i)
    }

    fn parse_right_bracket<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::RightBracket, i)
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
        self.parse_known_kind(&TokenKind::LeftParenthesis, i)
    }

    fn parse_right_parenthesis<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ()> {
        self.parse_known_kind(&TokenKind::RightParenthesis, i)
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

    fn parse_is_claim<'a, 'b: 'a, T, U>(
        &mut self,
        mut l: impl ParsingFunction<'a, 'b, T>,
        mut r: impl ParsingFunction<'a, 'b, U>,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, (T, U)> {
        let (lhs, next) = l(self, i)?;
        let (_, next) = self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::Is), next)?;
        let (rhs, end) = r(self, next)?;
        Ok(((lhs, rhs), end))
    }

    fn parse_field_description<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, FieldDescription> {
        let (ident, next) = self.parse_identifier(i)?;
        let (_, next) = self.parse_known_kind(&TokenKind::Colon, next)?;
        let (t, next) = self.parse_type(next)?;

        Ok((FieldDescription { name: ident, t }, next))
    }

    fn parse_record_desc<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, RecordDescription> {
        let (_, next) = self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::Record), i)?;
        let (fields, next) = self.parse_many(
            |ctx, i| ctx.parse_before(Self::parse_field_description, Self::parse_semicolon, i),
            next,
        )?;

        let (_, next) = self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::End), next)?;
        Ok((RecordDescription { fields }, next))
    }

    fn parse_array_desc<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, ArrayDescription> {
        let (_, next) = self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::Array), i)?;
        let (length, next) =
            self.parse_in_brackets(|ctx, i| ctx.try_parse(Self::parse_expr, i), next)?;
        let (element_type, next) = self.parse_type(next)?;
        Ok((
            ArrayDescription {
                length,
                t: element_type,
            },
            next,
        ))
    }

    fn parse_type<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Type>> {
        self.parse_identifier(i)
            .map(|(ident, next)| (Rc::new(Type::Alias(ident)), next))
            .or_else(|_| {
                self.parse_known_kind(
                    &TokenKind::BuiltinTypename(tokens::BuiltinTypename::Real),
                    i,
                )
                .map(|((), next)| (Rc::new(Type::Real), next))
            })
            .or_else(|_| {
                self.parse_known_kind(
                    &TokenKind::BuiltinTypename(tokens::BuiltinTypename::Integer),
                    i,
                )
                .map(|((), next)| (Rc::new(Type::Int), next))
            })
            .or_else(|_| {
                self.parse_known_kind(
                    &TokenKind::BuiltinTypename(tokens::BuiltinTypename::Boolean),
                    i,
                )
                .map(|((), next)| (Rc::new(Type::Bool), next))
            })
            .or_else(|_| {
                self.parse_array_desc(i)
                    .map(|(arr_desc, i)| (Rc::new(Type::Array(arr_desc)), i))
            })
            .or_else(|_| {
                self.parse_record_desc(i)
                    .map(|(rec_desc, i)| (Rc::new(Type::Record(rec_desc)), i))
            })
            .map_err(|_| ParsingError {
                what: "Type exptected, but not found".to_owned(),
                position: i.position(),
            })
    }

    fn parse_operators<'a, 'b: 'a>(
        &mut self,
        mut ops: impl Iterator<Item = &'static [SyntacticOperator]> + Clone,
        mut atom: impl ParsingFunction<'a, 'b, Rc<Expression>> + Clone,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        match ops.next() {
            Some(operators) => {
                // println!("Parsing operators: {:?}", operators);

                let (mut head, mut next) = self.parse_operators(ops.clone(), atom.clone(), i)?;

                // println!("parsed head {:?}", head);

                while let Some(Token {
                    kind: TokenKind::Operator(op),
                    ..
                }) = next.current()
                {
                    if operators.contains(op) {
                        next = self.next(next);
                        let (rhs, rest) = self.parse_operators(ops.clone(), atom.clone(), next)?;
                        next = rest;
                        head = Rc::new(Expression::Binop {
                            op: *op,
                            lhs: head,
                            rhs,
                        });
                    } else {
                        return Ok((head, next));
                    }
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
                Some(Token {
                    kind: TokenKind::Dot,
                    ..
                }) => {
                    next = self.next(next);
                    let (field_name, rest) = self.parse_identifier(next)?;
                    result = Rc::new(LvalueExpression::Member {
                        lhs: result,
                        member_name: field_name,
                    });
                    next = rest;
                }
                Some(Token {
                    kind: TokenKind::LeftBracket,
                    ..
                }) => {
                    let (index, rest) = self.parse_in_brackets(Self::parse_expr, next)?;
                    result = Rc::new(LvalueExpression::Index { lhs: result, index });
                    next = rest;
                }
                Some(_) => break,
                None => break,
            }
        }

        Ok((result, next))
    }

    fn parse_unop<'a, 'b: 'a>(
        &mut self,
        op: SyntacticOperator,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        self.parse_after(
            |ctx, i| ctx.parse_known_kind(&TokenKind::Operator(op), i),
            Self::parse_atom,
            i,
        )
        .map(|(atom, i)| (Rc::new(Expression::Unop { op, operand: atom }), i))
    }

    fn parse_new<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        self.parse_after(
            |ctx, i| ctx.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::New), i),
            |ctx, i| {
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
                                                ctx.parse_is_claim(
                                                    Self::parse_identifier,
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

                Ok((Rc::new(Expression::New { t, fields }), next))
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
                self.parse_known_kind(&TokenKind::Keyword(tokens::Keyword::Null), i)
                    .map(|((), next)| (Rc::new(Expression::Null), next))
            })
            .or_else(|_| {
                self.parse_real_literal(i)
                    .map(|(literal, next)| (Rc::new(Expression::RealLiteral(literal)), next))
            })
            .or_else(|_| {
                self.parse_integer_literal(i)
                    .map(|(literal, next)| (Rc::new(Expression::IntegerLiteral(literal)), next))
            })
            .or_else(|_| {
                self.parse_bool_literal(i)
                    .map(|(literal, next)| (Rc::new(Expression::BoolLiteral(literal)), next))
            })
            .or_else(|_| self.parse_unop(SyntacticOperator::Neg, i))
            .or_else(|_| self.parse_unop(SyntacticOperator::Sub, i))
            .or_else(|_| self.parse_new(i))
            .or_else(|_| self.parse_in_parentheses(Self::parse_expr, i))
            .or_else(|_| {
                self.parse_lvalue_expr(i)
                    .map(|(lvalue, next)| (Rc::new(Expression::LvalueToRvalue(lvalue)), next))
            })?;

        while let Some(Token {
            kind: TokenKind::Cast,
            ..
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

        // println!("Parsed atom: {:?}", *head);

        Ok((head, next))
    }

    pub fn parse_expr<'a, 'b: 'a>(
        &mut self,
        i: IndexedIterator<'a, 'b>,
    ) -> ParsingResult<'a, 'b, Rc<Expression>> {
        static OPERATORS_PRECEDENCE_TABLE: &[&[SyntacticOperator]] = &[
            &[
                SyntacticOperator::And,
                SyntacticOperator::Or,
                SyntacticOperator::Xor,
            ],
            &[
                SyntacticOperator::Lt,
                SyntacticOperator::Le,
                SyntacticOperator::Neq,
                SyntacticOperator::Eq,
                SyntacticOperator::Gt,
                SyntacticOperator::Ge,
            ],
            &[SyntacticOperator::Add, SyntacticOperator::Sub],
            &[
                SyntacticOperator::Mul,
                SyntacticOperator::Div,
                SyntacticOperator::Mod,
            ],
        ];

        self.parse_operators(
            OPERATORS_PRECEDENCE_TABLE.iter().map(|line| *line),
            Self::parse_atom,
            i,
        )
    }
}
