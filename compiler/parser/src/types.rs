use std::fmt::Debug;
use std::rc::Rc;

use common::RawIdentifier;

use crate::Expression;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FieldDescription {
    pub name: RawIdentifier,
    pub t: Rc<Type>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct RecordDescription {
    pub fields: Vec<FieldDescription>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ArrayDescription {
    pub t: Rc<Type>,
    pub length: Option<Rc<Expression>>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Type {
    Int,
    Real,
    Bool,
    Alias(RawIdentifier),
    Record(RecordDescription),
    Array(ArrayDescription),
}
