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
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::Ne => Some(SemanticBinaryOperator::Ne),
            SyntacticOperator::Add => Some(SemanticBinaryOperator::Real(RealBinOp::Add)),
            SyntacticOperator::Sub => Some(SemanticBinaryOperator::Real(RealBinOp::Sub)),
            SyntacticOperator::Mul => Some(SemanticBinaryOperator::Real(RealBinOp::Mul)),
            SyntacticOperator::Div => Some(SemanticBinaryOperator::Real(RealBinOp::Div)),
            SyntacticOperator::Lt => Some(SemanticBinaryOperator::Real(RealBinOp::Lt)),
            SyntacticOperator::Le => Some(SemanticBinaryOperator::Real(RealBinOp::Le)),
            SyntacticOperator::Gt => Some(SemanticBinaryOperator::Real(RealBinOp::Gt)),
            SyntacticOperator::Ge => Some(SemanticBinaryOperator::Real(RealBinOp::Ge)),
            SyntacticOperator::Mod
            | SyntacticOperator::And
            | SyntacticOperator::Or
            | SyntacticOperator::Xor
            | SyntacticOperator::Neg => None,
        }
    }

    #[must_use]
    pub fn to_integer_binary_semantic(self) -> Option<SemanticBinaryOperator> {
        match self {
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::Ne => Some(SemanticBinaryOperator::Ne),
            SyntacticOperator::Add => Some(SemanticBinaryOperator::Int(IntBinOp::Add)),
            SyntacticOperator::Sub => Some(SemanticBinaryOperator::Int(IntBinOp::Sub)),
            SyntacticOperator::Mul => Some(SemanticBinaryOperator::Int(IntBinOp::Mul)),
            SyntacticOperator::Div => Some(SemanticBinaryOperator::Int(IntBinOp::Div)),
            SyntacticOperator::Mod => Some(SemanticBinaryOperator::Int(IntBinOp::Mod)),
            SyntacticOperator::Lt => Some(SemanticBinaryOperator::Int(IntBinOp::Lt)),
            SyntacticOperator::Le => Some(SemanticBinaryOperator::Int(IntBinOp::Le)),
            SyntacticOperator::Gt => Some(SemanticBinaryOperator::Int(IntBinOp::Gt)),
            SyntacticOperator::Ge => Some(SemanticBinaryOperator::Int(IntBinOp::Ge)),
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
            | SyntacticOperator::Ge
            | SyntacticOperator::Neg => None,
            SyntacticOperator::Eq => Some(SemanticBinaryOperator::Eq),
            SyntacticOperator::And => Some(SemanticBinaryOperator::Bool(BoolBinOp::And)),
            SyntacticOperator::Or => Some(SemanticBinaryOperator::Bool(BoolBinOp::Or)),
            SyntacticOperator::Ne | SyntacticOperator::Xor => {
                Some(SemanticBinaryOperator::Bool(BoolBinOp::Xor))
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum RealBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Le,
    Lt,
    Gt,
    Ge,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum IntBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Le,
    Lt,
    Gt,
    Ge,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum BoolBinOp {
    And,
    Or,
    Xor,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum SemanticBinaryOperator {
    Eq,
    Ne,
    Real(RealBinOp),
    Int(IntBinOp),
    Bool(BoolBinOp),
}

#[allow(
    clippy::enum_variant_names,
    clippy::allow_attributes,
    reason = "Negation is the only unary operation that we have
    (expect seems to be broken here, yippee)"
)]
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[repr(u8)]
pub enum SemanticUnaryOperator {
    IntNeg = 25,
    RealNeg = 26,
    BoolNeg = 27,
}
