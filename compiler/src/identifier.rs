use std::fmt::Debug;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct RawIdentifier {
    pub name: String,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Identifier {
    pub raw: RawIdentifier,
    pub id: usize,
}
