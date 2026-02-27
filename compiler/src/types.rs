#![expect(dead_code, reason = "WIP")]

use crate::ast::{Expression, Identifier};
use std::rc::Rc;

#[derive(Debug, Hash, PartialEq, Eq)]
struct FieldDescription {
    name: Identifier,
    t: Rc<Type>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct RecordDeclaration {
    fields: Vec<FieldDescription>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct ArrayDescription {
    t: Rc<Type>,
    length: Option<usize>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum Type {
    Int,
    Real,
    Bool,
    Alias(Identifier),
    Record(RecordDeclaration),
    Array(ArrayDescription),
}

fn is_primitive(t: &Type) -> bool {
    match &t {
        Type::Int | Type::Real | Type::Bool => true,
        Type::Alias(_) | Type::Record(_) | Type::Array(_) => false,
    }
}

struct TypeInferenceError {
    reason: String,
}

fn infer(expr: &Expression) -> Result<Rc<Type>, TypeInferenceError> {
    match &expr {
        Expression::IntegerLiteral(_) => Ok(Rc::new(Type::Int)),
        Expression::RealLiteral(_) => Ok(Rc::new(Type::Real)),
        Expression::BoolLiteral(_) => Ok(Rc::new(Type::Bool)),
        Expression::Call { .. } => todo!("No context lookup yet"),
        Expression::LvalueToRvalue(..) => todo!("No context lookup yet"),
        Expression::Binop { .. } => todo!("Tricky type conversions"),
        Expression::BoolToInt(inner) => ensure(inner, &Type::Bool).map(|()| Rc::new(Type::Int)),
        Expression::RealToInt(inner) => ensure(inner, &Type::Real).map(|()| Rc::new(Type::Int)),
        Expression::IntToBool(inner) => ensure(inner, &Type::Int).map(|()| Rc::new(Type::Bool)),
    }
}

fn ensure(expr: &Expression, t: &Type) -> Result<(), TypeInferenceError> {
    todo!("Check if {expr:?} : {t:?}")
}

fn convert(expr: &Expression, source_type: &Type, dest_type: &Type) -> Rc<Expression> {
    todo!("Convert {expr:?} from {source_type:?} to {dest_type:?}")
}
