use core::fmt;
use std::rc::Rc;

mod convert;
mod operators;
mod tree;
mod types;

#[cfg(test)]
mod tests;

pub use crate::{
    convert::convert,
    operators::{BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp, UnaryOperator},
    tree::{
        Binding, Block, BoolLiteral, ConstDecl, Decl, Expression, IdentifierTable, IntegerLiteral,
        LocalBinding, LocalDecl, LvalueExpression, Program, RealLiteral, Routine, RoutineBody,
        RoutineDecl, RoutineSignature, Statement, TypeDecl, VarDecl,
    },
    types::{ArrayDescription, FieldDescription, RecordDescription, Type},
};

#[derive(Debug)]
struct Typed<T = Expression> {
    value: Rc<T>,
    ty: Rc<Type>,
}

impl<T> Typed<T> {
    #[must_use]
    fn map<U>(self, f: impl FnOnce(Rc<T>) -> Rc<U>) -> Typed<U> {
        let Self { value, ty } = self;
        Typed {
            value: f(value),
            ty,
        }
    }
}

#[derive(Debug)]
pub struct AnalysisError {
    pub what: String,
}

pub type AnalysisResult<T> = Result<T, AnalysisError>;

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { what } = self;
        f.write_str(what)
    }
}

impl core::error::Error for AnalysisError {}
