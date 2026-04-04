use std::rc::Rc;

use derive_where::derive_where;

use common::{
    Identifier, Integer, Location, LoopOrder, Position, RawIdentifier, Real, VarLoc,
    integer_to_real, real_to_integer,
};

use crate::{
    AnalysisError, AnalysisResult, Typed,
    operators::{BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp, UnaryOperator},
    types::{ArrayDescription, Type},
};

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct IntegerLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: Integer,
}

impl From<&IntegerLiteral> for Integer {
    fn from(value: &IntegerLiteral) -> Self {
        value.value
    }
}

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct RealLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: Real,
}

impl From<&RealLiteral> for Real {
    fn from(value: &RealLiteral) -> Self {
        value.value
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum BoolLiteral {
    True,
    False,
}

impl From<bool> for BoolLiteral {
    fn from(value: bool) -> Self {
        #[expect(clippy::match_bool, reason = "prettier this way")]
        match value {
            true => Self::True,
            false => Self::False,
        }
    }
}

impl From<BoolLiteral> for bool {
    fn from(value: BoolLiteral) -> Self {
        match value {
            BoolLiteral::True => true,
            BoolLiteral::False => false,
        }
    }
}

impl From<BoolLiteral> for Integer {
    fn from(value: BoolLiteral) -> Self {
        match value {
            BoolLiteral::True => 1,
            BoolLiteral::False => 0,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum LvalueExpression {
    Identifier(Identifier),
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
    IntegerLiteral(IntegerLiteral),
    RealLiteral(RealLiteral),
    BoolLiteral(BoolLiteral),
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
    LengthOf {
        arr: Rc<Expression>,
    },
    Null,
    IntToBool(Rc<Expression>),
    BoolToInt(Rc<Expression>),
    RealToInt(Rc<Expression>),
    IntToReal(Rc<Expression>),
}

#[derive(Debug, PartialEq)]
pub(crate) enum EvaluatedValue {
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
    // FIXME(GrigorenkoPV): introduce Expression::Literal
    pub(crate) fn as_literal(&self) -> Rc<Expression> {
        match self {
            EvaluatedValue::Int(val) => Expression::IntegerLiteral(IntegerLiteral {
                repr: val.to_string(),
                value: *val,
            })
            .into(),
            EvaluatedValue::Real(val) => Expression::RealLiteral(RealLiteral {
                repr: val.to_string(),
                value: *val,
            })
            .into(),
            EvaluatedValue::Bool(val) => Expression::BoolLiteral(if *val {
                BoolLiteral::True
            } else {
                BoolLiteral::False
            })
            .into(),
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
            Expression::IntegerLiteral(lit) => EvaluatedValue::Int(lit.value),
            Expression::RealLiteral(lit) => EvaluatedValue::Real(lit.value),
            &Expression::BoolLiteral(lit) => EvaluatedValue::Bool(lit.into()),

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

/// FIXME(GrigorenkoPV): Typed<Literal>
#[derive(Debug, Hash, Clone)]
pub struct ConstDecl {
    pub t: Rc<Type>,
    pub value: Rc<Expression>,
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
    pub fn get_effective(&self) -> AnalysisResult<Rc<Type>> {
        match self {
            TypeDecl::Full {
                prescribed: _,
                effective,
            } => Ok(Rc::clone(effective)),
            TypeDecl::Forward { alias } => Err(AnalysisError {
                what: format!("Trying to get a effective type of forward declared type {alias:?}"),
            }),
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

impl From<LocalBinding> for Binding {
    fn from(sb: LocalBinding) -> Self {
        let LocalBinding { name, decl } = sb;
        Binding {
            name,
            decl: decl.into(),
        }
    }
}

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

impl TryFrom<Decl> for LocalDecl {
    type Error = AnalysisError;

    fn try_from(value: Decl) -> AnalysisResult<LocalDecl> {
        match value {
            Decl::Var(var_decl) => Ok(LocalDecl::Var(var_decl)),
            Decl::Type(type_decl) => Ok(LocalDecl::Type(type_decl)),
            Decl::Const(const_decl) => Ok(LocalDecl::Const(const_decl)),
            Decl::Routine(_) => Err(AnalysisError {
                what: "nested functions are not supported".to_string(),
            }),
        }
    }
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

impl TryFrom<Binding> for LocalBinding {
    type Error = AnalysisError;

    fn try_from(value: Binding) -> AnalysisResult<LocalBinding> {
        let Binding { name, decl } = value;
        Ok(LocalBinding {
            name,
            decl: decl.try_into()?,
        })
    }
}

#[derive(Debug)]
pub struct Program(pub Vec<Binding>);

#[derive(Debug, Default)]
pub struct IdentifierTable {
    bindings: Vec<Binding>,
}

impl IdentifierTable {
    pub fn create_binding(&mut self, name: &RawIdentifier, decl: Decl) -> Identifier {
        let id = self.bindings.len();
        let identifier = Identifier {
            raw: name.clone(),
            id,
        };
        self.bindings.push(Binding {
            name: identifier.clone(),
            decl,
        });
        identifier
    }

    pub fn get_default_initialiser(&self, ty: &Rc<Type>) -> AnalysisResult<Rc<Expression>> {
        match &*self.get_effective_type(ty)? {
            Type::Int => Ok(Expression::IntegerLiteral(IntegerLiteral {
                repr: "0".to_string(),
                value: 0,
            })
            .into()),
            Type::Real => Ok(Expression::RealLiteral(RealLiteral {
                repr: "0.0".to_string(),
                value: 0.0,
            })
            .into()),
            Type::Bool => Ok(Expression::BoolLiteral(BoolLiteral::False).into()),
            Type::Alias(_) => Err(AnalysisError {
                what: "Effective type cannot be alias".to_string(),
            }),
            Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => {
                Ok(Expression::Null.into())
            }
        }
    }

    pub fn rebind(&mut self, ident: &Identifier, new_decl: Decl) {
        self.bindings[ident.id].decl = new_decl;
    }

    #[must_use]
    pub fn get_binding(&self, ident: &Identifier) -> &Binding {
        &self.bindings[ident.id]
    }

    #[must_use]
    pub fn get_binding_by_id(&self, id: usize) -> &Binding {
        &self.bindings[id]
    }

    pub fn get_effective_type(&self, t: &Rc<Type>) -> AnalysisResult<Rc<Type>> {
        match &**t {
            Type::Alias(identifier) => self
                .get_binding(identifier)
                .ensure_is_type()
                .and_then(TypeDecl::get_effective),
            Type::Int
            | Type::Real
            | Type::Bool
            | Type::Record(_)
            | Type::Array(_)
            | Type::Null
            | Type::Unit => Ok(Rc::clone(t)),
        }
    }
}

// FIXME: add target effective type
pub(crate) fn cast_to(
    expr: Typed<Expression>,
    target_type: &Type,
) -> AnalysisResult<Rc<Expression>> {
    let Typed {
        value: expr,
        ty: own_type,
    } = expr;
    let own_type = &*own_type;
    match target_type {
        Type::Int => match own_type {
            Type::Int => Ok(expr),
            Type::Real => Ok(Rc::new(Expression::RealToInt(expr))),
            Type::Bool => Ok(Rc::new(Expression::BoolToInt(expr))),
            Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => {
                Err(AnalysisError {
                    what: format!(
                        "There is no implicit conversion from `{own_type}` to `{target_type}`"
                    ),
                })
            }
        },
        Type::Real => match own_type {
            Type::Real => Ok(expr),
            Type::Int => Ok(Rc::new(Expression::IntToReal(expr))),
            Type::Bool => Ok(Rc::new(Expression::IntToReal(Rc::new(
                Expression::BoolToInt(expr),
            )))),
            Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => {
                Err(AnalysisError {
                    what: format!(
                        "There is no implicit conversion from `{own_type}` to `{target_type}`"
                    ),
                })
            }
        },

        Type::Bool => match own_type {
            Type::Bool => Ok(expr),
            Type::Int => Ok(Rc::new(Expression::IntToBool(expr))),
            Type::Real => Ok(Rc::new(Expression::IntToBool(Rc::new(
                Expression::RealToInt(expr),
            )))),
            Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => {
                Err(AnalysisError {
                    what: format!(
                        "There is no implicit conversion from `{own_type}` to `{target_type}`"
                    ),
                })
            }
        },

        Type::Array(ArrayDescription { t, length: None }) => {
            let element_type = own_type.get_element_type()?;
            if element_type == t {
                Ok(expr)
            } else {
                Err(AnalysisError {
                    what: format!("Array of {element_type} is casted to array of {t} type"),
                })
            }
        }

        Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => {
            if own_type == target_type || *own_type == Type::Null {
                Ok(expr)
            } else {
                Err(AnalysisError {
                    what: format!(
                        "There is no implicit conversion from `{own_type}` to `{target_type}`"
                    ),
                })
            }
        }
    }
}
