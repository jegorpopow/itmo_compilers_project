use std::rc::Rc;

use derive_where::derive_where;

use common::{Integer, LoopOrder, Position, RawIdentifier, Real};
pub use lexer::Operator;

use crate::Type;

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct IntegerLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: Integer,
}

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct RealLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: Real,
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum BoolLiteral {
    True,
    False,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum LvalueExpression {
    Identifier(RawIdentifier),
    Member {
        lhs: Rc<LvalueExpression>,
        member_name: RawIdentifier,
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
        callee: RawIdentifier,
        args: Vec<Rc<Expression>>,
    },
    BinOp {
        op: Operator,
        lhs: Rc<Expression>,
        rhs: Rc<Expression>,
    },
    UnOp {
        op: Operator,
        operand: Rc<Expression>,
    },
    Cast {
        operand: Rc<Expression>,
        target: Type,
    },
    New {
        t: Type,
        fields: Option<Vec<(RawIdentifier, Rc<Expression>)>>,
    },
    Null,
}

#[derive(Debug, Hash)]
pub struct VarDecl {
    pub name: RawIdentifier,
    pub t: Option<Type>,
    pub initialiser: Option<Rc<Expression>>,
}

#[derive(Debug, Hash)]
pub struct ConstDecl {
    pub name: RawIdentifier,
    pub t: Option<Type>,
    pub initialiser: Rc<Expression>,
}

#[derive(Debug, Hash)]
pub struct TypeDecl {
    pub name: RawIdentifier,
    pub t: Type,
}

#[derive(Debug)]
pub enum RoutineBody {
    Block(Block),
    Expression(Rc<Expression>),
}

#[derive(Debug)]
pub struct RoutineDecl {
    pub name: RawIdentifier,
    pub arguments: Vec<(RawIdentifier, Type)>,
    pub return_type: Option<Type>,
    pub body: Option<RoutineBody>,
}

#[derive(Debug)]
pub enum BlockElem {
    Stmt(Statement),
    VarDecl(VarDecl),
    ConstDecl(ConstDecl),
    TypeDecl(TypeDecl),
}

#[derive(Debug)]
pub struct Block(pub Vec<BlockElem>);

#[derive(Debug)]
pub enum Statement {
    Assignment {
        lhs: Rc<LvalueExpression>,
        rhs: Rc<Expression>,
    },
    While {
        condition: Rc<Expression>,
        body: Block,
    },
    Expr(Rc<Expression>),
    If {
        condition: Rc<Expression>,
        on_true: Block,
        on_false: Option<Block>,
    },
    For {
        counter: RawIdentifier,
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
    Panic {
        pos: Position,
    },
    Assert {
        pos: Position,
        value: Rc<Expression>,
    },
}

#[derive(Debug)]
pub enum Declaration {
    Var(VarDecl),
    Const(ConstDecl),
    Type(TypeDecl),
    Routine(RoutineDecl),
}

#[derive(Debug)]
pub struct Program(pub Vec<Declaration>);
