use derive_where::derive_where;

use common::{Integer, LoopOrder, Position, RawIdentifier, Real};
pub use lexer::Operator;

use crate::Type;

#[derive(Debug, Clone)]
#[derive_where(Hash, Eq, PartialEq)]
pub enum Literal {
    Bool {
        value: bool,
    },
    Integer {
        repr: String,
        #[derive_where(skip(EqHashOrd))]
        value: Integer,
    },
    Real {
        repr: String,
        #[derive_where(skip(EqHashOrd))]
        value: Real,
    },
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum LvalueExpression {
    Identifier(RawIdentifier),
    Member {
        lhs: Box<LvalueExpression>,
        member_name: RawIdentifier,
    },
    Index {
        lhs: Box<LvalueExpression>,
        index: Box<Expression>,
    },
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Expression {
    LvalueToRvalue(LvalueExpression),
    Literal(Literal),
    Call {
        callee: RawIdentifier,
        args: Vec<Expression>,
    },
    BinOp {
        op: Operator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    UnOp {
        op: Operator,
        operand: Box<Expression>,
    },
    Cast {
        operand: Box<Expression>,
        target: Box<Type>,
    },
    New {
        t: Box<Type>,
        fields: Option<Vec<(RawIdentifier, Expression)>>,
        array_length: Option<Box<Expression>>,
    },
    Null,
}

#[derive(Debug, Hash)]
pub struct VarDecl {
    pub name: RawIdentifier,
    pub t: Option<Type>,
    pub initialiser: Option<Expression>,
}

#[derive(Debug, Hash)]
pub struct ConstDecl {
    pub name: RawIdentifier,
    pub t: Option<Type>,
    pub initialiser: Expression,
}

#[derive(Debug, Hash)]
pub struct TypeDecl {
    pub name: RawIdentifier,
    pub t: Type,
}

#[derive(Debug)]
pub enum RoutineBody {
    Block(Block),
    Expression(Expression),
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
        lhs: LvalueExpression,
        rhs: Expression,
    },
    While {
        condition: Expression,
        body: Block,
    },
    Expr(Expression),
    If {
        condition: Expression,
        on_true: Block,
        on_false: Option<Block>,
    },
    For {
        counter: RawIdentifier,
        from: Expression,
        to: Option<Expression>,
        order: LoopOrder,
        body: Block,
    },
    Print {
        value: Expression,
    },
    Return {
        value: Expression,
    },
    Panic {
        pos: Position,
    },
    Assert {
        pos: Position,
        value: Expression,
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
