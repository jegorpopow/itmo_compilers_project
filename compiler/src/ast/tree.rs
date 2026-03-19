#![allow(dead_code, reason = "WIP")]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::float_cmp)]
#![allow(clippy::cast_possible_truncation)]
use std::rc::Rc;

use crate::ast::error::{AnalysisError, AnalysisResult};
use crate::ast::types::{ArrayDescription, Type};
use crate::bytecode::Location;
use crate::identifier::{Identifier, RawIdentifier};
use crate::loop_order::LoopOrder;

use crate::operators::{SemanticBinaryOperator, SemanticUnaryOperator};
use derive_where::derive_where;

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
pub enum EvaluatedValue {
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
    pub fn as_usize(&self) -> AnalysisResult<usize> {
        match self {
            EvaluatedValue::Int(val) => {
                if *val >= 0 {
                    Ok(usize::try_from(*val).unwrap())
                } else {
                    Err(AnalysisError {
                        what: format!("Complile-time non negative constant expected {val} found"),
                    })
                }
            }
            EvaluatedValue::Real(_) | EvaluatedValue::Bool(_) => Err(AnalysisError {
                what: "Complile-time expression of type integer expected".to_owned(),
            }),
        }
    }
}

impl Expression {
    pub fn try_constexpr_evaluate(&self) -> AnalysisResult<EvaluatedValue> {
        match self {
            Expression::IntegerLiteral(integer_literal) => {
                Ok(EvaluatedValue::Int(integer_literal.value))
            }
            Expression::RealLiteral(real_literal) => Ok(EvaluatedValue::Real(real_literal.value)),
            Expression::BoolLiteral(bool_literal) => {
                Ok(EvaluatedValue::Bool(bool_literal.to_bool()))
            }
            Expression::Binop { op, lhs, rhs } => match op {
                SemanticBinaryOperator::RealAdd => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Real(lhs + rhs))
                }
                SemanticBinaryOperator::RealSub => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Real(lhs - rhs))
                }
                SemanticBinaryOperator::RealMul => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Real(lhs * rhs))
                }
                SemanticBinaryOperator::RealDiv => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Real(lhs / rhs))
                }
                SemanticBinaryOperator::RealLe => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Bool(lhs <= rhs))
                }
                SemanticBinaryOperator::RealLt => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Bool(lhs < rhs))
                }
                SemanticBinaryOperator::RealGt => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Bool(lhs > rhs))
                }
                SemanticBinaryOperator::RealGe => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_real()?;
                    Ok(EvaluatedValue::Bool(lhs >= rhs))
                }
                SemanticBinaryOperator::Eq => Ok(EvaluatedValue::Bool(lhs == rhs)),
                SemanticBinaryOperator::Neq => Ok(EvaluatedValue::Bool(lhs != rhs)),
                SemanticBinaryOperator::IntAdd => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Int(lhs + rhs))
                }
                SemanticBinaryOperator::IntSub => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Int(lhs - rhs))
                }
                SemanticBinaryOperator::IntMul => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Int(lhs * rhs))
                }
                SemanticBinaryOperator::IntDiv => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Int(lhs / rhs))
                }
                SemanticBinaryOperator::IntMod => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Int(lhs % rhs))
                }
                SemanticBinaryOperator::IntLe => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Bool(lhs <= rhs))
                }
                SemanticBinaryOperator::IntLt => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Bool(lhs < rhs))
                }
                SemanticBinaryOperator::IntGt => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Bool(lhs > rhs))
                }
                SemanticBinaryOperator::IntGe => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_int()?;
                    Ok(EvaluatedValue::Bool(lhs >= rhs))
                }
                SemanticBinaryOperator::BoolAnd => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_bool()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_bool()?;
                    Ok(EvaluatedValue::Bool(lhs && rhs))
                }
                SemanticBinaryOperator::BoolXor => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_bool()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_bool()?;
                    Ok(EvaluatedValue::Bool(lhs ^ rhs))
                }
                SemanticBinaryOperator::BoolOr => {
                    let lhs = lhs.as_ref().try_constexpr_evaluate()?.as_bool()?;
                    let rhs = rhs.as_ref().try_constexpr_evaluate()?.as_bool()?;
                    Ok(EvaluatedValue::Bool(lhs || rhs))
                }
            },

            Expression::Unop { op, operand } => match op {
                SemanticUnaryOperator::IntNeg => Ok(EvaluatedValue::Int(
                    -operand.as_ref().try_constexpr_evaluate()?.as_int()?,
                )),
                SemanticUnaryOperator::RealNeg => Ok(EvaluatedValue::Real(
                    -operand.as_ref().try_constexpr_evaluate()?.as_real()?,
                )),
                SemanticUnaryOperator::BoolNeg => Ok(EvaluatedValue::Bool(
                    !operand.as_ref().try_constexpr_evaluate()?.as_bool()?,
                )),
            },

            Expression::IntToBool(expression) => Ok(EvaluatedValue::Bool(
                0 != expression.as_ref().try_constexpr_evaluate()?.as_int()?,
            )),
            Expression::BoolToInt(expression) => Ok(EvaluatedValue::Int(
                expression.as_ref().try_constexpr_evaluate()?.as_bool()? as i64,
            )),
            Expression::RealToInt(expression) => Ok(EvaluatedValue::Int(
                expression.as_ref().try_constexpr_evaluate()?.as_real()? as i64,
            )),
            Expression::IntToReal(expression) => Ok(EvaluatedValue::Real(
                expression.as_ref().try_constexpr_evaluate()?.as_int()? as f64,
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

pub trait OptionalDecl {
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
    pub fn signature(&self) -> &RoutineSignature {
        match self {
            RoutineDecl::Full { signature, .. } => signature,
            RoutineDecl::Forward { signature } => signature,
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

impl SimpleDecl {
    pub fn to_generic_decl(self) -> Decl {
        match self {
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

impl SimpleBinding {
    pub fn to_generic_binding(self) -> Binding {
        Binding {
            name: self.name,
            decl: self.decl.to_generic_decl(),
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

impl Decl {
    pub fn to_simple_decl(self) -> AnalysisResult<SimpleDecl> {
        match self {
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

    pub fn to_simple_binding(self) -> AnalysisResult<SimpleBinding> {
        Ok(SimpleBinding {
            name: self.name,
            decl: self.decl.to_simple_decl()?,
        })
    }
}

#[derive(Debug)]
pub struct Program(pub Vec<Binding>);

#[derive(Debug)]
pub struct IdentifierTable {
    bindings: Vec<Binding>,
}

impl IdentifierTable {
    pub fn new() -> Self {
        IdentifierTable {
            bindings: Vec::new(),
        }
    }

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
        assert!(ident.id < self.bindings.len());
        self.bindings[ident.id].decl = new_decl;
    }

    pub fn get_binding(&self, ident: &Identifier) -> &Binding {
        &self.bindings[ident.id]
    }

    pub fn get_binding_by_id(&self, id: usize) -> &Binding {
        &self.bindings[id]
    }

    pub fn get_effective_type(&self, t: &Rc<Type>) -> AnalysisResult<Rc<Type>> {
        match &**t {
            Type::Alias(identifier) => self
                .get_binding(identifier)
                .ensure_is_type()
                .and_then(|type_decl| type_decl.get_effective()),
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
pub fn cast_to(
    expr: Rc<Expression>,
    own_type: &Rc<Type>,
    target_type: &Rc<Type>,
) -> AnalysisResult<Rc<Expression>> {
    match &**target_type {
        Type::Int => match &**own_type {
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
        Type::Real => match &**own_type {
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

        Type::Bool => match &**own_type {
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
            if element_type == *t {
                Ok(expr)
            } else {
                Err(AnalysisError {
                    what: format!("Array of {element_type} is casted to array of {t} type"),
                })
            }
        }

        Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null | Type::Unit => {
            if *own_type == *target_type || **own_type == Type::Null {
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
