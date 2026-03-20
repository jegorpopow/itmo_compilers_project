#![allow(dead_code, reason = "WIP")]

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum SyntacticOperator {
    Add, // Either binary or unary one
    Sub, // Either binary or unary one
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    Neg,
}

impl SyntacticOperator {
    #[must_use]
    pub fn to_semantic_compare(self) -> Option<SemanticBinaryOperator> {
        match self {
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::Ne => Some(SemanticBinaryOperator::Ne),
            SyntacticOperator::Add
            | SyntacticOperator::Sub
            | SyntacticOperator::Mul
            | SyntacticOperator::Div
            | SyntacticOperator::Mod
            | SyntacticOperator::Lt
            | SyntacticOperator::Le
            | SyntacticOperator::Gt
            | SyntacticOperator::Ge
            | SyntacticOperator::And
            | SyntacticOperator::Or
            | SyntacticOperator::Xor
            | SyntacticOperator::Neg => None,
        }
    }

    #[must_use]
    pub fn to_real_binary_semantic(self) -> Option<SemanticBinaryOperator> {
        match self {
            SyntacticOperator::Add => Some(SemanticBinaryOperator::RealAdd),
            SyntacticOperator::Sub => Some(SemanticBinaryOperator::RealSub),
            SyntacticOperator::Mul => Some(SemanticBinaryOperator::RealMul),
            SyntacticOperator::Div => Some(SemanticBinaryOperator::RealDiv),
            SyntacticOperator::Mod => None,
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::Ne => Some(SemanticBinaryOperator::Ne),
            SyntacticOperator::Lt => Some(SemanticBinaryOperator::RealLt),
            SyntacticOperator::Le => Some(SemanticBinaryOperator::RealLe),
            SyntacticOperator::Gt => Some(SemanticBinaryOperator::RealGt),
            SyntacticOperator::Ge => Some(SemanticBinaryOperator::RealGe),
            SyntacticOperator::And
            | SyntacticOperator::Or
            | SyntacticOperator::Xor
            | SyntacticOperator::Neg => None,
        }
    }

    #[must_use]
    pub fn to_integer_binary_semantic(self) -> Option<SemanticBinaryOperator> {
        match self {
            SyntacticOperator::Add => Some(SemanticBinaryOperator::IntAdd),
            SyntacticOperator::Sub => Some(SemanticBinaryOperator::IntSub),
            SyntacticOperator::Mul => Some(SemanticBinaryOperator::IntMul),
            SyntacticOperator::Div => Some(SemanticBinaryOperator::IntDiv),
            SyntacticOperator::Mod => Some(SemanticBinaryOperator::IntMod),
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::Ne => Some(SemanticBinaryOperator::Ne),
            SyntacticOperator::Lt => Some(SemanticBinaryOperator::IntLt),
            SyntacticOperator::Le => Some(SemanticBinaryOperator::IntLe),
            SyntacticOperator::Gt => Some(SemanticBinaryOperator::IntGt),
            SyntacticOperator::Ge => Some(SemanticBinaryOperator::IntGe),
            SyntacticOperator::And
            | SyntacticOperator::Or
            | SyntacticOperator::Xor
            | SyntacticOperator::Neg => None,
        }
    }

    #[must_use]
    pub fn to_boolean_binary_semantic(self) -> Option<SemanticBinaryOperator> {
        match self {
            SyntacticOperator::Add
            | SyntacticOperator::Sub
            | SyntacticOperator::Mul
            | SyntacticOperator::Div
            | SyntacticOperator::Mod
            | SyntacticOperator::Lt
            | SyntacticOperator::Le
            | SyntacticOperator::Gt
            | SyntacticOperator::Ge => None,
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::Ne => Some(SemanticBinaryOperator::BoolXor),
            SyntacticOperator::And => Some(SemanticBinaryOperator::BoolAnd),
            SyntacticOperator::Or => Some(SemanticBinaryOperator::BoolOr),
            SyntacticOperator::Xor => Some(SemanticBinaryOperator::BoolXor),
            SyntacticOperator::Neg => None,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[repr(u8)]
pub enum SemanticBinaryOperator {
    RealAdd = 0,
    RealSub = 1,
    RealMul = 2,
    RealDiv = 3,
    RealLe = 4,
    RealLt = 5,
    RealGt = 6,
    RealGe = 7,
    Eq = 8,
    Ne = 9,
    IntAdd = 10,
    IntSub = 11,
    IntMul = 12,
    IntDiv = 13,
    IntMod = 14,
    IntLe = 15,
    IntLt = 16,
    IntGt = 17,
    IntGe = 18,
    BoolAnd = 21,
    BoolXor = 22,
    BoolOr = 23,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[repr(u8)]
pub enum SemanticUnaryOperator {
    IntNeg = 25,
    RealNeg = 26,
    BoolNeg = 27,
}
