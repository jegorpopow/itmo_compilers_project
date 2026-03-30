#![cfg_attr(
    test,
    expect(
        clippy::tests_outside_test_module,
        reason = "makes more sense this way"
    )
)]

use core::cmp::Ordering;

use anyhow::Error;
use culpa::throws;
use testing::Mode;

mod cli;
mod folders;
mod listings;

pub(crate) use cli::Stage;

impl Stage {
    #[must_use]
    const fn prev(self) -> Option<Self> {
        match self {
            Self::Lexer => None,
            Self::Parser => Some(Self::Lexer),
            Self::AST => Some(Self::Parser),
            Self::Run => Some(Self::AST),
        }
    }
}

impl PartialOrd for Stage {
    fn lt(&self, other: &Self) -> bool {
        let mut other = *other;
        while let Some(parent) = other.prev() {
            if *self == parent {
                return true;
            }
            other = parent;
        }
        false
    }

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self < other {
            Some(Ordering::Less)
        } else if other < self {
            Some(Ordering::Greater)
        } else {
            None
        }
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
        cli::Task::AddTest { name, fail_stage } => {
            let new_src = folders::add_test(&name, fail_stage)?;
            listings::update_all()?;
            println!(
                "\n\nYour new source file, sir:\n\t\t{}\n",
                new_src.display()
            )
        }
    }
}
