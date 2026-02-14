use std::rc::Rc;

enum BinaryOperator {
    And,
    Or,
    Xor,
    Le,
    Lg,
    Gt,
    Ge,
    Eq,
    Neq,
    Mul,
    Div,
    Mod,
    Add,
    Sub,
}

struct Identifier {
    name: String,
}

struct IntegerLiteral {
    repr: String,
    value: i64, // Encloses sign and negation
}

struct RealLiteral {
    repr: String,
    value: f64, // Encloses sign
}

enum BoolLiteral {
    True,
    False,
}

enum LvalueExpression {
    Identifier(Identifier),
    Member {
        lhs: Rc<LvalueExpression>,
        member_name: Identifier,
    },
    Index {
        lhs: Rc<LvalueExpression>,
        index: Rc<Expression>,
    },
}

enum Expression {
    LvalueToRvalue(Rc<LvalueExpression>),
    IntegerLiteral(IntegerLiteral),
    RealLiteral(RealLiteral),
    BoolLiteral(BoolLiteral),
    Call {
        callee: Identifier,
        args: Vec<Rc<Expression>>,
    },
    Binop {
        op: BinaryOperator,
        lhs: Rc<Expression>,
        rhs: Rc<Expression>,
    },
    BoolToInt(Rc<Expression>),
    RealToInt(Rc<Expression>),
    IntToBool(Rc<Expression>), // It cannot be expressed as value != 0, since it shoould panic on value out of [0:1]
}

struct FieldDeclaration {
    name: Identifier,
    t: Rc<Type>,
}

struct RecordDeclaration {
    fields: Vec<FieldDeclaration>,
}

struct ArrayDeclaration {
    t: Rc<Type>,
    length: Option<usize>,
}

enum Type {
    Int,
    Real,
    Bool,
    Alias(String),
    Record(RecordDeclaration),
    Array(ArrayDeclaration),
}

fn is_primtive(t: &Type) -> bool {
    match t {
        Type::Int | Type::Real | Type::Bool => true,
        _ => false,
    }
}

struct TypeInferenceError {
    reason: String,
}

struct TypeCoercionError {
    reason: String,
}

fn infer(expr: &Expression) -> Result<Rc<Type>, TypeInferenceError> {
    match expr {
        Expression::IntegerLiteral(_) => Ok(Rc::new(Type::Int)),
        Expression::RealLiteral(_) => Ok(Rc::new(Type::Real)),
        Expression::BoolLiteral(_) => Ok(Rc::new(Type::Bool)),
        Expression::Call { callee, args } => unimplemented!("No context lookup yet"),
        Expression::LvalueToRvalue(inner) => unimplemented!("No context lookup yet"),
        Expression::Binop { op, lhs, rhs } => unimplemented!("Tricky type conversions"),
        Expression::BoolToInt(inner) => Ok(Rc::new(Type::Int)), // Type correctness will probably be checked elsewhere
        Expression::RealToInt(inner) => Ok(Rc::new(Type::Int)),
        Expression::IntToBool(inner) => Ok(Rc::new(Type::Bool)),
    }
}

fn coerce(
    expr: Rc<Expression>,
    source_type: &Type,
    dest_type: &Type,
) -> Result<Rc<Expression>, TypeCoercionError> {
    unimplemented!("meow");
}

fn typecheck(expr: &Expression) -> Option<TypeInferenceError> {
    unimplemented!("woof");
}

struct SimpleDeclaration {}

enum BlockElement {
    stmt(Rc<Statement>),
    decl(Rc<SimpleDeclaration>),
}

struct Block {
    elements: Vec<BlockElement>,
}

enum LoopOrder {
    Direct,
    Reversed,
}

enum Statement {
    Assignment {
        lhs: Identifier,
        rhs: Rc<Expression>,
    },
    While {
        condition: Rc<Expression>,
        body: Block,
    },
    If {
        condition: Rc<Expression>,
        on_true: Block,
        on_false: Option<Block>,
    },
    For {
        // It may be desugared into while
        identifier: Identifier,
        from: Rc<Expression>,
        to: Option<Rc<Expression>>,
        order: LoopOrder,
        body: Block,
    },
    Print {
        value: Rc<Expression>,
    },
    Return {
        value: Rc<Expression>,
    },
}
