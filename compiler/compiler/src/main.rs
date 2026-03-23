use std::env;
use std::fs;

use anyhow::Context;
use ast::convert;
use lexer::Lexer;
use parser::parse_program;

mod bytecode;

// TODO: create a Driver module

fn main() -> anyhow::Result<()> {
    let file = env::args().nth(1).context("No file provided")?;
    let source = fs::read_to_string(file).context("IO error: cannot read from input file")?;

    let tokens: Vec<_> = Lexer::from(source.as_str()).collect();
    for token in &tokens {
        println!("{token}")
    }

    let (program, errs) = parse_program(tokens.as_slice()).context("Parsing error")?;

    if !errs.is_empty() {
        eprintln!("Following errors occurred:");
        for err in errs {
            eprintln!("\t{err}");
        }
    }

    for decl in &program.0 {
        println!("{decl:?}");
    }

    let (program, _identifiers) = convert(&program).context("Typecheck error")?;

    for decl in program.0 {
        println!("{decl:?}");
    }

    Ok(())
}
