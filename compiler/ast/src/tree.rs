use core::ops::Index;
use std::rc::Rc;

use common::{
    BindingId, Identifier, Integer, Location, LoopOrder, Position, RawIdentifier, Real, VarLoc,
    integer_to_real, real_to_integer,
};
pub use parser::Literal;

use crate::{
    AnalysisError, AnalysisResult,
    data_representation::{
        ArrayRepresentation, Interner, RecordRepresentation, Representation, TypeId,
    },
    operators::{BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp, UnaryOperator},
    types::{ArrayDescription, Type},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ValueCategory {
    Lvalue,
    Rvalue,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ExpressionShape {
    Identifier(Identifier),
    Member {
        lhs: Rc<Expression>,
        member_name: RawIdentifier,
    },
    Index {
        lhs: Rc<Expression>,
        index: Rc<Expression>,
    },
    LvalueToRvalue(Rc<Expression>),
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
        fields: Vec<(RawIdentifier, Rc<Expression>)>,
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

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Expression {
    pub shape: ExpressionShape,
    pub ty: Rc<Type>,
    pub value_category: ValueCategory,
}

impl From<Literal> for Expression {
    fn from(literal: Literal) -> Self {
        Expression {
            ty: match literal {
                Literal::Bool { .. } => Type::bool(),
                Literal::Integer { .. } => Type::int(),
                Literal::Real { .. } => Type::real(),
            },
            shape: ExpressionShape::Literal(literal),
            value_category: ValueCategory::Rvalue,
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
    pub fn int_to_bool(expr: &Rc<Expression>) -> Rc<Expression> {
        assert_eq!(expr.ty, Type::int(), "Int -> Bool cast");
        assert_eq!(
            expr.value_category,
            ValueCategory::Rvalue,
            "Type coersion is for rvalue only"
        );
        Rc::new(Expression {
            shape: ExpressionShape::IntToBool(Rc::clone(expr)),
            ty: Type::bool(),
            value_category: ValueCategory::Rvalue,
        })
    }

    pub fn lvalue_to_rvalue(expr: &Rc<Expression>) -> Rc<Expression> {
        assert_eq!(
            expr.value_category,
            ValueCategory::Lvalue,
            "Lvalue -> Rvalue cast"
        );

        Rc::new(Expression {
            shape: ExpressionShape::LvalueToRvalue(Rc::clone(expr)),
            ty: Rc::clone(&expr.ty),
            value_category: ValueCategory::Rvalue,
        })
    }

    pub fn to_rvalue(expr: &Rc<Expression>) -> Rc<Expression> {
        match expr.value_category {
            ValueCategory::Lvalue => Rc::clone(expr),
            ValueCategory::Rvalue => Expression::lvalue_to_rvalue(&expr),
        }
    }

    pub fn int_to_real(expr: &Rc<Expression>) -> Rc<Expression> {
        assert_eq!(expr.ty, Type::int(), "Int -> Real cast");
        assert_eq!(
            expr.value_category,
            ValueCategory::Rvalue,
            "Type coersion is for rvalue only"
        );
        Rc::new(Expression {
            shape: ExpressionShape::IntToReal(Rc::clone(expr)),
            ty: Type::real(),
            value_category: ValueCategory::Rvalue,
        })
    }

    pub fn bool_to_int(expr: &Rc<Expression>) -> Rc<Expression> {
        assert_eq!(expr.ty, Type::bool(), "Int -> bool cast");
        assert_eq!(
            expr.value_category,
            ValueCategory::Rvalue,
            "Type coersion is for rvalue only"
        );
        Rc::new(Expression {
            shape: ExpressionShape::BoolToInt(Rc::clone(expr)),
            ty: Type::int(),
            value_category: ValueCategory::Rvalue,
        })
    }

    pub fn real_to_int(expr: &Rc<Expression>) -> Rc<Expression> {
        assert_eq!(expr.ty, Type::real(), "Int -> bool cast");
        assert_eq!(
            expr.value_category,
            ValueCategory::Rvalue,
            "Type coersion is for rvalue only"
        );
        Rc::new(Expression {
            shape: ExpressionShape::RealToInt(Rc::clone(expr)),
            ty: Type::int(),
            value_category: ValueCategory::Rvalue,
        })
    }

    pub(crate) fn try_constexpr_evaluate(&self) -> AnalysisResult<EvaluatedValue> {
        Ok(match &self.shape {
            ExpressionShape::Literal(lit) => match *lit {
                Literal::Bool { value } => EvaluatedValue::Bool(value),
                Literal::Integer { repr: _, value } => EvaluatedValue::Int(value),
                Literal::Real { repr: _, value } => EvaluatedValue::Real(value),
            },

            ExpressionShape::BinOp { op, lhs, rhs } => {
                let lhs = lhs.try_constexpr_evaluate()?;
                let rhs = rhs.try_constexpr_evaluate()?;
                match op {
                    BinaryOperator::Eq(op) => op.apply(lhs, rhs),
                    BinaryOperator::Real(op) => op.apply(lhs.as_real()?, rhs.as_real()?),
                    BinaryOperator::Int(op) => op.apply(lhs.as_int()?, rhs.as_int()?),
                    BinaryOperator::Bool(op) => op.apply(lhs.as_bool()?, rhs.as_bool()?),
                }
            }

            ExpressionShape::UnOp { op, operand } => {
                let operand = operand.try_constexpr_evaluate()?;
                match op {
                    UnaryOperator::IntNeg => EvaluatedValue::Int(-operand.as_int()?),
                    UnaryOperator::RealNeg => EvaluatedValue::Real(-operand.as_real()?),
                    UnaryOperator::BoolNeg => EvaluatedValue::Bool(!operand.as_bool()?),
                }
            }

            ExpressionShape::IntToBool(expression) => {
                EvaluatedValue::Bool(expression.try_constexpr_evaluate()?.as_int()? != 0)
            }
            ExpressionShape::BoolToInt(expression) => {
                EvaluatedValue::Int(expression.try_constexpr_evaluate()?.as_bool()?.into())
            }

            ExpressionShape::RealToInt(expression) => EvaluatedValue::Int(real_to_integer(
                expression.try_constexpr_evaluate()?.as_real()?,
            )),

            ExpressionShape::IntToReal(expression) => EvaluatedValue::Real(integer_to_real(
                expression.try_constexpr_evaluate()?.as_int()?,
            )),

            ExpressionShape::Call { .. }
            | ExpressionShape::Cast { .. }
            | ExpressionShape::New { .. }
            | ExpressionShape::NewArray { .. }
            | ExpressionShape::LengthOf { .. }
            | ExpressionShape::Null
            | ExpressionShape::Identifier(_)
            | ExpressionShape::Member { .. }
            | ExpressionShape::Index { .. }
            | ExpressionShape::LvalueToRvalue(_) => Err(AnalysisError {
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
        lhs: Rc<Expression>,
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
        collection: Rc<Expression>,
        order: LoopOrder,
        body: Block,
    },
    Print {
        value: Rc<Expression>,
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
            Type::Int => Expression {
                shape: ExpressionShape::Literal(Literal::Integer {
                    repr: "0".to_string(),
                    value: 0,
                }),
                ty: Type::int(),
                value_category: ValueCategory::Rvalue,
            },
            Type::Real => Expression {
                shape: ExpressionShape::Literal(Literal::Real {
                    repr: "0.0".to_string(),
                    value: 0.0,
                }),
                ty: Type::real(),
                value_category: ValueCategory::Rvalue,
            },
            Type::Bool => Expression {
                shape: ExpressionShape::Literal(Literal::Bool { value: false }),
                ty: Type::int(),
                value_category: ValueCategory::Rvalue,
            },
            Type::Alias(_) => Err(AnalysisError {
                what: "Effective type cannot be alias".to_string(),
            })?,
            Type::Record(_) | Type::Array(_) | Type::Null => Expression {
                shape: ExpressionShape::Null,
                ty: Rc::clone(ty),
                value_category: ValueCategory::Rvalue,
            },
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

    // TODO: handle recursive types
    pub fn get_type_representation<'a>(
        &'a self,
        t: &'a Rc<Type>,
        interner: &mut Interner,
    ) -> AnalysisResult<TypeId> {
        let type_id = match interner.register_type(&**t) {
            Ok(id) => return Ok(id),
            Err(id) => id,
        };

        let effective = self.get_effective_type(t)?;
        let representation = match &**effective {
            Type::Int => Representation::IntegerRepresentation,
            Type::Real => Representation::RealRepresentation,
            Type::Bool => Representation::BooleanRepresentation,
            Type::Null => Representation::NullRepresentation,
            Type::Alias(_) => {
                return Err(AnalysisError {
                    what: "Effective type can not be alias".to_string(),
                });
            }
            Type::Record(record_description) => {
                let representation_fields = record_description
                    .fields
                    .iter()
                    .map(|field| {
                        self.get_type_representation(&field.t, interner)
                            .map(|type_id| (field.name.clone(), type_id))
                    })
                    .collect::<AnalysisResult<Vec<(RawIdentifier, TypeId)>>>()?;
                Representation::RecordRepresentation(RecordRepresentation {
                    fields: representation_fields,
                })
            }
            Type::Array(array_description) => {
                Representation::ArrayRepresentation(ArrayRepresentation {
                    element: self.get_type_representation(&array_description.t, interner)?,
                })
            }
        };

        interner.intern_with_id(representation, type_id);
        Ok(type_id)
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
        expr: &Rc<Expression>,
        target_type: &Type,
    ) -> AnalysisResult<Rc<Expression>> {
        let own_type = Rc::clone(&expr.ty);
        let source_type = own_type.as_ref();

        match [source_type, target_type] {
            [Type::Int, Type::Int]
            | [Type::Bool, Type::Bool]
            | [Type::Real, Type::Real]
            | [Type::Null, Type::Null | Type::Record(_) | Type::Array(_)] => Ok(Rc::clone(expr)),

            [Type::Null, &Type::Alias(Identifier { raw: _, id })] => {
                self.coerce(expr, self[id].ensure_is_type()?.get_effective())
            }

            [Type::Bool, Type::Real] => Ok(Expression::int_to_real(&Expression::bool_to_int(expr))),
            [Type::Bool, Type::Int] => Ok(Expression::bool_to_int(expr)),
            [Type::Int, Type::Real] => Ok(Expression::int_to_real(expr)),
            [Type::Real, Type::Bool] => Ok(Expression::int_to_bool(&Expression::real_to_int(expr))),
            [Type::Real, Type::Int] => Ok(Expression::real_to_int(expr)),
            [Type::Int, Type::Bool] => Ok(Expression::int_to_bool(expr)),

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

            [Type::Alias(from), Type::Alias(to)] if from == to => Ok(Rc::clone(expr)),
            [Type::Record(r1), Type::Record(r2)] if r1 == r2 => Ok(Rc::clone(expr)),
            [
                Type::Array(ArrayDescription {
                    t: from_t,
                    length: from_length,
                }),
                Type::Array(ArrayDescription {
                    t: to_t,
                    length: to_length,
                }),
            ] if from_t == to_t && from_length == to_length => Ok(Rc::clone(expr)),
            [
                Type::Array(ArrayDescription {
                    t: from_t,
                    length: from_length,
                }),
                Type::Array(ArrayDescription {
                    t: to_t,
                    length: to_length,
                }),
            ] if from_t == to_t && to_length.is_none() => match expr.value_category {
                ValueCategory::Lvalue => Err(AnalysisError {
                    what: "Can not discard length for lvalue array".to_string(),
                }),
                ValueCategory::Rvalue => Ok(Rc::new(Expression {
                    shape: expr.shape.clone(),
                    ty: Rc::new(target_type.clone()),
                    value_category: ValueCategory::Rvalue,
                })),
            },
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
