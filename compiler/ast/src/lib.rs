use core::fmt;
use std::rc::Rc;

mod convert;
mod operators;
mod tree;
mod types;

#[cfg(test)]
mod tests;

use common::RawIdentifier;

pub use crate::{
    convert::convert,
    operators::{BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp, UnaryOperator},
    tree::{
        Block, ConstDecl, Decl, EvaluatedValue, Expression, Literal, LvalueExpression, Program,
        Routine, RoutineBody, RoutineDecl, RoutineSignature, Statement, TypeDecl, VarDecl,
    },
    types::{ArrayDescription, FieldDescription, RecordDescription, Type},
};

pub type Bindings = indexed_arena::Arena<Decl, usize>;
pub type BindingId = indexed_arena::Idx<Decl, usize>;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Identifier {
    pub raw: RawIdentifier,
    pub id: BindingId,
}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { raw, id } = self;
        write!(f, "{raw}({})", id.into_raw())
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { raw, id: _ } = self;
        raw.fmt(f)
    }
}

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
