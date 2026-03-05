#![allow(dead_code, reason = "WIP")]

use std::fmt::Debug;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct RawIdentifier {
    pub name: String,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Identifier {
    raw: RawIdentifier,
    id: usize,
}

pub trait NameLike: Clone + Sized + Debug + PartialEq + Eq {}

impl NameLike for RawIdentifier {}
impl NameLike for Identifier {}
