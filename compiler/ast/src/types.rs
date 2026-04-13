use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::rc::Rc;

use common::RawIdentifier;

use crate::operators::BinaryOperator;
use crate::{AnalysisError, AnalysisResult, Expression, Literal};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct FieldDescription {
    pub name: RawIdentifier,
    pub t: Rc<Type>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct RecordDescription {
    pub fields: Vec<FieldDescription>,
}

impl RecordDescription {
    pub fn get_field_type(&self, name: &RawIdentifier) -> AnalysisResult<Rc<Type>> {
        Ok(Rc::clone(&self.fields[self.get_field_index(name)?].t))
    }

    pub fn get_field_index(&self, name: &RawIdentifier) -> AnalysisResult<usize> {
        for (i, field) in self.fields.iter().enumerate() {
            if field.name == *name {
                return Ok(i);
            }
        }

        Err(AnalysisError {
            what: format!(
                "No field of name {name} in struct with fields {:?}",
                self.fields
            ),
        })
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ArrayDescription {
    pub t: Rc<Type>,
    pub length: Option<usize>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Type {
    Int,
    Real,
    Bool,
    Alias(
        // FIXME: replace this with EffectiveType
        Rc<Type>,
    ),
    Record(RecordDescription),
    Array(ArrayDescription),
    Null,
}

// TODO(#65, #111): get rid of `Rc`s
impl Type {
    pub fn bool() -> Rc<Self> {
        thread_local! { static BOOL: Rc<Type> = Rc::new(Type::Bool); }
        BOOL.with(Rc::clone)
    }

    pub fn int() -> Rc<Self> {
        thread_local! { static INT: Rc<Type> = Rc::new(Type::Int); }
        INT.with(Rc::clone)
    }

    pub fn real() -> Rc<Self> {
        thread_local! { static REAL: Rc<Type> = Rc::new(Type::Real); }
        REAL.with(Rc::clone)
    }

    pub fn null() -> Rc<Self> {
        thread_local! { static NULL: Rc<Type> = Rc::new(Type::Null); }
        NULL.with(Rc::clone)
    }

    #[must_use]
    pub fn effective(self: &Rc<Self>) -> &Rc<Self> {
        if let Self::Alias(to) = self.as_ref() {
            to
        } else {
            self
        }
    }

    #[must_use]
    pub fn get_default_initialiser(&self) -> Expression {
        match self {
            Type::Int => Expression::Literal(Literal::Integer {
                repr: "0".to_string(),
                value: 0,
            }),
            Type::Real => Expression::Literal(Literal::Real {
                repr: "0.0".to_string(),
                value: 0.0,
            }),
            Type::Bool => Expression::Literal(Literal::Bool { value: false }),
            Type::Alias(ty) =>ty.get_default_initialiser(),

            Type::Record(_) | Type::Array(_) | Type::Null => Expression::Null,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "integer"),
            Type::Real => write!(f, "real"),
            Type::Bool => write!(f, "bool"),
            Type::Alias(ty) => write!(f, "alias of {ty}",),
            Type::Record(record_description) => {
                write!(f, "record")?;

                for field in &record_description.fields {
                    write!(f, " var {} is {};", field.name, field.t)?;
                }

                write!(f, "end;")
            }
            Type::Array(array_description) => {
                write!(f, "array [")?;

                if let Some(l) = array_description.length {
                    write!(f, "{l}")?;
                }
                write!(f, "] {}", array_description.t)
            }
            Type::Null => write!(f, "null type"),
        }
    }
}

impl Type {
    fn most_precise(l: &Rc<Self>, r: &Rc<Self>) -> Rc<Self> {
        match (l.as_ref(), r.as_ref()) {
            (Type::Real, Type::Real | Type::Int | Type::Bool)
            | (Type::Int, Type::Int | Type::Bool)
            | (Type::Bool, Type::Bool) => Rc::clone(l),

            (Type::Int, Type::Real) | (Type::Bool, Type::Real | Type::Int) => Rc::clone(r),

            (Type::Alias(_) | Type::Array(_) | Type::Record(_) | Type::Null, _)
            | (_, Type::Alias(_) | Type::Array(_) | Type::Record(_) | Type::Null) => {
                unreachable!()
            }
        }
    }

    fn is_scalar(&self) -> bool {
        match self {
            Type::Int | Type::Real => true,
            Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Bool | Type::Null => false,
        }
    }

    fn is_logical(&self) -> bool {
        matches!(self, Type::Bool) || self.is_scalar()
    }

    pub fn get_field_type(&self, name: &RawIdentifier) -> AnalysisResult<Rc<Type>> {
        match self {
            Type::Record(record_description) => record_description.get_field_type(name),
            Type::Int | Type::Real | Type::Bool | Type::Alias(_) | Type::Array(_) | Type::Null => {
                Err(AnalysisError {
                    what: format!(
                        "Type `{self}` have no fields, but field `{name}` was requested for it"
                    ),
                })
            }
        }
    }

    pub fn get_element_type(&self) -> AnalysisResult<&Rc<Type>> {
        match self {
            Type::Array(record_description) => Ok(&record_description.t),
            Type::Int | Type::Real | Type::Bool | Type::Alias(_) | Type::Record(_) | Type::Null => {
                Err(AnalysisError {
                    what: format!("Type {self} is not an array, but its element was requested"),
                })
            }
        }
    }

    pub fn ensure_is(&self, other: &Type) -> AnalysisResult<()> {
        if self == other {
            Ok(())
        } else {
            Err(AnalysisError {
                what: format!("Type mismatch: {other} expected, {self} found"),
            })
        }
    }
}

/// lhs `op` rhs ↦ cast_to(lhs, operand) `operator` cast_to(rhs, operand) :: result
#[derive(Debug, Clone)]
pub(crate) struct BinOpAdjustment {
    pub result: Rc<Type>,
    pub operand: Rc<Type>,
    pub operator: BinaryOperator,
}

// FIXME: rewrite
pub(crate) fn infer_binary_operator_type(
    lhs_type: &Rc<Type>,
    rhs_type: &Rc<Type>,
    op: parser::Operator,
) -> AnalysisResult<BinOpAdjustment> {
    match op {
        parser::Operator::Not => unreachable!("Parser never emits `not` as a binary operator"),

        parser::Operator::And | parser::Operator::Or | parser::Operator::Xor => {
            if lhs_type.is_logical() && rhs_type.is_logical() {
                Ok(BinOpAdjustment {
                    operand: Type::bool(),
                    result: Type::bool(),
                    operator: BinaryOperator::try_as_bool_bin_op(op).expect("Already checked"),
                })
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply logical operator {op:?} for {lhs_type} and {rhs_type}"
                    ),
                })
            }
        }

        parser::Operator::Eq | parser::Operator::Ne => {
            if **lhs_type == **rhs_type || **lhs_type == Type::Null || **rhs_type == Type::Null {
                Ok(BinOpAdjustment {
                    result: Type::bool(),
                    operand: if **lhs_type == Type::Null {
                        Rc::clone(rhs_type)
                    } else {
                        Rc::clone(lhs_type)
                    },
                    operator: BinaryOperator::try_as_eq_bin_op(op).expect("already checked"),
                })
            } else {
                Err(AnalysisError {
                    what: format!("Can not apply operator {op:?} for {lhs_type} and {rhs_type}"),
                })
            }
        }

        parser::Operator::Plus
        | parser::Operator::Minus
        | parser::Operator::Mul
        | parser::Operator::Div => {
            if lhs_type.is_scalar() && rhs_type.is_scalar() {
                let result_type = Type::most_precise(lhs_type, rhs_type);
                if let Type::Int = *result_type {
                    Ok(BinOpAdjustment {
                        operand: Rc::clone(&result_type),
                        result: result_type,
                        operator: BinaryOperator::try_as_int_bin_op(op).expect("Already checked"),
                    })
                } else if let Type::Real = *result_type {
                    Ok(BinOpAdjustment {
                        operand: Rc::clone(&result_type),
                        result: result_type,
                        operator: BinaryOperator::try_as_real_bin_op(op).expect("Already checked"),
                    })
                } else {
                    Err(AnalysisError {
                        what: format!(
                            "Can not apply arithmetic operator {op:?} for {lhs_type} and {rhs_type}"
                        ),
                    })
                }
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply arithmetic operator {op:?} for {lhs_type} and {rhs_type}"
                    ),
                })
            }
        }

        parser::Operator::Mod => {
            if let Type::Int = lhs_type.as_ref()
                && let Type::Int = rhs_type.as_ref()
            {
                Ok(BinOpAdjustment {
                    operand: Rc::clone(lhs_type),
                    result: Rc::clone(lhs_type),
                    operator: BinaryOperator::try_as_int_bin_op(op).expect("Already checked"),
                })
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply modulo operator {op:?} for {lhs_type} and {rhs_type}"
                    ),
                })
            }
        }

        parser::Operator::Lt
        | parser::Operator::Le
        | parser::Operator::Gt
        | parser::Operator::Ge => {
            if lhs_type.is_scalar() && rhs_type.is_scalar() {
                let operand_type = Type::most_precise(lhs_type, rhs_type);
                if let Type::Int = *operand_type {
                    Ok(BinOpAdjustment {
                        result: Type::bool(),
                        operand: operand_type,
                        operator: BinaryOperator::try_as_int_bin_op(op).expect("Already checked"),
                    })
                } else if let Type::Real = *operand_type {
                    Ok(BinOpAdjustment {
                        result: Type::bool(),
                        operand: operand_type,
                        operator: BinaryOperator::try_as_real_bin_op(op).expect("Already checked"),
                    })
                } else {
                    Err(AnalysisError {
                        what: format!(
                            "Can not apply order comparison operator {op:?} for {lhs_type} and {rhs_type}"
                        ),
                    })
                }
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply order comparison operator {op:?} for {lhs_type} and {rhs_type}"
                    ),
                })
            }
        }
    }
}
