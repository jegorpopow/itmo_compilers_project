#![allow(
    clippy::tests_outside_test_module,
    reason = "makes more sense this way"
)]

use std::path::{Path, PathBuf};

use anyhow::Error;
use culpa::throws;
use strum::EnumIter;

mod cli;
mod folders;
mod listings;
mod utils;

use crate::utils::PathExt as _;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .make_relative_if_possible()
}

fn tests_dir() -> PathBuf {
    workspace_root().append(&["tests"])
}

fn tests_src_dir() -> PathBuf {
    tests_dir().append(&["src"])
}

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
