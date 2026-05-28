use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use ast::convert;
use bytecode::Serialize;
use codegen::compile;
use lexer::Lexer;
use parser::parse_program;

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let file = args.next().context("No file provided")?;
    let out = match args.next() {
        None => Path::new(&file).with_extension("obj"),
        Some(out) => out.into(),
    };
    drop(args);

    let source = fs::read_to_string(&file).context("IO error: cannot read from input file")?;

    let tokens: Vec<_> = Lexer::from(source.as_str()).collect();
    let program = parse_program(tokens).context("Parsing error")?;
    let program = convert(&program).context("Typecheck error")?;
    let program = compile(&program).context("Codegen error")?;
    File::create(out)
        .and_then(|mut out| program.serialize(&mut |bytes| out.write_all(bytes)))
        .context("IO error: cannot write output file")?;
    Ok(())
}
