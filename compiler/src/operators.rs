#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum SyntacticOperator {
    Add,
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

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum SemanticBinaryOperator {
    RealAdd,
    RealSub,
    RealMul,
    RealDiv,
    RealLe,
    RealLg,
    RealGt,
    RealGe,
    RealEq,
    RealNeq,
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntMod,
    IntLe,
    IntLg,
    IntGt,
    IntGe,
    IntEq,
    IntNeq,
    BoolAnd,
    BoolXor,
    BoolOr,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
#[expect(clippy::enum_variant_names, reason = "TODO")]
pub enum SemanticUnaryOperator {
    IntNeg,
    RealNeg,
    BoolNeg,
}
