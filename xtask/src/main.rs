#![allow(
    clippy::tests_outside_test_module,
    reason = "makes more sense this way"
)]

use anyhow::Error;
use culpa::throws;
use strum::EnumIter;
use testing::Mode;

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
    #[cfg(test)]
    #[must_use]
    const fn prev(self) -> Option<Self> {
        match self {
            Self::Lexer => None,
            Self::Parser => Some(Self::Lexer),
            Self::AST => Some(Self::Parser),
            // Self::Interpreter => Some(Self::AST),
        }
    }

    fn all() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }
}

struct Both<T> {
    pass: T,
    fail: T,
}

impl<T> Both<T> {
    fn map<U>(self, f: impl Fn(T) -> U) -> Both<U> {
        let Self { pass, fail } = self;
        Both {
            pass: f(pass),
            fail: f(fail),
        }
    }

    fn iter(&self) -> impl Iterator<Item = (Mode, &T)> {
        let Self { pass, fail } = self;
        [(Mode::Pass, pass), (Mode::Fail, fail)].into_iter()
    }
}
#[throws]
fn main() {
    match cli::Task::parse() {
        cli::Task::UpdateListings => listings::update_all()?,
    }
}
