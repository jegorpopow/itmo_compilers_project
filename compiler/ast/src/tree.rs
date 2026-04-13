use std::rc::Rc;

use common::{
    Integer, Location, LoopOrder, Position, RawIdentifier, Real, VarLoc, integer_to_real,
    real_to_integer,
};
pub use parser::Literal;

use crate::{
    AnalysisError, AnalysisResult, BindingId, Bindings, Typed,
    operators::{BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp, UnaryOperator},
    types::Type,
};

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum LvalueExpression {
    Binding(BindingId),
    Member {
        lhs: Rc<LvalueExpression>,
        member_name: RawIdentifier,
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
        callee: BindingId,
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
    pub initialiser: Rc<Expression>,
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
    pub args_bindings: Vec<BindingId>,
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
pub struct Block {
    pub stmts: Vec<Statement>,
    pub locals_count: VarLoc,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Initialization {
        lhs: BindingId,
        rhs: Rc<Expression>,
    },
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
        counter: BindingId,
        lower_bound: Rc<Expression>,
        upper_bound: Rc<Expression>,
        order: LoopOrder,
        body: Block,
    },
    ForEach {
        counter: BindingId,
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

impl Decl {
    #[must_use]
    pub fn unwrap_type(&self) -> &TypeDecl {
        self.ensure_is_type().expect("unwrap type on non type")
    }

    pub fn ensure_is_type(&self,) -> AnalysisResult<&TypeDecl> {
        match &self {
            Decl::Type(t) => Ok(t),
            Decl::Var(_) | Decl::Const(_) | Decl::Routine(_) => Err(AnalysisError {
                what: format!("Name {self:?} does not name a type",),
            }),
        }
    }

    pub fn ensure_is_var(&self) -> AnalysisResult<&VarDecl> {
        match &self {
            Decl::Var(t) => Ok(t),
            Decl::Routine(_) | Decl::Type(_) | Decl::Const(_) => Err(AnalysisError {
                what: format!("Name {self:?} does not name a variable"),
            }),
        }
    }

    pub fn ensure_is_routine(&self, name: &RawIdentifier) -> AnalysisResult<&RoutineDecl> {
        match &self {
            Decl::Routine(t) => Ok(t),
            Decl::Var(_) | Decl::Type(_) | Decl::Const(_) => Err(AnalysisError {
                what: format!("Name {name} does not name a routine",),
            }),
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub globals: Vec<BindingId>,
    pub bindings: Bindings,
}
