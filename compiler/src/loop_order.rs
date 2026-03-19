use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub enum LoopOrder {
    Direct,
    Reversed,
}
