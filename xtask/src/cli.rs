use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(author, about)]
#[command(
    bin_name = "cargo x",
    arg_required_else_help = true,
    help_expected = true
)]
#[derive(Debug)]
pub(crate) enum Task {
    /// Update test cases listed in source files based on tests/src/ dir content
    UpdateListings,
    /// Add a new test case.
    AddTest {
        /// Name of the test case (will be used as a filename stem).
        name: String,
        /// Stage at which the test should fail (if any).
        fail_stage: Option<Stage>,
    },
    /// Convert raytracer's output to a png.
    Render {
        /// File with raytracer's output.
        raytracer_output: PathBuf,
        /// Path to the output image.
        image: PathBuf,
    },
}

impl Task {
    #[must_use]
    #[expect(clippy::same_name_method, reason = "hiding clap under the rug")]
    pub(crate) fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum Stage {
    Lexer,
    Parser,
    AST,
    Run,
}

impl Stage {
    pub(crate) fn all() -> &'static [Self] {
        <Self as ValueEnum>::value_variants()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Task::command().debug_assert()
    }
}
