use core::fmt;

mod convert;
mod tree;
mod types;

#[cfg(test)]
mod tests;

pub use crate::{
    convert::convert,
    tree::{
        Binding, Block, BlockElem, BoolLiteral, Decl, Expression, IdentifierTable, IntegerLiteral,
        LvalueExpression, Program, RealLiteral, Routine, RoutineBody, RoutineDecl,
        RoutineSignature, SimpleBinding, SimpleDecl, Statement, TypeDecl, VarDecl,
    },
    types::{ArrayDescription, FieldDescription, RecordDescription, Type},
};

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
