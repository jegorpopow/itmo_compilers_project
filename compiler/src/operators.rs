#![allow(dead_code, reason = "WIP")]

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum SyntacticOperator {
    Add, // Either binary or unary one
    Sub, // Either binary or unary one
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    Neg,
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
    RealEq = 8,
    RealNeq = 9,
    IntAdd = 10,
    IntSub = 11,
    IntMul = 12,
    IntDiv = 13,
    IntMod = 14,
    IntLe = 15,
    IntLt = 16,
    IntGt = 17,
    IntGe = 18,
    IntEq = 19,
    IntNeq = 20,
    BoolAnd = 21,
    BoolXor = 22,
    BoolOr = 23,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[repr(u8)]
pub enum SemanticUnaryOperator {
    IntNeg = 24,
    RealNeg = 25,
    BoolNeg = 26,
}
