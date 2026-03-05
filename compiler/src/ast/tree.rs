#![allow(dead_code, reason = "WIP")]

use std::rc::Rc;

use crate::ast::types::Type;
use crate::identifier::{Identifier, RawIdentifier};

use compiler::operators::{SemanticBinaryOperator, SemanticUnaryOperator};
use derive_where::derive_where;

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct IntegerLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: i64,
}

#[derive(Debug)]
#[derive_where(Hash, Eq, PartialEq)]
pub struct RealLiteral {
    pub repr: String,
    #[derive_where(skip(EqHashOrd))]
    pub value: f64,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum BoolLiteral {
    True,
    False,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum LvalueExpression {
    Identifier(Identifier),
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
        callee: Identifier,
        args: Vec<Rc<Expression>>,
    },
    Binop {
        op: SemanticBinaryOperator,
        lhs: Rc<Expression>,
        rhs: Rc<Expression>,
    },
    Unop {
        op: SemanticUnaryOperator,
        operand: Rc<Expression>,
    },
    Cast {
        operand: Rc<Expression>,
        target: Rc<Type>,
    },
    New {
        t: Rc<Type>,
        fields: Option<Vec<(Identifier, Rc<Expression>)>>,
    },
    Null,
    IntToBool(Rc<Type>),
    BoolToInt(Rc<Type>),
    RealToInt(Rc<Type>),
    IntToReal(Rc<Type>),
}

#[derive(Debug, Hash, Clone)]
pub struct VarDecl {
    pub name: Identifier,
    pub t: Rc<Type>,
    pub initialiser: Option<Rc<Expression>>,
}

#[derive(Debug, Hash, Clone)]
pub struct TypeDecl {
    pub name: Identifier,
    pub t: Rc<Type>,
}

#[derive(Debug, Clone)]
pub enum RoutineBody {
    Block(Block),
    Expression(Rc<Expression>),
}

#[derive(Debug, Clone)]
pub struct RoutineDecl {
    pub name: Identifier,
    pub arguments: Vec<(RawIdentifier, Rc<Type>)>,
    pub return_type: Rc<Type>,
    pub body: Option<RoutineBody>,
}

#[derive(Debug, Clone, Copy)]
pub enum LoopOrder {
    Direct,
    Reversed,
}

#[derive(Debug, Clone)]
pub enum BlockElem {
    Stmt(Statement),
    VarDecl(VarDecl),
    TypeDecl(TypeDecl),
}

#[derive(Debug, Clone)]
pub struct Block(pub Vec<BlockElem>);

#[derive(Debug, Clone)]
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
        counter: Identifier,
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

#[derive(Debug)]
pub enum Declaration {
    Var(VarDecl),
    Type(TypeDecl),
    Routine(RoutineDecl),
}
