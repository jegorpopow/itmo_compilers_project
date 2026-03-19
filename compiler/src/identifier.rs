use std::fmt::{Debug, Display};

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct RawIdentifier {
    pub name: String,
}

impl Debug for RawIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r\"{}\"", self.name)
    }
}

impl Display for RawIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Identifier {
    pub raw: RawIdentifier,
    pub id: usize,
}

impl Debug for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.raw.name, self.id)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw.name)
    }
}
