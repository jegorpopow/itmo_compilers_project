use anyhow::{Context, Error, ensure};
use culpa::throws;

#[throws]
fn interpret(src: &str) -> String {
    let tokens: Vec<_> = lexer::Lexer::from(src).collect();
    let (program, parsing_errors) = parser::parse_program(&tokens).expect("Failed to parse");
    ensure!(
        parsing_errors.is_empty(),
        "Parsing errors: {parsing_errors:?}"
    );
    let (program, _) = ast::convert(&program).expect("Failed to typecheck");
    let mut result: Vec<u8> = vec![];
    crate::interpret(&mut result, &program).context("Interpreter error")?;
    String::from_utf8(result).context("Somehow got non-UTF8 output")?
}

testing::tests! {
    folder = "run"
    extension = "stdout"
    fun = interpret
    pass = [
        real_literals => "real_literals"
    ]
    fail = [

    ]
}
