use core::ops::ControlFlow;

use culpa::throws;

use common::{LoopOrder, RawIdentifier, Real};
use lexer::{
    BuiltinTypename, Identifier as TokenIdentifier, Keyword, Lexeme, Literal as TokenLiteral, Token,
};

mod parser;
mod reporting;
mod tree;
mod types;

#[cfg(test)]
mod tests;

use crate::parser::Error;
pub use crate::{
    parser::{Expected, Parser, TokenKind},
    reporting::{Fatal, FinalError, FinalResult, ParsingError, Recoverable},
    tree::{
        Block, BlockElem, ConstDecl, Declaration, Expression, Literal, LvalueExpression, Operator,
        Program, RoutineBody, RoutineDecl, Statement, TypeDecl, VarDecl,
    },
    types::{ArrayDescription, FieldDescription, RecordDescription, Type},
};

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

impl<'src, I: Iterator<Item = Lexeme<'src>>> Parser<'src, I> {
    #[throws]
    fn eat_identifier(&mut self) -> RawIdentifier {
        let lexeme = self.eat_lexeme(TokenKind::Identifier)?;
        let Token::Identifier(TokenIdentifier { name }) = lexeme.token else {
            unreachable!("We ate `TokenKind::Identifier` but got {lexeme:?}")
        };
        RawIdentifier {
            name: name.to_owned(),
        }
    }

    #[throws]
    fn eat_literal(&mut self) -> Literal {
        let lexeme = self.eat_lexeme(TokenKind::Literal)?;
        let Token::Literal(literal) = lexeme.token else {
            unreachable!("We ate `TokenKind::Literal` but got {lexeme:?}")
        };
        let pos = lexeme.extent.start;
        match literal {
            TokenLiteral::Integer(value) => Literal::Integer {
                value: value.unwrap_or_else(|e| {
                    self.recover(Recoverable::MalformedInteger(e), pos);
                    0
                }),
                repr: lexeme.text.to_owned(),
            },
            TokenLiteral::Real(value) => Literal::Real {
                value: value.unwrap_or_else(|e| {
                    self.recover(Recoverable::MalformedReal(e), pos);
                    Real::NAN
                }),
                repr: lexeme.text.to_owned(),
            },
            TokenLiteral::Bool(value) => Literal::Bool { value },
        }
    }

    #[throws]
    fn parse_array_type(&mut self) -> ArrayDescription {
        self.eat(Keyword::Array)?;
        self.eat(Token::LeftBracket)?;
        let length = if self.check(Token::RightBracket) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.eat(Token::RightBracket)?;
        let t = self.parse_type()?;
        ArrayDescription {
            t: Box::new(t),
            length,
        }
    }

    #[throws]
    fn parse_record_type(&mut self) -> RecordDescription {
        self.eat(Keyword::Record)?;
        let mut fields = vec![];
        while !self.try_eat(Keyword::End) {
            self.eat(Keyword::Var)?;
            let name = self.eat_identifier()?;
            self.eat(Keyword::Is)?;
            let t = self.parse_type()?;
            self.eat(Token::Semicolon)?;
            fields.push(FieldDescription { name, t });
        }
        RecordDescription { fields }
    }

    #[throws]
    fn parse_in_parens_comma_sep<T>(
        &mut self,
        parser: fn(&mut Self) -> Result<T, Error>,
    ) -> Vec<T> {
        self.eat(Token::LeftParenthesis)?;
        let mut result = vec![];
        while !self.try_eat(Token::RightParenthesis) {
            result.push(parser(self)?);
            if self.try_eat(Token::RightParenthesis) {
                break;
            }
            self.eat(Token::Comma)?;
        }
        result
    }

    #[throws]
    fn parse_type(&mut self) -> Type {
        if self.try_eat(BuiltinTypename::Integer) {
            Type::Int
        } else if self.try_eat(BuiltinTypename::Real) {
            Type::Real
        } else if self.try_eat(BuiltinTypename::Boolean) {
            Type::Bool
        } else if self.check(Keyword::Array) {
            Type::Array(self.parse_array_type()?)
        } else if self.check(Keyword::Record) {
            Type::Record(self.parse_record_type()?)
        } else {
            Type::Alias(self.eat_identifier()?)
        }
    }

    #[throws]
    fn parse_colon_type(&mut self) -> Option<Type> {
        if self.try_eat(Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        }
    }

    #[throws]
    fn parse_var_decl(&mut self) -> VarDecl {
        self.eat(Keyword::Var)?;
        let name = self.eat_identifier()?;
        let t = self.parse_colon_type()?;
        let initialiser = if self.try_eat(Keyword::Is) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.eat(Token::Semicolon)?;
        VarDecl {
            name,
            t,
            initialiser,
        }
    }

    #[throws]
    fn parse_const_decl(&mut self) -> ConstDecl {
        self.eat(Keyword::Constant)?;
        let name = self.eat_identifier()?;
        let t = self.parse_colon_type()?;
        self.eat(Keyword::Is)?;
        let initialiser = self.parse_expr()?;
        self.eat(Token::Semicolon)?;
        ConstDecl {
            name,
            t,
            initialiser,
        }
    }

    #[throws]
    fn parse_type_decl(&mut self) -> TypeDecl {
        self.eat(Keyword::Type)?;
        let name = self.eat_identifier()?;
        self.eat(Keyword::Is)?;
        let t = self.parse_type()?;
        self.eat(Token::Semicolon)?;
        TypeDecl { name, t }
    }

    #[throws]
    fn parse_lvalue(&mut self, ident: RawIdentifier) -> LvalueExpression {
        let mut current = LvalueExpression::Identifier(ident);
        loop {
            if self.try_eat(Token::Dot) {
                let member_name = self.eat_identifier()?;
                current = LvalueExpression::Member {
                    lhs: current.into(),
                    member_name,
                };
            } else if self.try_eat(Token::LeftBracket) {
                let index = Box::new(self.parse_expr()?);
                self.eat(Token::RightBracket)?;
                current = LvalueExpression::Index {
                    lhs: current.into(),
                    index,
                };
            } else {
                break current;
            }
        }
    }

    #[throws]
    fn parse_new(&mut self) -> Expression {
        self.eat(Keyword::New)?;
        let array_length = if self.try_eat(Token::LeftBracket) {
            let len = self.parse_expr()?;
            self.eat(Token::RightBracket)?;
            Some(Box::new(len))
        } else {
            None
        };
        let t = Box::new(self.parse_type()?);
        let fields = if self.try_eat(Keyword::Where) {
            let mut fields = vec![];
            while !self.try_eat(Keyword::End) {
                let name = self.eat_identifier()?;
                self.eat(Keyword::Is)?;
                let init = self.parse_expr()?;
                self.eat(Token::Semicolon)?;
                fields.push((name, init));
            }
            Some(fields)
        } else {
            None
        };
        Expression::New {
            t,
            fields,
            array_length,
        }
    }

    #[throws]
    fn try_parse_unop(&mut self, op: Operator) -> Option<Expression> {
        if self.try_eat(op) {
            Some(Expression::UnOp {
                op,
                operand: Box::new(self.parse_atom()?),
            })
        } else {
            None
        }
    }

    #[throws]
    fn parse_atom(&mut self) -> Expression {
        let mut result = if self.try_eat(Token::LeftParenthesis) {
            let res = self.parse_expr()?;
            self.eat(Token::RightParenthesis)?;
            res
        } else if self.try_eat(Keyword::Null) {
            Expression::Null
        } else if self.check(Keyword::New) {
            self.parse_new()?
        } else if self.check(TokenKind::Literal) {
            Expression::Literal(self.eat_literal()?)
        } else if let Some(unop) = self.try_parse_unop(Operator::Not)? {
            unop
        } else if let Some(unop) = self.try_parse_unop(Operator::Minus)? {
            unop
        } else {
            let ident = self.eat_identifier()?;
            if self.check(Token::LeftParenthesis) {
                Expression::Call {
                    callee: ident,
                    args: self.parse_in_parens_comma_sep(Self::parse_expr)?,
                }
            } else {
                Expression::LvalueToRvalue(self.parse_lvalue(ident)?)
            }
        };

        while self.try_eat(Token::Cast) {
            let ty = self.parse_type()?;
            result = Expression::Cast {
                operand: Box::new(result),
                target: Box::new(ty),
            }
        }
        result
    }

    #[throws]
    fn parse_binops(&mut self, operators: &[&[Operator]]) -> Expression {
        let Some((operators, rest)) = operators.split_first() else {
            return self.parse_atom()?;
        };
        let mut lhs = self.parse_binops(rest)?;
        'level: loop {
            for op in operators.iter().copied() {
                if self.try_eat(op) {
                    let rhs = Box::new(self.parse_binops(rest)?);
                    lhs = Expression::BinOp {
                        op,
                        lhs: Box::new(lhs),
                        rhs,
                    };
                    continue 'level;
                }
            }
            break;
        }
        lhs
    }

    #[throws]
    pub fn parse_expr(&mut self) -> Expression {
        self.parse_binops(OPERATORS_PRECEDENCE_TABLE)?
    }

    #[throws]
    fn parse_if(&mut self) -> Statement {
        self.eat(Keyword::If)?;
        let condition = self.parse_expr()?;
        self.eat(Keyword::Then)?;
        let (on_true, found_else) = self.parse_block_until(|this| {
            if this.try_eat(Keyword::Else) {
                ControlFlow::Break(true)
            } else if this.try_eat(Keyword::End) {
                ControlFlow::Break(false)
            } else {
                ControlFlow::Continue(())
            }
        })?;
        let on_false = if found_else {
            Some(self.parse_block_until_kw(Keyword::End)?)
        } else {
            None
        };
        Statement::If {
            condition,
            on_true,
            on_false,
        }
    }

    #[throws]
    fn parse_loop(&mut self) -> Block {
        self.eat(Keyword::Loop)?;
        self.parse_block_until_kw(Keyword::End)?
    }

    #[throws]
    fn parse_statement(&mut self) -> Statement {
        if self.check(Keyword::If) {
            self.parse_if()?
        } else if self.try_eat(Keyword::While) {
            let condition = self.parse_expr()?;
            let body = self.parse_loop()?;
            Statement::While { condition, body }
        } else if self.try_eat(Keyword::For) {
            let counter = self.eat_identifier()?;
            self.eat(Keyword::In)?;

            let from = self.parse_expr()?;
            self.eat(Token::RangeSymbol)?;
            let to = if self.check(Keyword::Loop) || self.check(Keyword::Reverse) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            let order = if self.try_eat(Keyword::Reverse) {
                LoopOrder::Reversed
            } else {
                LoopOrder::Direct
            };
            let body = self.parse_loop()?;
            Statement::For {
                counter,
                from,
                to,
                order,
                body,
            }
        } else if self.try_eat(Keyword::Return) {
            let value = self.parse_expr()?;
            self.eat(Token::Semicolon)?;
            Statement::Return { value }
        } else if self.try_eat(Keyword::Print) {
            let value = self.parse_expr()?;
            self.eat(Token::Semicolon)?;
            Statement::Print { value }
        } else if let Some(Lexeme { extent, .. }) = self.try_eat_lexeme(Keyword::Panic) {
            self.eat(Token::Semicolon)?;
            Statement::Panic { pos: extent.start }
        } else if let Some(Lexeme { extent, .. }) = self.try_eat_lexeme(Keyword::Assert) {
            let value = self.parse_expr()?;
            self.eat(Token::Semicolon)?;
            Statement::Assert {
                value,
                pos: extent.start,
            }
        } else {
            let ident = self.eat_identifier()?;
            let stmt = if self.check(Token::LeftParenthesis) {
                let args = self.parse_in_parens_comma_sep(Self::parse_expr)?;
                Statement::Expr(Expression::Call {
                    callee: ident,
                    args,
                })
            } else {
                let lhs = self.parse_lvalue(ident)?;
                self.eat(Token::Assignment)?;
                let rhs = self.parse_expr()?;
                Statement::Assignment { lhs, rhs }
            };
            self.eat(Token::Semicolon)?;
            stmt
        }
    }

    #[throws]
    fn parse_block_elem(&mut self) -> BlockElem {
        if self.check(Keyword::Var) {
            BlockElem::VarDecl(self.parse_var_decl()?)
        } else if self.check(Keyword::Constant) {
            BlockElem::ConstDecl(self.parse_const_decl()?)
        } else if self.check(Keyword::Type) {
            BlockElem::TypeDecl(self.parse_type_decl()?)
        } else {
            BlockElem::Stmt(self.parse_statement()?)
        }
    }

    #[throws]
    fn parse_block_until<T>(
        &mut self,
        callback: impl Fn(&mut Self) -> ControlFlow<T>,
    ) -> (Block, T) {
        let mut elems = vec![];
        loop {
            match callback(self) {
                ControlFlow::Continue(()) => elems.push(self.parse_block_elem()?),
                ControlFlow::Break(t) => break (Block(elems), t),
            }
        }
    }

    #[throws]
    fn parse_block_until_kw(&mut self, kw: Keyword) -> Block {
        let (block, ()) = self.parse_block_until(|this| {
            if this.try_eat(kw) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
        block
    }

    #[throws]
    fn parse_routine_decl(&mut self) -> RoutineDecl {
        self.eat(Keyword::Routine)?;
        let name = self.eat_identifier()?;
        let arguments = self.parse_in_parens_comma_sep(|this| {
            let name = this.eat_identifier()?;
            this.eat(Token::Colon)?;
            let ty = this.parse_type()?;
            Ok((name, ty))
        })?;
        let return_type = self.parse_colon_type()?;
        let body = if self.try_eat(Keyword::Is) {
            Some(RoutineBody::Block(self.parse_block_until_kw(Keyword::End)?))
        } else if self.try_eat(Token::RightArrow) {
            let body = self.parse_expr()?;
            self.eat(Token::Semicolon)?;
            Some(RoutineBody::Expression(body))
        } else {
            self.eat(Token::Semicolon)?;
            None
        };
        RoutineDecl {
            name,
            arguments,
            return_type,
            body,
        }
    }

    #[throws]
    fn parse_declaration(&mut self) -> Declaration {
        if self.check(Keyword::Var) {
            Declaration::Var(self.parse_var_decl()?)
        } else if self.check(Keyword::Constant) {
            Declaration::Const(self.parse_const_decl()?)
        } else if self.check(Keyword::Type) {
            Declaration::Type(self.parse_type_decl()?)
        } else {
            Declaration::Routine(self.parse_routine_decl()?)
        }
    }

    #[throws]
    pub fn parse_program(&mut self) -> Program {
        let mut result = vec![];
        while !self.eof() {
            result.push(self.parse_declaration()?);
        }
        Program(result)
    }
}

#[expect(single_use_lifetimes, reason = "Cannot use `'_` in impl Trait")]
pub fn parse_program<'src>(tokens: impl IntoIterator<Item = Lexeme<'src>>) -> FinalResult<Program> {
    let mut parser = Parser::new(tokens.into_iter());
    let program = parser.parse_program();
    parser.finish(program)
}
