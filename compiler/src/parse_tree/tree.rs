#![allow(dead_code, reason = "WIP")]

use std::rc::Rc;

use derive_where::derive_where;

use crate::identifier::RawIdentifier;
use crate::operators::SyntacticOperator;
use crate::parse_tree::types::Type;
use crate::loop_order::LoopOrder;

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
    Binop {
        op: SyntacticOperator,
        lhs: Rc<Expression>,
        rhs: Rc<Expression>,
    },
    Unop {
        op: SyntacticOperator,
        operand: Rc<Expression>,
    },
    Cast {
        operand: Rc<Expression>,
        target: Rc<Type>,
    },
    New {
        t: Rc<Type>,
        fields: Option<Vec<(RawIdentifier, Rc<Expression>)>>,
    },
    Null,
}

#[derive(Debug, Hash, Clone)]
pub struct VarDecl {
    pub name: RawIdentifier,
    pub t: Option<Rc<Type>>,
    pub initialiser: Option<Rc<Expression>>,
}

#[derive(Debug, Hash, Clone)]
pub struct TypeDecl {
    pub name: RawIdentifier,
    pub t: Rc<Type>,
}

#[derive(Debug, Clone)]
pub enum RoutineBody {
    Block(Block),
    Expression(Rc<Expression>),
}

#[derive(Debug, Clone)]
pub struct RoutineDecl {
    pub name: RawIdentifier,
    pub arguments: Vec<(RawIdentifier, Rc<Type>)>,
    pub return_type: Option<Rc<Type>>,
    pub body: Option<RoutineBody>,
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
}

#[derive(Debug)]
pub enum Declaration {
    Var(VarDecl),
    Type(TypeDecl),
    Routine(RoutineDecl),
}

impl Declaration {
    #[must_use]
    pub fn name(&self) -> &RawIdentifier {
        match self {
            Declaration::Var(var_decl) => &var_decl.name,
            Declaration::Type(type_decl) => &type_decl.name,
            Declaration::Routine(routine_decl) => &routine_decl.name,
        }
    }
}

#[derive(Debug)]
pub struct Program(pub Vec<Declaration>);
