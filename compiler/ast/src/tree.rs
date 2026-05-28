use core::ops::Index;
use std::rc::Rc;

use common::{
    BindingId, Identifier, Integer, Location, LoopOrder, Position, RawIdentifier, Real, VarLoc,
    integer_to_real, real_to_integer,
};
pub use parser::Literal;

use crate::{
    AnalysisError, AnalysisResult, FieldDescription, RecordDescription, Typed,
    data_representation::{
        ArrayRepresentation, Interner, RecordRepresentation, Representation, TypeId,
    },
    operators::{BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp, UnaryOperator},
    types::{ArrayDescription, Type},
};

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum LvalueExpression {
    Identifier(Identifier),
    Member {
        lhs: Rc<LvalueExpression>,
        member_name: RawIdentifier,
        member_offset: u64, // This should be calculated from type of lhs and field name, but we do not store type in ast
    },
    Index {
        lhs: Rc<LvalueExpression>,
        index: Rc<Expression>,
    },
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Expression {
    LvalueToRvalue(Rc<LvalueExpression>),
    Literal(Literal),
    Call {
        callee: Identifier,
        args: Vec<Rc<Expression>>,
    },
    BinOp {
        op: BinaryOperator,
        lhs: Rc<Expression>,
        rhs: Rc<Expression>,
    },
    UnOp {
        op: UnaryOperator,
        operand: Rc<Expression>,
    },
    Cast {
        operand: Rc<Expression>,
        target: Rc<Type>,
    },
    New {
        t: Rc<Type>,
        fields: Option<Vec<(RawIdentifier, Rc<Expression>)>>,
    },
    NewArray {
        elements: Rc<Type>,
        length: Rc<Expression>,
    },
    LengthOf {
        arr: Rc<Expression>,
    },
    Null,
    IntToBool(Rc<Expression>),
    BoolToInt(Rc<Expression>),
    RealToInt(Rc<Expression>),
    IntToReal(Rc<Expression>),
}

impl From<Literal> for Typed<Expression> {
    fn from(literal: Literal) -> Self {
        Typed {
            ty: match literal {
                Literal::Bool { .. } => Type::bool(),
                Literal::Integer { .. } => Type::int(),
                Literal::Real { .. } => Type::real(),
            },
            value: Rc::new(Expression::Literal(literal)),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EvaluatedValue {
    Int(Integer),
    Real(Real),
    Bool(bool),
}

impl EvaluatedValue {
    fn as_int(&self) -> AnalysisResult<Integer> {
        match self {
            EvaluatedValue::Int(val) => Ok(*val),
            EvaluatedValue::Real(_) | EvaluatedValue::Bool(_) => Err(AnalysisError {
                what: "fail".to_string(),
            }),
        }
    }

    fn as_real(&self) -> AnalysisResult<Real> {
        match self {
            EvaluatedValue::Real(val) => Ok(*val),
            EvaluatedValue::Int(_) | EvaluatedValue::Bool(_) => Err(AnalysisError {
                what: "fail".to_string(),
            }),
        }
    }

    fn as_bool(&self) -> AnalysisResult<bool> {
        match self {
            EvaluatedValue::Bool(val) => Ok(*val),
            EvaluatedValue::Real(_) | EvaluatedValue::Int(_) => Err(AnalysisError {
                what: "fail".to_string(),
            }),
        }
    }
}

impl EvaluatedValue {
    pub(crate) fn as_literal(&self) -> Literal {
        match self {
            EvaluatedValue::Int(val) => Literal::Integer {
                repr: val.to_string(),
                value: *val,
            },
            EvaluatedValue::Real(val) => Literal::Real {
                repr: val.to_string(),
                value: *val,
            },
            EvaluatedValue::Bool(val) => Literal::Bool { value: *val },
        }
    }

    pub(crate) fn as_usize(&self) -> AnalysisResult<usize> {
        match self {
            &EvaluatedValue::Int(val) => val.try_into().map_err(|e| AnalysisError {
                what: format!("Compile-time non negative constant expected but found {val}: {e}"),
            }),
            EvaluatedValue::Real(_) | EvaluatedValue::Bool(_) => Err(AnalysisError {
                what: "Compile-time expression of type integer expected".to_owned(),
            }),
        }
    }
}

trait BinOp<T> {
    fn apply(&self, lhs: T, rhs: T) -> EvaluatedValue;
}

impl BinOp<Real> for RealBinOp {
    fn apply(&self, lhs: Real, rhs: Real) -> EvaluatedValue {
        match self {
            Self::Add => EvaluatedValue::Real(lhs + rhs),
            Self::Sub => EvaluatedValue::Real(lhs - rhs),
            Self::Mul => EvaluatedValue::Real(lhs * rhs),
            Self::Div => EvaluatedValue::Real(lhs / rhs),
            Self::Le => EvaluatedValue::Bool(lhs <= rhs),
            Self::Lt => EvaluatedValue::Bool(lhs < rhs),
            Self::Gt => EvaluatedValue::Bool(lhs > rhs),
            Self::Ge => EvaluatedValue::Bool(lhs >= rhs),
        }
    }
}

impl BinOp<Integer> for IntBinOp {
    fn apply(&self, lhs: Integer, rhs: Integer) -> EvaluatedValue {
        match self {
            Self::Add => EvaluatedValue::Int(lhs + rhs),
            Self::Sub => EvaluatedValue::Int(lhs - rhs),
            Self::Mul => EvaluatedValue::Int(lhs * rhs),
            Self::Div => EvaluatedValue::Int(lhs / rhs),
            Self::Mod => EvaluatedValue::Int(lhs % rhs),
            Self::Le => EvaluatedValue::Bool(lhs <= rhs),
            Self::Lt => EvaluatedValue::Bool(lhs < rhs),
            Self::Gt => EvaluatedValue::Bool(lhs > rhs),
            Self::Ge => EvaluatedValue::Bool(lhs >= rhs),
        }
    }
}

impl BinOp<bool> for BoolBinOp {
    fn apply(&self, lhs: bool, rhs: bool) -> EvaluatedValue {
        EvaluatedValue::Bool(match self {
            Self::And => lhs && rhs,
            Self::Or => lhs || rhs,
            Self::Xor => lhs ^ rhs,
        })
    }
}

impl BinOp<EvaluatedValue> for EqBinOp {
    fn apply(&self, lhs: EvaluatedValue, rhs: EvaluatedValue) -> EvaluatedValue {
        EvaluatedValue::Bool(match self {
            Self::Eq => lhs == rhs,
            Self::Ne => lhs != rhs,
        })
    }
}

impl Expression {
    pub fn ensure_is_lvalue(&self) -> AnalysisResult<Rc<LvalueExpression>> {
        match self {
            Expression::LvalueToRvalue(lvalue_expression) => Ok(Rc::clone(lvalue_expression)),
            Expression::Literal(_)
            | Expression::Call { .. }
            | Expression::BinOp { .. }
            | Expression::UnOp { .. }
            | Expression::Cast { .. }
            | Expression::New { .. }
            | Expression::NewArray { .. }
            | Expression::LengthOf { .. }
            | Expression::Null
            | Expression::IntToBool(_)
            | Expression::BoolToInt(_)
            | Expression::RealToInt(_)
            | Expression::IntToReal(_) => Err(AnalysisError {
                what: format!("Expression {self:?} expected to be lvalue"),
            }),
        }
    }

    pub(crate) fn try_constexpr_evaluate(&self) -> AnalysisResult<EvaluatedValue> {
        Ok(match self {
            Expression::Literal(lit) => match *lit {
                Literal::Bool { value } => EvaluatedValue::Bool(value),
                Literal::Integer { repr: _, value } => EvaluatedValue::Int(value),
                Literal::Real { repr: _, value } => EvaluatedValue::Real(value),
            },

            Expression::BinOp { op, lhs, rhs } => {
                let lhs = lhs.try_constexpr_evaluate()?;
                let rhs = rhs.try_constexpr_evaluate()?;
                match op {
                    BinaryOperator::Eq(op) => op.apply(lhs, rhs),
                    BinaryOperator::Real(op) => op.apply(lhs.as_real()?, rhs.as_real()?),
                    BinaryOperator::Int(op) => op.apply(lhs.as_int()?, rhs.as_int()?),
                    BinaryOperator::Bool(op) => op.apply(lhs.as_bool()?, rhs.as_bool()?),
                }
            }

            Expression::UnOp { op, operand } => {
                let operand = operand.try_constexpr_evaluate()?;
                match op {
                    UnaryOperator::IntNeg => EvaluatedValue::Int(-operand.as_int()?),
                    UnaryOperator::RealNeg => EvaluatedValue::Real(-operand.as_real()?),
                    UnaryOperator::BoolNeg => EvaluatedValue::Bool(!operand.as_bool()?),
                }
            }

            Expression::IntToBool(expression) => {
                EvaluatedValue::Bool(expression.try_constexpr_evaluate()?.as_int()? != 0)
            }
            Expression::BoolToInt(expression) => {
                EvaluatedValue::Int(expression.try_constexpr_evaluate()?.as_bool()?.into())
            }

            Expression::RealToInt(expression) => EvaluatedValue::Int(real_to_integer(
                expression.try_constexpr_evaluate()?.as_real()?,
            )),

            Expression::IntToReal(expression) => EvaluatedValue::Real(integer_to_real(
                expression.try_constexpr_evaluate()?.as_int()?,
            )),

            Expression::Call { .. }
            | Expression::Cast { .. }
            | Expression::New { .. }
            | Expression::NewArray { .. }
            | Expression::LengthOf { .. }
            | Expression::Null
            | Expression::LvalueToRvalue(_) => Err(AnalysisError {
                what: format!(
                    "Non constexpr expression {self:?} in compile-time computation context"
                ),
            })?,
        })
    }
}

#[derive(Debug, Hash, Clone)]
pub struct VarDecl {
    pub t: Rc<Type>,
    pub initialiser: Option<Rc<Expression>>,
    pub relative_location: Location,
}

#[derive(Debug, Clone, Copy)]
pub struct ConstDecl {
    pub value: EvaluatedValue,
}

#[derive(Debug, Hash, Clone)]
pub enum TypeDecl {
    Full {
        prescribed: Rc<Type>,
        effective: Rc<Type>,
    },
    Forward {
        alias: RawIdentifier,
    },
}

impl TypeDecl {
    #[must_use]
    pub fn get_effective(&self) -> &Rc<Type> {
        match self {
            TypeDecl::Full {
                prescribed: _,
                effective,
            } => effective,
            TypeDecl::Forward { alias } => {
                unreachable!("Trying to get a effective type of forward declared type {alias:?}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineSignature {
    pub args: Vec<(RawIdentifier, Rc<Type>)>,
    pub return_type: Rc<Type>,
}

#[derive(Debug, Clone)]
pub struct Routine {
    pub signature: RoutineSignature,
    pub args_bindings: Vec<Binding<VarDecl>>,
    pub body: RoutineBody,
}

#[derive(Debug, Clone)]
pub enum RoutineDecl {
    Full(Routine),
    Forward { signature: RoutineSignature },
}

impl RoutineDecl {
    #[must_use]
    pub fn signature(&self) -> &RoutineSignature {
        match self {
            RoutineDecl::Full(Routine { signature, .. }) | RoutineDecl::Forward { signature } => {
                signature
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RoutineBody {
    Block(Block),
    Expression(Rc<Expression>),
}

#[derive(Debug, Clone)]
pub enum LocalDecl {
    Var(VarDecl),
    Const(ConstDecl),
    Type(TypeDecl),
}

impl From<LocalDecl> for Decl {
    fn from(value: LocalDecl) -> Self {
        match value {
            LocalDecl::Var(var_decl) => Decl::Var(var_decl),
            LocalDecl::Type(type_decl) => Decl::Type(type_decl),
            LocalDecl::Const(const_decl) => Decl::Const(const_decl),
        }
    }
}

pub type LocalBinding = Binding<LocalDecl>;

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
    pub locals_count: VarLoc,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Declaration(LocalBinding),
    Assignment {
        lhs: Rc<LvalueExpression>,
        rhs: Rc<Expression>,
    },
    While {
        condition: Rc<Expression>,
        body: Block,
    },
    Expr(Rc<Expression>),
    If {
        condition: Rc<Expression>,
        on_true: Block,
        on_false: Option<Block>,
    },
    For {
        counter: Identifier,
        lower_bound: Rc<Expression>,
        upper_bound: Rc<Expression>,
        order: LoopOrder,
        body: Block,
    },
    ForEach {
        counter: Identifier,
        index: Identifier,
        collection: Rc<LvalueExpression>,
        order: LoopOrder,
        body: Block,
    },
    Print {
        value: Rc<Expression>,
        t: Rc<Type>,
    },
    Return {
        value: Rc<Expression>,
    },
    Panic {
        pos: Position,
    },
}

#[derive(Debug, Clone)]
pub enum Decl {
    Var(VarDecl),
    Const(ConstDecl),
    Type(TypeDecl),
    Routine(RoutineDecl),
}

#[derive(Debug, Clone)]
pub struct Binding<T = Decl> {
    pub name: Identifier,
    pub decl: T,
}

impl<T> Binding<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Binding<U> {
        let Self { name, decl } = self;
        Binding {
            name,
            decl: f(decl),
        }
    }
}

impl Binding {
    pub fn ensure_is_type(&self) -> AnalysisResult<&TypeDecl> {
        match &self.decl {
            Decl::Type(t) => Ok(t),
            Decl::Var(_) | Decl::Const(_) | Decl::Routine(_) => Err(AnalysisError {
                what: format!("Name {:?} does not name a type", self.name),
            }),
        }
    }

    pub fn ensure_is_var(&self) -> AnalysisResult<&VarDecl> {
        match &self.decl {
            Decl::Var(t) => Ok(t),
            Decl::Routine(_) | Decl::Type(_) | Decl::Const(_) => Err(AnalysisError {
                what: format!("Name {:?} does not name a variable", self.name),
            }),
        }
    }

    pub fn ensure_is_routine(&self) -> AnalysisResult<&RoutineDecl> {
        match &self.decl {
            Decl::Routine(t) => Ok(t),
            Decl::Var(_) | Decl::Type(_) | Decl::Const(_) => Err(AnalysisError {
                what: format!("Name {:?} does not name a routine", self.name),
            }),
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub globals: Vec<Binding>,
    pub bindings: Bindings,
}

#[derive(Debug, Default)]
pub struct Bindings {
    arena: Vec<Binding>,
}

impl Bindings {
    pub fn create(&mut self, name: &RawIdentifier, decl: Decl) -> Identifier {
        let identifier = Identifier {
            raw: name.clone(),
            id: BindingId(self.arena.len()),
        };
        self.arena.push(Binding {
            name: identifier.clone(),
            decl,
        });
        identifier
    }

    pub fn get_default_initialiser(&self, ty: &Rc<Type>) -> AnalysisResult<Expression> {
        Ok(match self.get_effective_type(ty)?.as_ref() {
            Type::Int => Expression::Literal(Literal::Integer {
                repr: "0".to_string(),
                value: 0,
            }),
            Type::Real => Expression::Literal(Literal::Real {
                repr: "0.0".to_string(),
                value: 0.0,
            }),
            Type::Bool => Expression::Literal(Literal::Bool { value: false }),
            Type::Alias(_) => Err(AnalysisError {
                what: "Effective type cannot be alias".to_string(),
            })?,
            Type::Record(_) | Type::Array(_) | Type::Null => Expression::Null,
        })
    }

    pub fn rebind(&mut self, ident: &Identifier, new_decl: Decl) {
        self.arena[ident.id.0].decl = new_decl;
    }

    pub fn get_effective_type<'a>(&'a self, t: &'a Rc<Type>) -> AnalysisResult<&'a Rc<Type>> {
        match t.as_ref() {
            Type::Alias(identifier) => self[identifier]
                .ensure_is_type()
                .map(TypeDecl::get_effective),
            Type::Int | Type::Real | Type::Bool | Type::Record(_) | Type::Array(_) | Type::Null => {
                Ok(t)
            }
        }
    }

    #[must_use]
    pub fn unwrap_effective_type<'a>(&'a self, t: &'a Rc<Type>) -> &'a Rc<Type> {
        self.get_effective_type(t)
            .expect("Internal compiler error: trying to get an effective type of a non-type alias")
    }

    pub fn get_type_representation<'a>(
        &'a self,
        t: &'a Rc<Type>,
        interner: &mut Interner,
    ) -> TypeId {
        let representation = match self.unwrap_effective_type(t).as_ref() {
            Type::Int => Representation::IntegerRepresentation,
            Type::Real => Representation::RealRepresentation,
            Type::Bool => Representation::BooleanRepresentation,
            Type::Null => Representation::NullRepresentation,
            Type::Alias(_) => {
                unreachable!("Effective type can not be alias")
            }
            record @ Type::Record(RecordDescription { fields }) => {
                let representation_fields: Vec<_> = fields
                    .iter()
                    .map(|FieldDescription { name, t }| {
                        debug_assert_ne!(
                            self.unwrap_effective_type(t).as_ref(),
                            record,
                            "Recursive types are not yet supported" // FIXME
                        );
                        (name.clone(), self.get_type_representation(t, interner))
                    })
                    .collect();
                Representation::RecordRepresentation(RecordRepresentation {
                    fields: representation_fields,
                })
            }
            array @ Type::Array(ArrayDescription { t, length: _ }) => {
                debug_assert_ne!(
                    self.unwrap_effective_type(t).as_ref(),
                    array,
                    "Recursive arrays are not supported (yet?)"
                );
                Representation::ArrayRepresentation(ArrayRepresentation {
                    element: self.get_type_representation(t, interner),
                })
            }
        };

        interner.intern(representation)
    }
}

impl Index<BindingId> for Bindings {
    type Output = Binding;

    fn index(&self, id: BindingId) -> &Self::Output {
        let BindingId(id) = id;
        &self.arena[id]
    }
}

impl Index<&Identifier> for Bindings {
    type Output = Binding;

    fn index(&self, ident: &Identifier) -> &Self::Output {
        &self[ident.id]
    }
}

impl Bindings {
    pub(crate) fn coerce(
        &self,
        expr: Typed<Expression>,
        target_type: &Type,
    ) -> AnalysisResult<Rc<Expression>> {
        let Typed {
            value: expr,
            ty: own_type,
        } = expr;

        let source_type = own_type.as_ref();

        match [source_type, target_type] {
            [Type::Int, Type::Int]
            | [Type::Bool, Type::Bool]
            | [Type::Real, Type::Real]
            | [Type::Null, Type::Null | Type::Record(_) | Type::Array(_)] => Ok(expr),

            [Type::Null, &Type::Alias(Identifier { raw: _, id })] => self.coerce(
                Typed {
                    value: expr,
                    ty: own_type,
                },
                self[id].ensure_is_type()?.get_effective(),
            ),

            [Type::Bool, Type::Real] => Ok(Rc::new(Expression::IntToReal(Rc::new(
                Expression::BoolToInt(expr),
            )))),
            [Type::Bool, Type::Int] => Ok(Rc::new(Expression::BoolToInt(expr))),
            [Type::Int, Type::Real] => Ok(Rc::new(Expression::IntToReal(expr))),

            [Type::Real, Type::Bool] => Ok(Rc::new(Expression::RealToInt(Rc::new(
                Expression::IntToBool(expr),
            )))),
            [Type::Real, Type::Int] => Ok(Rc::new(Expression::RealToInt(expr))),
            [Type::Int, Type::Bool] => Ok(Rc::new(Expression::IntToBool(expr))),

            [
                Type::Int
                | Type::Bool
                | Type::Real
                | Type::Array(_)
                | Type::Record(_)
                | Type::Alias(_),
                Type::Null,
            ] => Err(AnalysisError {
                what: format!("Cannot discard a value of type `{own_type}`"),
            }),

            [
                Type::Array(_) | Type::Record(_) | Type::Null,
                Type::Int | Type::Real | Type::Bool,
            ] => Err(AnalysisError {
                what: format!(
                    "Reference-counted type `{own_type}` cannot be converted to numeric type `{target_type}`"
                ),
            }),

            [
                Type::Int | Type::Real | Type::Bool,
                Type::Array(_) | Type::Record(_),
            ] => Err(AnalysisError {
                what: format!(
                    "Numeric type `{own_type}` cannot be converted to reference-counted type `{target_type}`"
                ),
            }),

            [Type::Alias(from), Type::Alias(to)] if from == to => Ok(expr),
            [Type::Record(r1), Type::Record(r2)] if r1 == r2 => Ok(expr),
            [
                Type::Array(ArrayDescription {
                    t: from_t,
                    length: from_length,
                }),
                Type::Array(ArrayDescription {
                    t: to_t,
                    length: to_length,
                }),
            ] if from_t == to_t && (from_length == to_length || to_length.is_none()) => Ok(expr),

            [
                Type::Array(_) | Type::Record(_),
                Type::Array(_) | Type::Record(_),
            ]
            | [Type::Alias(_), _]
            | [_, Type::Alias(_)] => Err(AnalysisError {
                what: format!(
                    "There is no implicit conversion from `{source_type}` to `{target_type}`"
                ),
            }),
        }
    }
}
