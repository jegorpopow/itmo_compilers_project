use parser::Operator;

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
pub enum EqBinOp {
    Eq,
    Ne,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum BinaryOperator {
    Eq(EqBinOp),
    Real(RealBinOp),
    Int(IntBinOp),
    Bool(BoolBinOp),
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[repr(u8)]
pub enum UnaryOperator {
    IntNeg = 25,
    RealNeg = 26,
    BoolNeg = 27,
}

impl BinaryOperator {
    #[must_use]
    pub(crate) fn try_as_eq_bin_op(op: Operator) -> Option<Self> {
        match op {
            Operator::Eq => Some(Self::Eq(EqBinOp::Eq)),
            Operator::Ne => Some(Self::Eq(EqBinOp::Ne)),
            Operator::Plus
            | Operator::Minus
            | Operator::Mul
            | Operator::Div
            | Operator::Mod
            | Operator::Lt
            | Operator::Le
            | Operator::Gt
            | Operator::Ge
            | Operator::And
            | Operator::Or
            | Operator::Xor
            | Operator::Not => None,
        }
    }

    #[must_use]
    pub(crate) fn try_as_real_bin_op(op: Operator) -> Option<Self> {
        match op {
            Operator::Eq => Some(Self::Eq(EqBinOp::Eq)),
            Operator::Ne => Some(Self::Eq(EqBinOp::Ne)),
            Operator::Plus => Some(Self::Real(RealBinOp::Add)),
            Operator::Minus => Some(Self::Real(RealBinOp::Sub)),
            Operator::Mul => Some(Self::Real(RealBinOp::Mul)),
            Operator::Div => Some(Self::Real(RealBinOp::Div)),
            Operator::Lt => Some(Self::Real(RealBinOp::Lt)),
            Operator::Le => Some(Self::Real(RealBinOp::Le)),
            Operator::Gt => Some(Self::Real(RealBinOp::Gt)),
            Operator::Ge => Some(Self::Real(RealBinOp::Ge)),
            Operator::Mod | Operator::And | Operator::Or | Operator::Xor | Operator::Not => None,
        }
    }

    #[must_use]
    pub(crate) fn try_as_int_bin_op(op: Operator) -> Option<Self> {
        match op {
            Operator::Eq => Some(Self::Eq(EqBinOp::Eq)),
            Operator::Ne => Some(Self::Eq(EqBinOp::Ne)),
            Operator::Plus => Some(Self::Int(IntBinOp::Add)),
            Operator::Minus => Some(Self::Int(IntBinOp::Sub)),
            Operator::Mul => Some(Self::Int(IntBinOp::Mul)),
            Operator::Div => Some(Self::Int(IntBinOp::Div)),
            Operator::Mod => Some(Self::Int(IntBinOp::Mod)),
            Operator::Lt => Some(Self::Int(IntBinOp::Lt)),
            Operator::Le => Some(Self::Int(IntBinOp::Le)),
            Operator::Gt => Some(Self::Int(IntBinOp::Gt)),
            Operator::Ge => Some(Self::Int(IntBinOp::Ge)),
            Operator::And | Operator::Or | Operator::Xor | Operator::Not => None,
        }
    }

    #[must_use]
    pub(crate) fn try_as_bool_bin_op(op: Operator) -> Option<Self> {
        match op {
            Operator::Plus
            | Operator::Minus
            | Operator::Mul
            | Operator::Div
            | Operator::Mod
            | Operator::Lt
            | Operator::Le
            | Operator::Gt
            | Operator::Ge
            | Operator::Not => None,
            Operator::Eq => Some(Self::Eq(EqBinOp::Eq)),
            Operator::And => Some(Self::Bool(BoolBinOp::And)),
            Operator::Or => Some(Self::Bool(BoolBinOp::Or)),
            Operator::Ne | Operator::Xor => Some(Self::Bool(BoolBinOp::Xor)),
        }
    }
}
