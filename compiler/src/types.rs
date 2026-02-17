use crate::ast::{Expression, Identifier};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct FieldDescription {
    name: Identifier,
    t: Rc<Type>,
}

struct RecordDeclaration {
    fields: Vec<FieldDescription>,
}

struct ArrayDescription {
    t: Rc<Type>,
    length: Option<usize>,
}

enum Type {
    Int,
    Real,
    Bool,
    Alias(Identifier),
    Record(RecordDeclaration),
    Array(ArrayDescription),
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unimplemented!("Can it be done with derivings?")
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        unimplemented!("Can it be done with derivings?")
    }
}

impl Eq for Type {}

fn is_primtive(t: &Type) -> bool {
    match &t {
        Type::Int | Type::Real | Type::Bool => true,
        _ => false,
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
        Expression::Call { callee, args } => unimplemented!("No context lookup yet"),
        Expression::LvalueToRvalue(inner) => unimplemented!("No context lookup yet"),
        Expression::Binop { op, lhs, rhs } => unimplemented!("Tricky type conversions"),
        Expression::BoolToInt(inner) => ensure(expr, &Type::Bool).map(|_| Rc::new(Type::Int)),
        Expression::RealToInt(inner) => ensure(expr, &Type::Real).map(|_| Rc::new(Type::Int)),
        Expression::IntToBool(inner) => ensure(expr, &Type::Int).map(|_| Rc::new(Type::Bool)),
    }
}

fn ensure(expr: &Expression, t: &Type) -> Result<(), TypeInferenceError> {
    unimplemented!("woof");
}

fn convert(expr: Rc<Expression>, source_type: &Type, dest_type: &Type) -> Rc<Expression> {
    unimplemented!("meow");
}
