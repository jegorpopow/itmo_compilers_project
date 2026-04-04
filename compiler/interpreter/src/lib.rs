use core::{fmt, iter};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, Write},
    rc::Rc,
};

use ast::{
    ArrayDescription, BinaryOperator, Binding, Block, BlockElem, BoolBinOp, Decl, EqBinOp,
    Expression, FieldDescription, IntBinOp, IntegerLiteral, LocalBinding, LocalDecl,
    LvalueExpression, Program, RealBinOp, RealLiteral, RecordDescription, Routine, RoutineBody,
    RoutineDecl, Type, TypeDecl, UnaryOperator, VarDecl,
};
use common::{
    Identifier, Integer, LoopOrder, Position, RawIdentifier, Real, integer_to_real, real_to_integer,
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
        fields: HashMap<&'a RawIdentifier, Address<'a>>,
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
    globals: Bindings<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeError {
    IndexOutOfBounds { index: Integer, len: usize },
    Panic { pos: Position },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexOutOfBounds { index, len } => {
                write!(
                    f,
                    "Index {index} is out of bounds for array of length {len}"
                )
            }
            Self::Panic { pos } => write!(f, "Panic @ {pos}"),
        }
    }
}

impl core::error::Error for RuntimeError {}

// Sadly, Rust's `?` does not work well when you try to combine a `ControlFlow` and a `Result`.
// This is a hack around it.
enum BlockError<'a> {
    Return(Value<'a>),
    Error(RuntimeError),
}

impl From<RuntimeError> for BlockError<'_> {
    fn from(e: RuntimeError) -> Self {
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
    #[throws(RuntimeError)]
    fn bool_expression(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) -> bool {
        match self.expression(bindings, expression)? {
            Value::Bool(value) => value,
            value => unreachable!("Expected bool, got {value:?}"),
        }
    }

    #[throws(RuntimeError)]
    fn integer_expression(
        &mut self,
        bindings: &mut Bindings<'a>,
        expression: &'a Expression,
    ) -> Integer {
        match self.expression(bindings, expression)? {
            Value::Integer(value) => value,
            value => unreachable!("Expected integer, got {value:?}"),
        }
    }

    #[throws(RuntimeError)]
    fn real_expression(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) -> Real {
        match self.expression(bindings, expression)? {
            Value::Real(value) => value,
            value => unreachable!("Expected real, got {value:?}"),
        }
    }

    #[throws(RuntimeError)]
    fn array_expression(
        &mut self,
        bindings: &mut Bindings<'a>,
        expression: &'a Expression,
    ) -> Vec<Address<'a>> {
        match self.expression(bindings, expression)? {
            Value::Array { elements } => elements,
            value => unreachable!("Expected array, got {value:?}"),
        }
    }
}

