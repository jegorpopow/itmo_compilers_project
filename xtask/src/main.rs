#![allow(
    clippy::tests_outside_test_module,
    reason = "makes more sense this way"
)]

use anyhow::Error;
use culpa::throws;
use strum::EnumIter;

mod cli;
mod folders;
mod listings;

#[derive(Clone, Copy, Debug, EnumIter)]
enum Stage {
    Lexer,
    Parser,
    AST,
    // TODO(GrigorenkoPV)
    // Interpreter,
}

impl Stage {
    // TODO(GrigorenkoPV)
    #[cfg(false)]
    #[must_use]
    const fn prev(self) -> Option<Self> {
        match self {
            Self::Lexer => None,
            Self::Parser => Some(Self::Lexer),
            Self::AST => Some(Self::Parser),
        }
    }

    fn all() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }
}

#[throws]
fn main() {
    match cli::Task::parse() {
        cli::Task::UpdateListings => listings::update_all()?,
    }
}
