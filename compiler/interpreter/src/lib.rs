#![expect(dead_code, reason = "WIP")]

use core::iter;
use std::{collections::HashMap, fmt::Debug, io::Write, rc::Rc};

use anyhow::{Context as _, Error, bail, ensure};
use ast::{
    ArrayDescription, Binding, Block, BlockElem, Decl, Expression, IntegerLiteral,
    LvalueExpression, Program, RealLiteral, Routine, RoutineBody, RoutineDecl, SimpleBinding,
    SimpleDecl, Type, VarDecl,
};
use common::{
    Identifier, Integer, LoopOrder, RawIdentifier, Real, integer_to_real,
    operators::{
        BoolBinOp, EqBinOp, IntBinOp, RealBinOp, SemanticBinaryOperator, SemanticUnaryOperator,
    },
    real_to_integer,
};
use culpa::{throw, throws};
use indexed_arena::{Arena, Idx};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default, PartialEq)]
enum Value<'a> {
    #[default]
    Null,
    Bool(bool),
    Integer(Integer),
    Real(Real),
    Array {
        elements: Vec<Address<'a>>,
    },
    Struct {
        fields: HashMap<&'a str, Address<'a>>,
    },
}

type MemoryIndex = usize;
type Memory<'a> = Arena<Value<'a>, MemoryIndex>;
type Address<'a> = Idx<Value<'a>, MemoryIndex>;
type Bindings<'a> = HashMap<&'a Identifier, Address<'a>>;

#[derive(Debug)]
struct Interpreter<'a, W: Write> {
    out: W,
    program: &'a Program,
    heap: Memory<'a>,
}

// Sadly, Rust's `?` does not work well when you try to combine a `ControlFlow` and a `Result`.
// This is a hack around it.
enum BlockError<'a> {
    Return(Value<'a>),
    Error(Error),
}

impl From<Error> for BlockError<'_> {
    fn from(e: Error) -> Self {
        Self::Error(e)
    }
}

trait IdentSelector: Debug {
    #[must_use]
    fn matches(&self, ident: &Identifier) -> bool;
}

impl IdentSelector for str {
    fn matches(&self, ident: &Identifier) -> bool {
        self == ident.raw.name
    }
}

impl IdentSelector for Identifier {
    fn matches(&self, ident: &Identifier) -> bool {
        self == ident
    }
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "essentially an if-let but with better error reporting"
)]
impl<'a, W: Write> Interpreter<'a, W> {
    #[throws]
    fn bool_expression(&mut self, bindings: &mut Bindings<'a>, expression: &Expression) -> bool {
        match self.expression(bindings, expression)? {
            Value::Bool(value) => value,
            value => bail!("Expected bool, got {value:?}"),
        }
    }

    #[throws]
    fn integer_expression(
        &mut self,
        bindings: &mut Bindings<'a>,
        expression: &Expression,
    ) -> Integer {
        match self.expression(bindings, expression)? {
            Value::Integer(value) => value,
            value => bail!("Expected integer, got {value:?}"),
        }
    }

    #[throws]
    fn real_expression(&mut self, bindings: &mut Bindings<'a>, expression: &Expression) -> Real {
        match self.expression(bindings, expression)? {
            Value::Real(value) => value,
            value => bail!("Expected real, got {value:?}"),
        }
    }

    #[throws]
    fn array_expression(
        &mut self,
        bindings: &mut Bindings<'a>,
        expression: &Expression,
    ) -> Vec<Address<'a>> {
        match self.expression(bindings, expression)? {
            Value::Array { elements } => elements,
            value => bail!("Expected array, got {value:?}"),
        }
    }
}

