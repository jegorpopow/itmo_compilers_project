use std::rc::Rc;

use derive_where::derive_where;

use common::{
    Identifier, Location, LoopOrder, RawIdentifier,
    operators::{
        BoolBinOp, EqBinOp, IntBinOp, RealBinOp, SemanticBinaryOperator, SemanticUnaryOperator,
    },
};

use crate::{
    AnalysisError, AnalysisResult,
    types::{ArrayDescription, Type},
};

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct IntegerLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: i64,
}

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct RealLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: f64,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum BoolLiteral {
    True,
    False,
}

impl BoolLiteral {
    #[must_use]
    pub fn to_bool(self) -> bool {
        match self {
            BoolLiteral::True => true,
            BoolLiteral::False => false,
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
    Binop {
        op: SemanticBinaryOperator,
        lhs: Rc<Expression>,
        rhs: Rc<Expression>,
    },
    Unop {
        op: SemanticUnaryOperator,
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
    LenghtOf {
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
    Int(i64),
    Real(f64),
    Bool(bool),
}

impl EvaluatedValue {
    fn as_int(&self) -> AnalysisResult<i64> {
        match self {
            EvaluatedValue::Int(val) => Ok(*val),
            EvaluatedValue::Real(_) | EvaluatedValue::Bool(_) => Err(AnalysisError {
                what: "fail".to_string(),
            }),
        }
    }

    fn as_real(&self) -> AnalysisResult<f64> {
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
    pub(crate) fn as_usize(&self) -> AnalysisResult<usize> {
        match self {
            &EvaluatedValue::Int(val) => val.try_into().map_err(|e| AnalysisError {
                what: format!("Complile-time non negative constant expected but found {val}: {e}"),
            }),
            EvaluatedValue::Real(_) | EvaluatedValue::Bool(_) => Err(AnalysisError {
                what: "Complile-time expression of type integer expected".to_owned(),
            }),
        }
    }
}

trait BinOp<T> {
    fn apply(&self, lhs: T, rhs: T) -> EvaluatedValue;
}

impl BinOp<f64> for RealBinOp {
    fn apply(&self, lhs: f64, rhs: f64) -> EvaluatedValue {
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

impl BinOp<i64> for IntBinOp {
    fn apply(&self, lhs: i64, rhs: i64) -> EvaluatedValue {
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
        match self {
            Expression::IntegerLiteral(integer_literal) => {
                Ok(EvaluatedValue::Int(integer_literal.value))
            }
            Expression::RealLiteral(real_literal) => Ok(EvaluatedValue::Real(real_literal.value)),
            Expression::BoolLiteral(bool_literal) => {
                Ok(EvaluatedValue::Bool(bool_literal.to_bool()))
            }
            Expression::Binop { op, lhs, rhs } => {
                let lhs = lhs.try_constexpr_evaluate()?;
                let rhs = rhs.try_constexpr_evaluate()?;
                Ok(match op {
                    SemanticBinaryOperator::Eq(op) => op.apply(lhs, rhs),
                    SemanticBinaryOperator::Real(op) => op.apply(lhs.as_real()?, rhs.as_real()?),
                    SemanticBinaryOperator::Int(op) => op.apply(lhs.as_int()?, rhs.as_int()?),
                    SemanticBinaryOperator::Bool(op) => op.apply(lhs.as_bool()?, rhs.as_bool()?),
                })
            }

            Expression::Unop { op, operand } => {
                let operand = operand.try_constexpr_evaluate()?;
                Ok(match op {
                    SemanticUnaryOperator::IntNeg => EvaluatedValue::Int(-operand.as_int()?),
                    SemanticUnaryOperator::RealNeg => EvaluatedValue::Real(-operand.as_real()?),
                    SemanticUnaryOperator::BoolNeg => EvaluatedValue::Bool(!operand.as_bool()?),
                })
            }

            Expression::IntToBool(expression) => Ok(EvaluatedValue::Bool(
                expression.try_constexpr_evaluate()?.as_int()? != 0,
            )),
            Expression::BoolToInt(expression) => Ok(EvaluatedValue::Int(
                expression.try_constexpr_evaluate()?.as_bool()?.into(),
            )),
            #[expect(clippy::cast_possible_truncation, reason = "By design")]
            Expression::RealToInt(expression) => Ok(EvaluatedValue::Int(
                expression.try_constexpr_evaluate()?.as_real()? as i64,
            )),
            #[expect(clippy::cast_precision_loss, reason = "By design")]
            Expression::IntToReal(expression) => Ok(EvaluatedValue::Real(
                expression.try_constexpr_evaluate()?.as_int()? as f64,
            )),

            Expression::Call { .. }
            | Expression::Cast { .. }
            | Expression::New { .. }
            | Expression::LenghtOf { .. }
            | Expression::Null
            | Expression::LvalueToRvalue(_) => Err(AnalysisError {
                what: format!(
                    "Non constexpr expression {self:?} in compile-time computation context"
                ),
            }),
        }
    }
}

#[derive(Debug, Hash, Clone)]
pub struct VarDecl {
    pub t: Rc<Type>,
    pub initialiser: Option<Rc<Expression>>,
    pub relative_location: Location,
}

pub(crate) trait OptionalDecl {
    fn is_full(&self) -> bool;
    fn is_forward(&self) -> bool {
        !self.is_full()
    }
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

impl OptionalDecl for TypeDecl {
    fn is_full(&self) -> bool {
        matches!(self, TypeDecl::Full { .. })
    }
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
pub enum RoutineDecl {
    Full {
        signature: RoutineSignature,
        args_bindings: Vec<Binding>,
        body: RoutineBody,
    },
    Forward {
        signature: RoutineSignature,
    },
}

impl OptionalDecl for RoutineDecl {
    fn is_full(&self) -> bool {
        matches!(self, RoutineDecl::Full { .. })
    }
}

impl RoutineDecl {
    #[must_use]
    pub fn signature(&self) -> &RoutineSignature {
        match self {
            RoutineDecl::Full { signature, .. } | RoutineDecl::Forward { signature } => signature,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RoutineBody {
    Block(Block),
    Expression(Rc<Expression>),
}

#[derive(Debug, Clone)]
pub enum SimpleDecl {
    Var(VarDecl),
    Type(TypeDecl),
}

impl From<SimpleDecl> for Decl {
    fn from(value: SimpleDecl) -> Self {
        match value {
            SimpleDecl::Var(var_decl) => Decl::Var(var_decl),
            SimpleDecl::Type(type_decl) => Decl::Type(type_decl),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimpleBinding {
    pub name: Identifier,
    pub decl: SimpleDecl,
}

impl From<SimpleBinding> for Binding {
    fn from(sb: SimpleBinding) -> Self {
        let SimpleBinding { name, decl } = sb;
        Binding {
            name,
            decl: decl.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlockElem {
    Stmt(Statement),
    Decl(SimpleBinding),
}

#[derive(Debug, Clone)]
pub struct Block(pub Vec<BlockElem>);

#[derive(Debug, Clone)]
pub enum Statement {
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
}

#[derive(Debug, Clone)]
pub enum Decl {
    Var(VarDecl),
    Type(TypeDecl),
    Routine(RoutineDecl),
}

impl TryFrom<Decl> for SimpleDecl {
    type Error = AnalysisError;

    fn try_from(value: Decl) -> AnalysisResult<SimpleDecl> {
        match value {
            Decl::Var(var_decl) => Ok(SimpleDecl::Var(var_decl)),
            Decl::Type(type_decl) => Ok(SimpleDecl::Type(type_decl)),
            Decl::Routine(_) => Err(AnalysisError {
                what: "nested functions are not supported".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub name: Identifier,
    pub decl: Decl,
}

impl Binding {
    pub fn ensure_is_type(&self) -> AnalysisResult<&TypeDecl> {
        match &self.decl {
            Decl::Type(t) => Ok(t),
            Decl::Var(_) | Decl::Routine(_) => Err(AnalysisError {
                what: format!("Name {:?} does not name a type", self.name),
            }),
        }
    }

    pub fn ensure_is_routine(&self) -> AnalysisResult<&RoutineDecl> {
        match &self.decl {
            Decl::Routine(t) => Ok(t),
            Decl::Var(_) | Decl::Type(_) => Err(AnalysisError {
                what: format!("Name {:?} does not name a routine", self.name),
            }),
        }
    }
}

impl TryFrom<Binding> for SimpleBinding {
    type Error = AnalysisError;

    fn try_from(value: Binding) -> AnalysisResult<SimpleBinding> {
        let Binding { name, decl } = value;
        Ok(SimpleBinding {
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
    expr: Rc<Expression>,
    own_type: &Type,
    target_type: &Type,
) -> AnalysisResult<Rc<Expression>> {
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
            Type::Bool => Ok(Rc::new(Expression::BoolToInt(Rc::new(
                Expression::IntToReal(expr),
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
            if *own_type == *target_type || *own_type == Type::Null {
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
