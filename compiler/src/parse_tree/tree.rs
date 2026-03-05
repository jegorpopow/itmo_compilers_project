// #![expect(dead_code, reason = "WIP")]

use std::rc::Rc;

use derive_where::derive_where;

use crate::identifier::RawIdentifier;
use crate::operators::SyntacticOperator;
use crate::parse_tree::types::Type;

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

// #[expect(clippy::empty_structs_with_brackets, reason = "WIP")]
// pub struct SimpleDeclaration {
//     // TODO
// }

// pub enum BlockElement<Name: NameLike> {
//     Stmt(Rc<Statement<Name>>),
//     Decl(Rc<SimpleDeclaration>),
// }

// pub struct Block<Name: NameLike> {
//     elements: Vec<BlockElement<Name>>,
// }

// pub enum LoopOrder {
//     Direct,
//     Reversed,
// }

// pub enum Statement<Name: NameLike> {
//     Assignment {
//         lhs: Name,
//         rhs: Rc<Expression<Name>>,
//     },
//     While {
//         condition: Rc<Expression<Name>>,
//         body: Block<Name>,
//     },
//     If {
//         condition: Rc<Expression<Name>>,
//         on_true: Block<Name>,
//         on_false: Option<Block<Name>>,
//     },
//     For {
//         // It may be desugared into while
//         identifier: Identifier,
//         from: Rc<Expression<Name>>,
//         to: Option<Rc<Expression<Name>>>,
//         order: LoopOrder,
//         body: Block<Name>,
//     },
//     Print {
//         value: Rc<Expression<Name>>,
//     },
//     Return {
//         value: Rc<Expression<Name>>,
//     },
// }
