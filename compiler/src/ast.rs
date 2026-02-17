use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub enum BinaryOperator {
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

pub struct Identifier {
    name: String,
    id: Option<usize>,
}

pub struct IntegerLiteral {
    repr: String,
    value: i64, // Encloses sign and negation
}

pub struct RealLiteral {
    repr: String,
    value: f64, // Encloses sign
}

pub enum BoolLiteral {
    True,
    False,
}

pub enum LvalueExpression {
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

pub enum Expression {
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

impl Hash for Expression {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unimplemented!("Can it be done with derivings?")
    }
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        unimplemented!("Can it be done with derivings?")
    }
}

impl Eq for Expression {}

pub struct SimpleDeclaration {}

pub enum BlockElement {
    Stmt(Rc<Statement>),
    Decl(Rc<SimpleDeclaration>),
}

pub struct Block {
    elements: Vec<BlockElement>,
}

pub enum LoopOrder {
    Direct,
    Reversed,
}

pub enum Statement {
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
