use core::{fmt, iter};
use std::{collections::HashMap, fmt::Debug, io::Write, rc::Rc};

use ast::{
    ArrayDescription, BinaryOperator, Binding, Block, BoolBinOp, Decl, EqBinOp, Expression,
    FieldDescription, IntBinOp, Literal, LocalBinding, LocalDecl, LvalueExpression, Program,
    RealBinOp, RecordDescription, Routine, RoutineBody, RoutineDecl, Type, TypeDecl, UnaryOperator,
    VarDecl,
};
use common::{
    Identifier, Integer, LoopOrder, Position, RawIdentifier, Real, integer_to_real, real_to_integer,
};
use culpa::{throw, throws};
use indexed_arena::{Arena, Idx};

mod print;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Primitive {
    Null,
    Bool(bool),
    Integer(Integer),
    Real(Real),
}

#[derive(Debug, Clone)]
enum Object<'a> {
    Array {
        elements: Vec<Place<'a>>,
    },
    Struct {
        fields: HashMap<&'a RawIdentifier, Place<'a>>,
    },
}
#[derive(Debug, Clone, PartialEq)]
enum Value<'a> {
    Primitive(Primitive),
    Reference(Reference<'a>),
}

impl<'a> Value<'a> {
    #[track_caller]
    fn unwrap_primitive(&self) -> Primitive {
        match *self {
            Self::Primitive(p) => p,
            Self::Reference(r) => {
                unreachable!("Expected a primitive, but found a reference: {r:?}")
            }
        }
    }

    #[track_caller]
    fn unwrap_reference(&self) -> Reference<'a> {
        match *self {
            Self::Primitive(p) => unreachable!("Expected a reference, but found {p:?}"),
            Self::Reference(r) => r,
        }
    }
}

type HeapAddress = usize;
type Reference<'a> = Idx<Object<'a>, HeapAddress>;
type Heap<'a> = Arena<Object<'a>, HeapAddress>;

type PlaceIndex = usize;
type Place<'a> = Idx<Value<'a>, PlaceIndex>;
type Places<'a> = Arena<Value<'a>, PlaceIndex>;

type Bindings<'a> = HashMap<&'a Identifier, Place<'a>>;

#[derive(Debug)]
struct Interpreter<'a, W: Write> {
    out: W,
    program: &'a Program,
    heap: Heap<'a>,
    places: Places<'a>,
    globals: Bindings<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeError {
    IndexOutOfBounds { index: Integer, len: usize },
    Panic { pos: Position },
    InvalidArrayLength { len: Integer },
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
            Self::InvalidArrayLength { len } => write!(f, "Cannot create array of length {len}"),
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

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "essentially an if-let but with better error reporting"
)]
impl<'a, W: Write> Interpreter<'a, W> {
    #[throws(RuntimeError)]
    #[track_caller]
    fn bool_expression(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) -> bool {
        match self.expression(bindings, expression)?.unwrap_primitive() {
            Primitive::Bool(value) => value,
            value => unreachable!("Expected bool, got {value:?}"),
        }
    }

    #[throws(RuntimeError)]
    #[track_caller]
    fn integer_expression(
        &mut self,
        bindings: &mut Bindings<'a>,
        expression: &'a Expression,
    ) -> Integer {
        match self.expression(bindings, expression)?.unwrap_primitive() {
            Primitive::Integer(value) => value,
            value => unreachable!("Expected integer, got {value:?}"),
        }
    }

    #[throws(RuntimeError)]
    #[track_caller]
    fn real_expression(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) -> Real {
        match self.expression(bindings, expression)?.unwrap_primitive() {
            Primitive::Real(value) => value,
            value => unreachable!("Expected real, got {value:?}"),
        }
    }

    #[throws(RuntimeError)]
    #[track_caller]
    fn array_expression<'this>(
        &'this mut self,
        bindings: &mut Bindings<'a>,
        expression: &'a Expression,
    ) -> &'this [Place<'a>] {
        let reference = self.expression(bindings, expression)?.unwrap_reference();
        match &self.heap[reference] {
            Object::Array { elements } => elements,
            e @ Object::Struct { .. } => unreachable!("Expected array, got {e:?}"),
        }
    }
}

impl<'a, W: Write> Interpreter<'a, W> {
    fn default_value_for_type(&self, ty: &Type) -> Primitive {
        match ty {
            Type::Int => Primitive::Integer(0),
            Type::Real => Primitive::Real(0.0),
            Type::Bool => Primitive::Bool(false),
            Type::Alias(ty) => self.default_value_for_type(self.get_effective_type(ty)),
            Type::Record(_) | Type::Array(_) | Type::Null => Primitive::Null,
        }
    }