impl<'a, W: Write> Interpreter<'a, W> {
    #[throws]
    fn find_routine<I: IdentSelector + ?Sized>(&self, routine: &I) -> &'a Routine {
        self.program
            .0
            .iter()
            .find_map(|Binding { name, decl }| {
                if let Decl::Routine(RoutineDecl::Full(r)) = decl
                    && routine.matches(name)
                {
                    Some(r)
                } else {
                    None
                }
            })
            .with_context(|| format!("Could not find routine {routine:?}"))?
    }

    #[throws]
    fn lvalue(&mut self, bindings: &mut Bindings<'a>, lvalue: &LvalueExpression) -> Address<'a> {
        match lvalue {
            LvalueExpression::Identifier(ident) => *bindings
                .get(ident)
                .with_context(|| format!("Could not find variable {ident:?}"))?,

            LvalueExpression::Member { lhs, member_name } => {
                let lhs = self.lvalue(bindings, lhs)?;
                match &self.heap[lhs] {
                    Value::Struct { fields } => *fields
                        .get(member_name.name.as_str())
                        .with_context(|| format!("No field {member_name:?}"))?,
                    e @ (Value::Null
                    | Value::Bool(_)
                    | Value::Integer(_)
                    | Value::Real(_)
                    | Value::Array { .. }) => bail!("{e:?} is not a struct"),
                }
            }
            LvalueExpression::Index { lhs, index } => {
                let lhs = self.lvalue(bindings, lhs)?;
                let index = self.integer_expression(&mut *bindings, index)?;
                match &self.heap[lhs] {
                    Value::Array { elements } => {
                        let len = elements.len();
                        *index
                            .saturating_sub(1)
                            .try_into()
                            .ok()
                            .and_then(|i: usize| elements.get(i))
                            .with_context(|| {
                                format!("Index {index} is out of bounds for array of length {len}")
                            })?
                    }
                    e @ (Value::Null
                    | Value::Bool(_)
                    | Value::Integer(_)
                    | Value::Real(_)
                    | Value::Struct { .. }) => bail!("{e:?} is not an array"),
                }
            }
        }
    }

    #[throws]
    fn binop(
        &mut self,
        bindings: &mut Bindings<'a>,
        op: SemanticBinaryOperator,
        lhs: &Expression,
        rhs: &Expression,
    ) -> Value<'a> {
        match op {
            SemanticBinaryOperator::Eq(op) => {
                let lhs = self.expression(bindings, lhs)?;
                let rhs = self.expression(bindings, rhs)?;
                Value::Bool(match op {
                    EqBinOp::Eq => lhs == rhs,
                    EqBinOp::Ne => lhs != rhs,
                })
            }
            SemanticBinaryOperator::Real(op) => {
                let lhs = self.real_expression(bindings, lhs)?;
                let rhs = self.real_expression(bindings, rhs)?;
                match op {
                    RealBinOp::Add => Value::Real(lhs + rhs),
                    RealBinOp::Sub => Value::Real(lhs - rhs),
                    RealBinOp::Mul => Value::Real(lhs * rhs),
                    RealBinOp::Div => Value::Real(lhs / rhs),
                    RealBinOp::Le => Value::Bool(lhs <= rhs),
                    RealBinOp::Lt => Value::Bool(lhs < rhs),
                    RealBinOp::Gt => Value::Bool(lhs > rhs),
                    RealBinOp::Ge => Value::Bool(lhs >= rhs),
                }
            }
            SemanticBinaryOperator::Int(op) => {
                let lhs = self.integer_expression(bindings, lhs)?;
                let rhs = self.integer_expression(bindings, rhs)?;
                match op {
                    IntBinOp::Add => Value::Integer(lhs + rhs),
                    IntBinOp::Sub => Value::Integer(lhs - rhs),
                    IntBinOp::Mul => Value::Integer(lhs * rhs),
                    IntBinOp::Div => Value::Integer(lhs / rhs),
                    IntBinOp::Mod => Value::Integer(lhs % rhs),
                    IntBinOp::Le => Value::Bool(lhs <= rhs),
                    IntBinOp::Lt => Value::Bool(lhs < rhs),
                    IntBinOp::Gt => Value::Bool(lhs > rhs),
                    IntBinOp::Ge => Value::Bool(lhs >= rhs),
                }
            }
            SemanticBinaryOperator::Bool(op) => Value::Bool(match op {
                BoolBinOp::And => {
                    self.bool_expression(bindings, lhs)? && self.bool_expression(bindings, rhs)?
                }
                BoolBinOp::Or => {
                    self.bool_expression(bindings, lhs)? || self.bool_expression(bindings, rhs)?
                }
                BoolBinOp::Xor => {
                    self.bool_expression(bindings, lhs)? ^ self.bool_expression(bindings, rhs)?
                }
            }),
        }
    }

    #[expect(
        clippy::wrong_self_convention,
        clippy::new_ret_no_self,
        reason = "`new` as in `Expression::New`"
    )]
    #[throws]
    fn new(
        &mut self,
        _bindings: &mut Bindings<'a>,
        ty: &Type,
        fields: Option<&[(RawIdentifier, Rc<Expression>)]>,
    ) -> Value<'a> {
        match ty {
            &Type::Array(ArrayDescription { t: _, length }) => Value::Array {
                elements: match (length, fields) {
                    (Some(n), None) => iter::repeat_with(|| self.heap.alloc(Value::Null))
                        .take(n)
                        .collect(),
                    _ => todo!(),
                },
            },

            Type::Alias(_) => todo!(),
            Type::Record(_) => todo!(),

            Type::Int | Type::Real | Type::Bool | Type::Null | Type::Unit => {
                bail!("Unsupported type for `new` expression: {ty}")
            }
        }
    }

    #[throws]
    fn expression(&mut self, bindings: &mut Bindings<'a>, expression: &Expression) -> Value<'a> {
        match expression {
            Expression::Null => Value::Null,
            &Expression::IntegerLiteral(IntegerLiteral { repr: _, value }) => Value::Integer(value),
            &Expression::RealLiteral(RealLiteral { repr: _, value }) => Value::Real(value),
            &Expression::BoolLiteral(l) => Value::Bool(l.into()),

            Expression::LvalueToRvalue(lvalue) => {
                let addr = self.lvalue(bindings, lvalue)?;
                self.heap[addr].clone()
            }

            Expression::Call { callee, args } => {
                let args: Vec<_> = args
                    .iter()
                    .map(|arg| self.expression(bindings, arg))
                    .collect::<Result<_, _>>()?;
                self.call(callee, args)?
            }
            Expression::BinOp { op, lhs, rhs } => self.binop(bindings, *op, lhs, rhs)?,
            Expression::UnOp { op, operand } => match op {
                SemanticUnaryOperator::IntNeg => {
                    Value::Integer(-self.integer_expression(bindings, operand)?)
                }
                SemanticUnaryOperator::RealNeg => {
                    Value::Real(-self.real_expression(bindings, operand)?)
                }
                SemanticUnaryOperator::BoolNeg => {
                    Value::Bool(!self.bool_expression(bindings, operand)?)
                }
            },
            Expression::Cast { operand, target: _ } => self.expression(bindings, operand)?,
            Expression::New { t, fields } => self.new(bindings, t, fields.as_deref())?,

            Expression::LengthOf { arr } => Value::Integer(
                self.array_expression(bindings, arr)?
                    .len()
                    .try_into()
                    .expect("Array too long"),
            ),

            Expression::IntToBool(expression) => {
                Value::Bool(self.integer_expression(bindings, expression)? != 0)
            }
            Expression::BoolToInt(expression) => {
                Value::Integer(self.bool_expression(bindings, expression)?.into())
            }
            Expression::RealToInt(expression) => {
                Value::Integer(real_to_integer(self.real_expression(bindings, expression)?))
            }
            Expression::IntToReal(expression) => Value::Real(integer_to_real(
                self.integer_expression(bindings, expression)?,
            )),
        }
    }

    #[throws]
    fn print(&mut self, value: &Value<'_>) {
        match value {
            Value::Bool(value) => writeln!(self.out, "{value}"),
            Value::Integer(value) => writeln!(self.out, "{value}"),
            &Value::Real(value) => {
                if value.fract() == 0.0 {
                    writeln!(self.out, "{value:?}")
                } else {
                    writeln!(self.out, "{value}")
                }
            }
            Value::Null => writeln!(self.out, "null"),
            Value::Array { .. } => todo!(),
            Value::Struct { .. } => todo!(),
        }
        .context("IO error")?
    }

    #[throws(BlockError<'a>)]
    fn block(&mut self, bindings: &mut Bindings<'a>, block: &'a Block) {
        let Block(stmts) = block;
        for stmt in stmts {
            match stmt {
                BlockElem::Decl(SimpleBinding { name, decl }) => match decl {
                    SimpleDecl::Type(_) => {}
                    SimpleDecl::Var(VarDecl {
                        t: _,
                        initialiser,
                        relative_location: _,
                    }) => match initialiser {
                        None => {}
                        Some(e) => {
                            let e = self.expression(bindings, e)?;
                            let e = self.heap.alloc(e);
                            if let Some(prev) = bindings.insert(name, e) {
                                eprintln!("Discarding previous value for {name}: {prev:?}")
                            }
                        }
                    },
                },

                BlockElem::Stmt(stmt) => match stmt {
                    ast::Statement::Assignment { lhs, rhs } => {
                        let lhs = self.lvalue(bindings, lhs)?;
                        let rhs = self.expression(bindings, rhs)?;
                        self.heap[lhs] = rhs;
                    }

                    ast::Statement::While { condition, body } => {
                        while self.bool_expression(bindings, condition)? {
                            self.block(bindings, body)?
                        }
                    }

                    ast::Statement::Expr(expression) => {
                        let value: Value<'a> = self.expression(bindings, expression)?;
                        drop(value);
                    }

                    ast::Statement::If {
                        condition,
                        on_true,
                        on_false,
                    } => {
                        if self.bool_expression(bindings, condition)? {
                            self.block(bindings, on_true)?
                        } else if let Some(on_false) = on_false {
                            self.block(bindings, on_false)?
                        }
                    }

                    ast::Statement::For {
                        counter,
                        lower_bound,
                        upper_bound,
                        order,
                        body,
                    } => {
                        let lower = self.integer_expression(bindings, lower_bound)?;
                        let upper = self.integer_expression(bindings, upper_bound)?;
                        let range: &mut dyn Iterator<Item = Integer> = match order {
                            LoopOrder::Direct => &mut (lower..=upper),
                            LoopOrder::Reversed => &mut (upper..=lower).rev(),
                        };
                        for value in range {
                            let _: Option<Address<'a>> =
                                bindings.insert(counter, self.heap.alloc(Value::Integer(value)));
                            self.block(bindings, body)?
                        }
                    }

                    ast::Statement::ForEach {
                        counter,
                        collection,
                        order,
                        body,
                    } => {
                        let mut collection = self.array_expression(bindings, collection)?;
                        match order {
                            LoopOrder::Direct => {}
                            LoopOrder::Reversed => collection.reverse(),
                        }
                        for value in collection {
                            let _: Option<Address<'a>> = bindings.insert(counter, value);
                            self.block(bindings, body)?
                        }
                    }

                    ast::Statement::Print { value } => {
                        let value = self.expression(bindings, value)?;
                        self.print(&value)?
                    }

                    ast::Statement::Return { value } => {
                        throw!(BlockError::Return(self.expression(bindings, value)?))
                    }
                },
            }
        }
    }

    #[throws]
    fn call<I: IdentSelector + ?Sized>(&mut self, routine: &I, args: Vec<Value<'a>>) -> Value<'a> {
        let Routine {
            signature: _,
            args_bindings,
            body,
        } = self.find_routine(routine)?;
        {
            let expected = args_bindings.len();
            let actual = args.len();
            ensure!(
                expected == actual,
                "Expected {expected} for {routine:?} args but got {actual}"
            )
        }
        let mut bindings: Bindings<'a> = args_bindings
            .iter()
            .zip(args)
            .map(|(Binding { name, decl: _ }, value)| (name, self.heap.alloc(value)))
            .collect();
        match body {
            RoutineBody::Expression(expression) => self.expression(&mut bindings, expression),
            RoutineBody::Block(block) => match self.block(&mut bindings, block) {
                Ok(()) => Ok(Value::Null),
                Err(BlockError::Return(v)) => Ok(v),
                Err(BlockError::Error(e)) => Err(e),
            },
        }
        .with_context(|| format!("Error interpreting {routine:?}"))?
    }

    #[throws]
    fn run(mut self) {
        let value = self.call("main", vec![])?;
        let Value::Null = value else {
            bail!("main returned non-null value: {value:?}")
        };
    }
}

pub fn interpret(out: impl Write, program: &Program) -> anyhow::Result<()> {
    Interpreter {
        out,
        program,
        heap: Default::default(),
    }
    .run()
}
