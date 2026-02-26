use std::rc::Rc;

use derive_where::derive_where;

#[derive(Debug, Hash, PartialEq, Eq)]
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

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Identifier {
    name: String,
    id: Option<usize>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct IntegerLiteral {
    repr: String,
    value: i64, // Encloses sign and negation
}

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct RealLiteral {
    repr: String,
    #[derive_where(skip(EqHashOrd))]
    value: f64, // Encloses sign
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum BoolLiteral {
    True,
    False,
}

#[derive(Debug, Hash, PartialEq, Eq)]
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

#[derive(Debug, Hash, PartialEq, Eq)]
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
#[expect(clippy::empty_structs_with_brackets, reason = "WIP")]
pub struct SimpleDeclaration {
    // TODO
}

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