    fn get_effective_type(&self, ty: &Identifier) -> &'a Type {
        let decl = &self.program.bindings[ty.id].decl;
        let Decl::Type(TypeDecl::Full {
            prescribed: _,
            effective,
        }) = decl
        else {
            unreachable!("Expected a type for {ty:?} but found {decl:?}")
        };
        if let Type::Alias(to) = effective.as_ref() {
            unreachable!("Effective type for {ty:?} is an alias to {to:?}")
        }
        effective
    }

    #[throws(RuntimeError)]
    fn lvalue(&mut self, bindings: &mut Bindings<'a>, lvalue: &'a LvalueExpression) -> Place<'a> {
        match lvalue {
            LvalueExpression::Identifier(ident) => {
                let Some(&var) = bindings.get(ident) else {
                    unreachable!("Could not find variable {ident:?}")
                };

                var
            }
            LvalueExpression::Member { lhs, member_name } => {
                let lhs = self.lvalue(bindings, lhs)?;
                let lhs = self.places[lhs].unwrap_reference();
                match &self.heap[lhs] {
                    Object::Struct { fields } => {
                        let Some(&field) = fields.get(&member_name) else {
                            unreachable!("No field {member_name:?}")
                        };
                        field
                    }

                    e @ Object::Array { .. } => unreachable!("{e:?} is not a struct"),
                }
            }
            LvalueExpression::Index { lhs, index } => {
                let lhs = self.lvalue(bindings, lhs)?;
                let lhs = self.places[lhs].unwrap_reference();
                let index = self.integer_expression(&mut *bindings, index)?;
                match &self.heap[lhs] {
                    Object::Array { elements } => {
                        let len = elements.len();
                        *index
                            .saturating_sub(1)
                            .try_into()
                            .ok()
                            .and_then(|i: usize| elements.get(i))
                            .ok_or(RuntimeError::IndexOutOfBounds { index, len })?
                    }
                    e @ Object::Struct { .. } => unreachable!("{e:?} is not an array"),
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
    ) -> Primitive {
        match op {
            BinaryOperator::Eq(op) => {
                let lhs = self.expression(bindings, lhs)?;
                let rhs = self.expression(bindings, rhs)?;
                Primitive::Bool(match op {
                    EqBinOp::Eq => lhs == rhs,
                    EqBinOp::Ne => lhs != rhs,
                })
            }
            BinaryOperator::Real(op) => {
                let lhs = self.real_expression(bindings, lhs)?;
                let rhs = self.real_expression(bindings, rhs)?;
                match op {
                    RealBinOp::Add => Primitive::Real(lhs + rhs),
                    RealBinOp::Sub => Primitive::Real(lhs - rhs),
                    RealBinOp::Mul => Primitive::Real(lhs * rhs),
                    RealBinOp::Div => Primitive::Real(lhs / rhs),
                    RealBinOp::Le => Primitive::Bool(lhs <= rhs),
                    RealBinOp::Lt => Primitive::Bool(lhs < rhs),
                    RealBinOp::Gt => Primitive::Bool(lhs > rhs),
                    RealBinOp::Ge => Primitive::Bool(lhs >= rhs),
                }
            }
            BinaryOperator::Int(op) => {
                let lhs = self.integer_expression(bindings, lhs)?;
                let rhs = self.integer_expression(bindings, rhs)?;
                match op {
                    IntBinOp::Add => Primitive::Integer(lhs + rhs),
                    IntBinOp::Sub => Primitive::Integer(lhs - rhs),
                    IntBinOp::Mul => Primitive::Integer(lhs * rhs),
                    IntBinOp::Div => Primitive::Integer(lhs / rhs),
                    IntBinOp::Mod => Primitive::Integer(lhs % rhs),
                    IntBinOp::Le => Primitive::Bool(lhs <= rhs),
                    IntBinOp::Lt => Primitive::Bool(lhs < rhs),
                    IntBinOp::Gt => Primitive::Bool(lhs > rhs),
                    IntBinOp::Ge => Primitive::Bool(lhs >= rhs),
                }
            }
            BinaryOperator::Bool(op) => Primitive::Bool(match op {
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
    ) -> Object<'a> {
        match ty {
            Type::Array(ArrayDescription { t, length }) => Object::Array {
                elements: match (*length, field_values) {
                    (Some(n), None) => {
                        let value = self.default_value_for_type(t);
                        iter::repeat_n(value, n)
                            .map(|v| self.places.alloc(Value::Primitive(v)))
                            .collect()
                    }
                    _ => todo!(),
                },
            },

            Type::Record(RecordDescription {
                fields: expected_fields,
            }) => {
                let fields: HashMap<&'a RawIdentifier, Place<'a>> = expected_fields
                    .iter()
                    .map(|FieldDescription { name, t }| {
                        let val = self.default_value_for_type(t);
                        (name, self.places.alloc(Value::Primitive(val)))
                    })
                    .collect();
                for (name, val) in field_values.unwrap_or_default() {
                    let Some(&id) = fields.get(name) else {
                        unreachable!("No field {name:?} in {ty}")
                    };
                    let val = self.expression(bindings, val)?;
                    self.places[id] = val;
                }
                Object::Struct { fields }
            }

            Type::Alias(ty) => self.new(bindings, self.get_effective_type(ty), field_values)?,

            Type::Int | Type::Real | Type::Bool | Type::Null => {
                unreachable!("Unsupported type for `new` expression: {ty}")
            }
        }
    }

    #[throws(RuntimeError)]
    fn new_array(
        &mut self,
        bindings: &mut Bindings<'a>,
        element_ty: &'a Type,
        len: &'a Expression,
    ) -> Object<'a> {
        let len = self.integer_expression(bindings, len)?;
        Object::Array {
            elements: iter::repeat_n(
                self.default_value_for_type(element_ty),
                len.try_into()
                    .map_err(|_e| RuntimeError::InvalidArrayLength { len })?,
            )
            .map(|v| self.places.alloc(Value::Primitive(v)))
            .collect(),
        }
    }

    #[throws(RuntimeError)]
    fn expression(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) -> Value<'a> {
        match expression {
            Expression::Null => Value::Primitive(Primitive::Null),
            Expression::Literal(literal) => Value::Primitive(match *literal {
                Literal::Bool { value } => Primitive::Bool(value),
                Literal::Integer { repr: _, value } => Primitive::Integer(value),
                Literal::Real { repr: _, value } => Primitive::Real(value),
            }),

            Expression::LvalueToRvalue(lvalue) => {
                let addr = self.lvalue(bindings, lvalue)?;
                self.places[addr].clone()
            }

            Expression::Call { callee, args } => {
                let args: Vec<_> = args
                    .iter()
                    .map(|arg| self.expression(bindings, arg))
                    .collect::<Result<_, _>>()?;
                self.call(callee, args)?
            }
            Expression::BinOp { op, lhs, rhs } => {
                Value::Primitive(self.binop(bindings, *op, lhs, rhs)?)
            }
            Expression::UnOp { op, operand } => Value::Primitive(match op {
                UnaryOperator::IntNeg => {
                    Primitive::Integer(-self.integer_expression(bindings, operand)?)
                }
                UnaryOperator::RealNeg => {
                    Primitive::Real(-self.real_expression(bindings, operand)?)
                }
                UnaryOperator::BoolNeg => {
                    Primitive::Bool(!self.bool_expression(bindings, operand)?)
                }
            }),
            Expression::Cast { operand, target: _ } => self.expression(bindings, operand)?,
            Expression::New { t, fields } => {
                let obj = self.new(bindings, t, fields.as_deref())?;
                Value::Reference(self.heap.alloc(obj))
            }
            Expression::NewArray { elements, length } => {
                let obj = self.new_array(bindings, elements, length)?;
                Value::Reference(self.heap.alloc(obj))
            }

            Expression::LengthOf { arr } => Value::Primitive(Primitive::Integer(
                self.array_expression(bindings, arr)?
                    .len()
                    .try_into()
                    .expect("Array too long"),
            )),

            Expression::IntToBool(expression) => Value::Primitive(Primitive::Bool(
                self.integer_expression(bindings, expression)? != 0,
            )),
            Expression::BoolToInt(expression) => Value::Primitive(Primitive::Integer(
                self.bool_expression(bindings, expression)?.into(),
            )),
            Expression::RealToInt(expression) => Value::Primitive(Primitive::Integer(
                real_to_integer(self.real_expression(bindings, expression)?),
            )),
            Expression::IntToReal(expression) => Value::Primitive(Primitive::Real(
                integer_to_real(self.integer_expression(bindings, expression)?),
            )),
        }
    }

    #[throws(RuntimeError)]
    fn print(&mut self, bindings: &mut Bindings<'a>, expression: &'a Expression) {
        let value = self.expression(bindings, expression)?;
        print::print(&mut self.out, &self.heap, &self.places, &value).expect("IO Error")
    }

    #[throws(BlockError<'a>)]
    fn block(&mut self, bindings: &mut Bindings<'a>, block: &'a Block) {
        let Block { stmts, .. } = block;
        for stmt in stmts {
            match stmt {
                ast::Statement::Declaration(LocalBinding { name, decl }) => match decl {
                    LocalDecl::Const(_) | LocalDecl::Type(_) => {}
                    LocalDecl::Var(VarDecl {
                        t,
                        initialiser,
                        relative_location: _,
                    }) => {
                        let e = match initialiser {
                            Some(e) => self.expression(bindings, e)?,
                            None => Value::Primitive(self.default_value_for_type(t)),
                        };
                        let e = self.places.alloc(e);
                        if let Some(prev) = bindings.insert(name, e)
                            && cfg!(debug_assertions)
                        {
                            eprintln!(
                                "Discarding previous value for {name}: {:?} => {:?}",
                                self.places[prev], self.places[e]
                            )
                        }
                    }
                },

                ast::Statement::Assignment { lhs, rhs } => {
                    let lhs = self.lvalue(bindings, lhs)?;
                    let rhs = self.expression(bindings, rhs)?;
                    self.places[lhs] = rhs;
                }

                ast::Statement::While { condition, body } => {
                    while self.bool_expression(bindings, condition)? {
                        self.block(bindings, body)?
                    }
                }

                ast::Statement::Expr(expression) => {
                    let _: Value<'a> = self.expression(bindings, expression)?;
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
                        let _: Option<Place<'a>> = bindings.insert(
                            counter,
                            self.places
                                .alloc(Value::Primitive(Primitive::Integer(value))),
                        );
                        self.block(bindings, body)?
                    }
                }

                ast::Statement::ForEach {
                    counter,
                    collection,
                    order,
                    body,
                } => {
                    // assigning to the loop variable won't change the array, so we need to create new places
                    let mut collection: Vec<_> = self
                        .array_expression(bindings, collection)?
                        .to_vec()
                        .into_iter()
                        .map(|lvalue| self.places[lvalue].clone())
                        .collect();
                    match order {
                        LoopOrder::Direct => {}
                        LoopOrder::Reversed => collection.reverse(),
                    }
                    for value in collection {
                        let _: Option<Place<'a>> =
                            bindings.insert(counter, self.places.alloc(value));
                        self.block(bindings, body)?
                    }
                }

                ast::Statement::Print { value } => self.print(bindings, value)?,

                ast::Statement::Return { value } => {
                    throw!(BlockError::Return(self.expression(bindings, value)?))
                }

                &ast::Statement::Panic { pos } => {
                    throw!(BlockError::Error(RuntimeError::Panic { pos }))
                }
            }
        }
    }

    #[throws(RuntimeError)]
    fn call(&mut self, routine: &Identifier, args: Vec<Value<'a>>) -> Value<'a> {
        let decl = &self.program.bindings[routine.id].decl;
        let Decl::Routine(RoutineDecl::Full(Routine {
            signature: _,
            args_bindings,
            body,
        })) = decl
        else {
            unreachable!("Expected a routine for {routine:?} but found {decl:?}")
        };
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
            let _: Option<Place<'a>> = bindings.insert(name, self.places.alloc(value));
        }

        match body {
            RoutineBody::Expression(expression) => self.expression(&mut bindings, expression),
            RoutineBody::Block(block) => match self.block(&mut bindings, block) {
                Ok(()) => Ok(Value::Primitive(Primitive::Null)),
                Err(BlockError::Return(v)) => Ok(v),
                Err(BlockError::Error(e)) => Err(e),
            },
        }?
    }

    #[throws(RuntimeError)]
    fn run(mut self) {
        for Binding { name, decl } in &self.program.globals {
            match decl {
                Decl::Type(_) | Decl::Routine(_) | Decl::Const(_) => {}
                Decl::Var(VarDecl {
                    t,
                    initialiser,
                    relative_location: _,
                }) => {
                    // FIXME: globals referencing other globals are not supported
                    let value = match initialiser.as_deref() {
                        Some(expression) => {
                            self.expression(&mut Bindings::default(), expression)?
                        }
                        None => Value::Primitive(self.default_value_for_type(t)),
                    };
                    let place = self.places.alloc(value);
                    if let Some(prev) = self.globals.insert(name, place) {
                        unreachable!(
                            "Redefinition of global {name:?}: {:?} => {:?}",
                            self.places[prev], self.places[place]
                        )
                    }
                }
            }
        }
        let Some(main) = self
            .program
            .globals
            .iter()
            .find_map(|Binding { name, decl }| {
                if name.raw.name == "main"
                    && let Decl::Routine(RoutineDecl::Full(_)) = decl
                {
                    Some(name)
                } else {
                    None
                }
            })
        else {
            todo!("No `main")
        };
        let value = self.call(main, vec![])?;
        let Value::Primitive(Primitive::Null) = value else {
            unreachable!("main returned non-null value: {value:?}")
        };
    }
}

pub fn interpret(out: impl Write, program: &Program) -> Result<(), RuntimeError> {
    Interpreter {
        out,
        program,
        heap: Heap::default(),
        places: Places::default(),
        globals: Bindings::default(),
    }
    .run()
}
