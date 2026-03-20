use crate::ast::error::{AnalysisError, AnalysisResult};
use crate::identifier::{Identifier, RawIdentifier};
use crate::operators::{SemanticBinaryOperator, SyntacticOperator};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::rc::Rc;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FieldDescription {
    pub name: RawIdentifier,
    pub t: Rc<Type>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct RecordDescription {
    pub fields: Vec<FieldDescription>,
}

impl RecordDescription {
    fn get_field_type(&self, name: &RawIdentifier) -> AnalysisResult<Rc<Type>> {
        for field in &self.fields {
            if field.name == *name {
                return Ok(Rc::clone(&field.t));
            }
        }

        Err(AnalysisError {
            what: format!("No field of name {name} in struct with fields {self:?}"),
        })
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ArrayDescription {
    pub t: Rc<Type>,
    pub length: Option<usize>,
}

impl ArrayDescription {
    fn get_element_type(&self) -> Rc<Type> {
        Rc::clone(&self.t)
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Type {
    Int,
    Real,
    Bool,
    Alias(Identifier),
    Record(RecordDescription),
    Array(ArrayDescription),
    Null,
    Unit,
}

// impl Debug for Type {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}({})", self.raw.name, self.id)
//     }
// }

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "integer"),
            Type::Real => write!(f, "real"),
            Type::Bool => write!(f, "bool"),
            Type::Alias(identifier) => write!(f, "{identifier}"),
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
            Type::Unit => write!(f, "unit type"),
        }
    }
}

impl Type {
    fn most_precise(l: &Rc<Self>, r: &Rc<Self>) -> AnalysisResult<Rc<Self>> {
        match (&**l, &**r) {
            (Type::Int, Type::Int) => Ok(Rc::clone(l)),
            (Type::Int, Type::Real) => Ok(Rc::clone(r)),
            (Type::Int, Type::Bool) => Ok(Rc::clone(l)),
            (Type::Int, Type::Null) => Ok(Rc::clone(l)),
            (Type::Real, Type::Int) => Ok(Rc::clone(l)),
            (Type::Real, Type::Real) => Ok(Rc::clone(l)),
            (Type::Real, Type::Bool) => Ok(Rc::clone(l)),
            (Type::Real, Type::Null) => Ok(Rc::clone(l)),
            (Type::Bool, Type::Int) => Ok(Rc::clone(r)),
            (Type::Bool, Type::Real) => Ok(Rc::clone(r)),
            (Type::Bool, Type::Bool) => Ok(Rc::clone(l)),
            (Type::Bool, Type::Null) => Ok(Rc::clone(l)),
            (Type::Null, Type::Int) => Ok(Rc::clone(r)),
            (Type::Null, Type::Real) => Ok(Rc::clone(r)),
            (Type::Null, Type::Bool) => Ok(Rc::clone(r)),
            (Type::Null, Type::Null) => Ok(Rc::clone(l)),
            (_, _) => Err(AnalysisError {
                what: format!("Can not find common arithmetic type for types {l} and  {r}"),
            }),
        }
    }

    fn is_scalar(&self) -> bool {
        match self {
            Type::Null | Type::Int | Type::Real => true,
            Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Bool | Type::Unit => false,
        }
    }

    fn is_logical(&self) -> bool {
        matches!(self, Type::Bool) || self.is_scalar()
    }

    pub fn get_field_type(&self, name: &RawIdentifier) -> AnalysisResult<Rc<Type>> {
        match self {
            Type::Record(record_description) => record_description.get_field_type(name),
            Type::Int
            | Type::Real
            | Type::Bool
            | Type::Alias(_)
            | Type::Array(_)
            | Type::Null
            | Type::Unit => Err(AnalysisError {
                what: format!(
                    "Type `{self}` have no fields, but field `{name}` was requested for it"
                ),
            }),
        }
    }

    pub fn get_element_type(&self) -> AnalysisResult<Rc<Type>> {
        match self {
            Type::Array(record_description) => Ok(record_description.get_element_type()),
            Type::Int
            | Type::Real
            | Type::Bool
            | Type::Alias(_)
            | Type::Record(_)
            | Type::Null
            | Type::Unit => Err(AnalysisError {
                what: format!("Type {self} is not an array, but its element was requested"),
            }),
        }
    }

    pub fn ensure_is(&self, other: Type) -> AnalysisResult<Type> {
        if *self == other {
            Ok(other)
        } else {
            Err(AnalysisError {
                what: format!("Type mismatch: {other} expected, {self} found"),
            })
        }
    }
}

// FIXME: rewrite
/// Generates type for Binop
/// Given types of lhs, rhs and op returns (result, operand, sem_op)
/// such as: lhs `op` rhs ↦ cast_to(lhs, operand) `sem_op` cast_to(rhs, operand) :: resultW
pub fn infer_binary_operator_type(
    lhs_type: &Rc<Type>,
    rhs_type: &Rc<Type>,
    op: SyntacticOperator,
) -> AnalysisResult<(Rc<Type>, Rc<Type>, SemanticBinaryOperator)> {
    match op {
        SyntacticOperator::Neg => Err(AnalysisError {
            what: "Logical negation operator can not be applied as binary".to_string(),
        }),

        SyntacticOperator::And | SyntacticOperator::Or | SyntacticOperator::Xor => {
            if lhs_type.is_logical() && rhs_type.is_logical() {
                Ok((
                    Rc::new(Type::Bool),
                    Rc::new(Type::Bool),
                    op.to_boolean_binary_semantic().expect("Already checked"),
                ))
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply logical operator {op:?} for {lhs_type} and {rhs_type}"
                    ),
                })
            }
        }

        SyntacticOperator::Eq | SyntacticOperator::Ne => {
            if **lhs_type == **rhs_type || **lhs_type == Type::Null || **rhs_type == Type::Null {
                Ok((
                    Rc::new(Type::Bool),
                    if **lhs_type == Type::Null {
                        Rc::clone(rhs_type)
                    } else {
                        Rc::clone(lhs_type)
                    },
                    op.to_semantic_compare().expect("already checked"),
                ))
            } else {
                Err(AnalysisError {
                    what: format!("Can not apply operator {op:?}for {lhs_type} and {rhs_type}"),
                })
            }
        }

        SyntacticOperator::Add
        | SyntacticOperator::Sub
        | SyntacticOperator::Mul
        | SyntacticOperator::Div => {
            if lhs_type.is_scalar() && rhs_type.is_scalar() {
                let result_type = Type::most_precise(lhs_type, rhs_type)?;
                if matches!(&*result_type, Type::Int) {
                    Ok((
                        Rc::clone(&result_type),
                        Rc::clone(&result_type),
                        op.to_integer_binary_semantic().expect("Already checked"),
                    ))
                } else if matches!(&*result_type, Type::Real) {
                    Ok((
                        Rc::clone(&result_type),
                        Rc::clone(&result_type),
                        op.to_real_binary_semantic().expect("Already checked"),
                    ))
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

        SyntacticOperator::Mod => {
            if **lhs_type == **rhs_type && matches!(&**lhs_type, Type::Int) {
                Ok((
                    Rc::clone(lhs_type),
                    Rc::clone(lhs_type),
                    op.to_integer_binary_semantic().expect("Already checked"),
                ))
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply modulo operator {op:?} for {lhs_type} and {rhs_type}"
                    ),
                })
            }
        }

        SyntacticOperator::Lt
        | SyntacticOperator::Le
        | SyntacticOperator::Gt
        | SyntacticOperator::Ge => {
            if lhs_type.is_scalar() && rhs_type.is_scalar() {
                let result_type = Type::most_precise(lhs_type, rhs_type)?;
                if matches!(&*result_type, Type::Int) {
                    Ok((
                        Rc::clone(&result_type),
                        Rc::clone(&result_type),
                        op.to_integer_binary_semantic().expect("Already checked"),
                    ))
                } else if matches!(&*result_type, Type::Real) {
                    Ok((
                        Rc::clone(&result_type),
                        Rc::clone(&result_type),
                        op.to_real_binary_semantic().expect("Already checked"),
                    ))
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
    }
}
