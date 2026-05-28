use std::env;
use std::fs;

use anyhow::Context;
use ast::convert;
use codegen::codegen;
use lexer::Lexer;
use parser::parse_program;

fn main() -> anyhow::Result<()> {
    let file = env::args().nth(1).context("No file provided")?;
    let out = env::args().nth(2).unwrap_or_else(|| {
        let p = std::path::Path::new(&file);
        p.with_extension("obj").to_string_lossy().into_owned()
    });

    let source = fs::read_to_string(&file).context("IO error: cannot read from input file")?;

    let tokens: Vec<_> = Lexer::from(source.as_str()).collect();
    let program = parse_program(tokens).context("Parsing error")?;
    let program = convert(&program).context("Typecheck error")?;

    let bytecode_file = codegen(&program).context("Codegen error")?;
    let bytes = bytecode_file.to_bytes();
    fs::write(&out, bytes).context("IO error: cannot write output file")?;

    Ok(())
}
