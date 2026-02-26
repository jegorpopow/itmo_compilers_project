use clap::Parser;

#[derive(Parser)]
#[command(author, about)]
#[command(
    bin_name = "cargo x",
    arg_required_else_help = true,
    help_expected = true,
)]
#[derive(Debug)]
pub(crate) enum Task {
    /// Update test cases listed in lexer src based on tests/ dir content
    UpdateLexerTests,
}

impl Task {
    #[must_use]
    #[expect(clippy::same_name_method, reason = "hiding clap under the rug")]
    pub(crate) fn parse() -> Self {
        <Self as Parser>::parse()
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
