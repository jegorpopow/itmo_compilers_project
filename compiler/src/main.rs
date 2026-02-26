#![expect(dead_code, unreachable_pub, reason = "WIP")]

use std::env;
use std::fs;
use std::process::ExitCode;

use crate::lexer::Lexer;

mod ast;
mod bytecode;
mod lexer;
mod operators;
mod tokens;
mod types;

// TODO: create a Driver module

#[expect(clippy::unwrap_used, reason = "WIP")]
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("No file provided");
        return ExitCode::from(1);
    }

    let source: String = fs::read_to_string(&args[1]).unwrap();
    for token in Lexer::from(source.as_str()) {
        println!("{token}")
    }
    ExitCode::SUCCESS
}
