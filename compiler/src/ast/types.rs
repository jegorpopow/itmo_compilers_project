use crate::ast::error::{AnalysisError, AnalysisResult};
use crate::identifier::{Identifier, RawIdentifier};
use crate::operators::{SemanticBinaryOperator, SyntacticOperator};
use std::fmt::Debug;
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
            what: format!("No field of name {name:?} in struct with fields {self:?}"),
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

impl Type {
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
                what: format!("Type {self:?} have no fields"),
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
                what: format!("Type {self:?} is not an array"),
            }),
        }
    }

    pub fn ensure_is(&self, other: Type) -> AnalysisResult<Type> {
        if *self == other {
            Ok(other)
        } else {
            Err(AnalysisError {
                what: format!("Type mismatch {other:?} expected, {self:?} found"),
            })
        }
    }
}

// FIXME: rewrite
// TODO: add implicit conversions
pub fn infer_binary_operator_type(
    lhs_type: &Rc<Type>,
    rhs_type: &Rc<Type>,
    op: SyntacticOperator,
) -> AnalysisResult<(Rc<Type>, SemanticBinaryOperator)> {
    match op {
        SyntacticOperator::Neg => Err(AnalysisError {
            what: "Logical negation operator can not be applied as binary".to_string(),
        }),

        SyntacticOperator::And | SyntacticOperator::Or | SyntacticOperator::Xor => {
            if matches!(&**lhs_type, Type::Bool) && matches!(&**rhs_type, Type::Bool) {
                Ok((
                    Rc::new(Type::Bool),
                    op.to_boolean_binary_semantic().expect("Already checked"),
                ))
            } else {
                Err(AnalysisError {
                    what: format!(
                        "Can not apply operator {op:?} for {lhs_type:?} and {rhs_type:?}"
                    ),
                })
            }
        }

        SyntacticOperator::Eq | SyntacticOperator::Neq => {
            if **lhs_type == **rhs_type {
                // TODO: add reference comparasion
                if matches!(&**lhs_type, Type::Bool) {
                    Ok((
                        Rc::new(Type::Bool),
                        op.to_boolean_binary_semantic().expect("Already checked"),
                    ))
                } else if matches!(&**lhs_type, Type::Int) {
                    Ok((
                        Rc::new(Type::Bool),
                        op.to_integer_binary_semantic().expect("Already checked"),
                    ))
                } else if matches!(&**lhs_type, Type::Real) {
                    Ok((
                        Rc::new(Type::Bool),
                        op.to_real_binary_semantic().expect("Already checked"),
                    ))
                } else {
                    Err(AnalysisError {
                        what: format!(
                            "Can not apply operator {op:?} for {lhs_type:?} and {rhs_type:?}"
                        ),
                    })
                }
            } else {
                Err(AnalysisError {
                    what: format!("Can not apply operator {op:?}for {lhs_type:?} and {rhs_type:?}"),
                })
            }
        }

        SyntacticOperator::Add
        | SyntacticOperator::Sub
        | SyntacticOperator::Mul
        | SyntacticOperator::Div => {
            if **lhs_type == **rhs_type {
                if matches!(&**lhs_type, Type::Int) {
                    Ok((
                        Rc::clone(lhs_type),
                        op.to_integer_binary_semantic().expect("Already checked"),
                    ))
                } else if matches!(&**lhs_type, Type::Real) {
                    Ok((
                        Rc::clone(lhs_type),
                        op.to_real_binary_semantic().expect("Already checked"),
                    ))
                } else {
                    Err(AnalysisError {
                        what: format!(
                            "Can not apply operator {op:?}for {lhs_type:?} and {rhs_type:?}"
                        ),
                    })
                }
            } else {
                Err(AnalysisError {
                    what: format!("Can not apply operator {op:?}for {lhs_type:?} and {rhs_type:?}"),
                })
            }
        }

        SyntacticOperator::Mod => {
            if **lhs_type == **rhs_type && matches!(&**lhs_type, Type::Int) {
                Ok((
                    Rc::clone(lhs_type),
                    op.to_integer_binary_semantic().expect("Already checked"),
                ))
            } else {
                Err(AnalysisError {
                    what: format!("Can not apply operator {op:?}for {lhs_type:?} and {rhs_type:?}"),
                })
            }
        }

        SyntacticOperator::Lt
        | SyntacticOperator::Le
        | SyntacticOperator::Gt
        | SyntacticOperator::Ge => {
            if **lhs_type == **rhs_type {
                if matches!(&**lhs_type, Type::Int) {
                    Ok((
                        Rc::new(Type::Bool),
                        op.to_integer_binary_semantic().expect("Already checked"),
                    ))
                } else if matches!(&**lhs_type, Type::Real) {
                    Ok((
                        Rc::new(Type::Bool),
                        op.to_real_binary_semantic().expect("Already checked"),
                    ))
                } else {
                    Err(AnalysisError {
                        what: format!(
                            "Can not apply operator {op:?}for {lhs_type:?} and {rhs_type:?}"
                        ),
                    })
                }
            } else {
                Err(AnalysisError {
                    what: format!("Can not apply operator {op:?}for {lhs_type:?} and {rhs_type:?}"),
                })
            }
        }
    }
}