impl<'a, W: Write> Interpreter<'a, W> {
    fn default_value_for_type(&self, ty: &Type) -> Value<'a> {
        match ty {
            Type::Int => Value::Integer(0),
            Type::Real => Value::Real(0.0),
            Type::Bool => Value::Bool(false),
            Type::Alias(ty) => self.default_value_for_type(self.find_type(ty)),
            Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => Value::Null,
        }
    }

    fn find_routine<I: IdentSelector + ?Sized>(&self, routine: &I) -> &'a Routine {
        let Some(routine) = self.program.0.iter().find_map(|Binding { name, decl }| {
            if let Decl::Routine(RoutineDecl::Full(r)) = decl
                && routine.matches(name)
            {
                Some(r)
            } else {
                None
            }
        }) else {
            unreachable!("Could not find routine {routine:?}")
        };
        routine
    }

    fn find_type(&self, ty: &Identifier) -> &'a Type {
        let Some(ty) = self.program.0.iter().find_map(|Binding { name, decl }| {
            if let Decl::Type(TypeDecl::Full {
                prescribed: _,
                effective,
            }) = decl
                && name == ty
            {
                Some(effective.as_ref())
            } else {
                None
            }
        }) else {
            unreachable!("Could not find type {ty:?}")
        };
        ty
    }

    #[throws(RuntimeError)]
    fn lvalue(&mut self, bindings: &mut Bindings<'a>, lvalue: &'a LvalueExpression) -> Address<'a> {
        match lvalue {
            LvalueExpression::Identifier(ident) => {
                let Some(&var) = bindings.get(ident) else {
                    unreachable!("Could not find variable {ident:?}")
                };

                var
            }
            LvalueExpression::Member { lhs, member_name } => {
                let lhs = self.lvalue(bindings, lhs)?;
                match &self.heap[lhs] {
                    Value::Struct { fields } => {
                        let Some(&val) = fields.get(&member_name) else {
                            unreachable!("No field {member_name:?}")
                        };
                        val
                    }

                    e @ (Value::Null
                    | Value::Bool(_)
                    | Value::Integer(_)
                    | Value::Real(_)
                    | Value::Array { .. }) => unreachable!("{e:?} is not a struct"),
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
                            .ok_or(RuntimeError::IndexOutOfBounds { index, len })?
                    }
                    e @ (Value::Null
                    | Value::Bool(_)
                    | Value::Integer(_)
                    | Value::Real(_)
                    | Value::Struct { .. }) => unreachable!("{e:?} is not an array"),
                }
            }
        }
    }

    #[throws(RuntimeError)]
    fn binop(
        &mut self,
        bindings: &mut Bindings<'a>,
        op: BinaryOperator,
        lhs: &'a Expression,
        rhs: &'a Expression,
    ) -> Value<'a> {
        match op {
            BinaryOperator::Eq(op) => {
                let lhs = self.expression(bindings, lhs)?;
                let rhs = self.expression(bindings, rhs)?;
                Value::Bool(match op {
                    EqBinOp::Eq => lhs == rhs,
                    EqBinOp::Ne => lhs != rhs,
                })
            }
            BinaryOperator::Real(op) => {
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
            BinaryOperator::Int(op) => {
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
            BinaryOperator::Bool(op) => Value::Bool(match op {
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
    #[throws(RuntimeError)]
    fn new(
        &mut self,
        bindings: &mut Bindings<'a>,
        ty: &'a Type,
        field_values: Option<&'a [(RawIdentifier, Rc<Expression>)]>,
    ) -> Value<'a> {
        match ty {
            Type::Array(ArrayDescription { t, length }) => Value::Array {
                elements: match (*length, field_values) {
                    (Some(n), None) => {
                        let value = self.default_value_for_type(t);
                        iter::repeat_n(value, n)
                            .map(|v| self.heap.alloc(v))
                            .collect()
                    }
                    _ => todo!(),
                },
            },

            Type::Alias(ty) => self.new(bindings, self.find_type(ty), field_values)?,
            Type::Record(RecordDescription {
                fields: expected_fields,
            }) => {
                let fields: HashMap<&'a RawIdentifier, Address<'a>> = expected_fields
                    .iter()
                    .map(|FieldDescription { name, t }| {
                        let val = self.default_value_for_type(t);
                        (name, self.heap.alloc(val))
                    })
                    .collect();
                for (name, val) in field_values.unwrap_or_default() {
                    let Some(&addr) = fields.get(name) else {
                        unreachable!("No field {name:?} in {ty}")
                    };
                    let val = self.expression(bindings, val)?;
                    self.heap[addr] = val;
                }
                Value::Struct { fields }
            }

            Type::Int | Type::Real | Type::Bool | Type::Null | Type::Unit => {
                unreachable!("Unsupported type for `new` expression: {ty}")
            }
        }
    }

    #[throws(RuntimeError)]
    fn expression(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) -> Value<'a> {
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
                UnaryOperator::IntNeg => {
                    Value::Integer(-self.integer_expression(bindings, operand)?)
                }
                UnaryOperator::RealNeg => Value::Real(-self.real_expression(bindings, operand)?),
                UnaryOperator::BoolNeg => Value::Bool(!self.bool_expression(bindings, operand)?),
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

    fn print(&mut self, value: &Value<'_>) -> io::Result<()> {
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
            Value::Array { elements } => {
                write!(self.out, "[")?;
                for idx in elements {
                    let val = self.heap[*idx].clone();
                    self.print(&val)?;
                    write!(self.out, ", ")?;
                }
                write!(self.out, "]")
            }
            Value::Struct { fields } => {
                write!(self.out, "{{ ")?;

                let mut sorted_entries: Vec<_> = fields.iter().collect();
                sorted_entries.sort_by_key(|&(key, _value)| key);

                for (name, idx) in sorted_entries {
                    write!(self.out, "{name} : ")?;
                    let val = self.heap[*idx].clone();
                    self.print(&val)?;
                    write!(self.out, ", ")?;
                }

                writeln!(self.out, " }}")
            }
        }
    }

    #[throws(BlockError<'a>)]
    fn block(&mut self, bindings: &mut Bindings<'a>, block: &'a Block) {
        let Block { elems: stmts, .. } = block;
        for stmt in stmts {
            match stmt {
                BlockElem::Decl(LocalBinding { name, decl }) => match decl {
                    LocalDecl::Const(_) | LocalDecl::Type(_) => {}
                    LocalDecl::Var(VarDecl {
                        t,
                        initialiser,
                        relative_location: _,
                    }) => {
                        let e = match initialiser {
                            Some(e) => self.expression(bindings, e)?,

                            None => self.default_value_for_type(t),
                        };
                        let e = self.heap.alloc(e);
                        if let Some(prev) = bindings.insert(name, e) {
                            eprintln!(
                                "Discarding previous value for {name}: {:?} => {:?}",
                                self.heap[prev], self.heap[e]
                            )
                        }
                    }
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
                        self.print(&value).expect("IO Error")
                    }

                    ast::Statement::Return { value } => {
                        throw!(BlockError::Return(self.expression(bindings, value)?))
                    }

                    &ast::Statement::Panic { pos } => {
                        throw!(BlockError::Error(RuntimeError::Panic { pos }))
                    }
                },
            }
        }
    }

    #[throws(RuntimeError)]
    fn call<I: IdentSelector + ?Sized>(&mut self, routine: &I, args: Vec<Value<'a>>) -> Value<'a> {
        let Routine {
            signature: _,
            args_bindings,
            body,
        } = self.find_routine(routine);
        {
            let expected = args_bindings.len();
            let actual = args.len();
            assert_eq!(
                expected, actual,
                "Expected {expected} for {routine:?} args but got {actual}"
            )
        }
        let mut bindings = self.globals.clone();
        for (Binding { name, decl: _ }, value) in args_bindings.iter().zip(args) {
            let _: Option<Address<'a>> = bindings.insert(name, self.heap.alloc(value));
        }

        match body {
            RoutineBody::Expression(expression) => self.expression(&mut bindings, expression),
            RoutineBody::Block(block) => match self.block(&mut bindings, block) {
                Ok(()) => Ok(Value::Null),
                Err(BlockError::Return(v)) => Ok(v),
                Err(BlockError::Error(e)) => Err(e),
            },
        }?
    }

    #[throws(RuntimeError)]
    fn run(mut self) {
        for Binding { name, decl } in &self.program.0 {
            match decl {
                Decl::Type(_) | Decl::Routine(_) | Decl::Const(_) => {}
                Decl::Var(VarDecl {
                    t,
                    initialiser,
                    relative_location: _,
                }) => {
                    // FIXME: globals referencing other globals are not supported
                    let value = match initialiser.as_deref() {
                        None => self.default_value_for_type(t),
                        Some(expression) => {
                            self.expression(&mut Bindings::default(), expression)?
                        }
                    };
                    let value = self.heap.alloc(value);
                    if let Some(prev) = self.globals.insert(name, value) {
                        unreachable!(
                            "Redefinition of global {name:?}: {:?} => {:?}",
                            self.heap[prev], self.heap[value]
                        )
                    }
                }
            }
        }
        let value = self.call("main", vec![])?;
        let Value::Null = value else {
            unreachable!("main returned non-null value: {value:?}")
        };
    }
}

pub fn interpret(out: impl Write, program: &Program) -> Result<(), RuntimeError> {
    Interpreter {
        out,
        program,
        heap: Memory::default(),
        globals: Bindings::default(),
    }
    .run()
}
