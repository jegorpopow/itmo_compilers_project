use std::env;
use std::fs;
use std::io;

use anyhow::Context;
use ast::convert;
use interpreter::interpret;
use lexer::Lexer;
use parser::parse_program;

// TODO: create a Driver module

fn main() -> anyhow::Result<()> {
    let file = env::args().nth(1).context("No file provided")?;
    let source = fs::read_to_string(file).context("IO error: cannot read from input file")?;

    let tokens: Vec<_> = Lexer::from(source.as_str()).collect();
    for token in &tokens {
        eprintln!("{token}")
    }

    let program = parse_program(tokens).context("Parsing error")?;

    for decl in &program.0 {
        eprintln!("{decl:?}");
    }

    let program = convert(&program).context("Typecheck error")?;

    for decl in &program.globals {
        eprintln!("{decl:?}");
    }

    interpret(io::stdout(), &program).expect("Interpretation failed");

    Ok(())
}
