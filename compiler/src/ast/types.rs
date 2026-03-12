use crate::ast::error::{AnalysisError, AnalysisResult};
use crate::identifier::{Identifier, RawIdentifier};
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
