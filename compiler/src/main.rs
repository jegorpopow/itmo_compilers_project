#![expect(unreachable_pub, reason = "WIP")]

use std::env;
use std::fs;
use std::process::ExitCode;

use crate::lexer::Lexer;
use crate::parser::{ParsingError, parse_programm};
use crate::tokens::Token;

mod source_positions;

mod ast;
mod identifier;
mod lexer;
mod operators;
mod parse_tree;
mod parser;
mod tokens;

mod bytecode;

// TODO: create a Driver module

#[expect(clippy::unwrap_used, reason = "WIP")]
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("No file provided");
        return ExitCode::from(1);
    }

    let source: String = fs::read_to_string(&args[1]).unwrap();
    let tokens: Vec<Token<'_>> = Lexer::from(source.as_str()).collect();
    for token in &tokens {
        println!("{token}")
    }

    match parse_programm(tokens.as_slice()) {
        Ok((decls, _)) => {
            for decl in decls {
                println!("{:?}", decl)
            }
        }
        Err(ParsingError { what, position }) => {
            println!("Error:\n\treason:{what}\n\tposition: {position}")
        }
    }

    ExitCode::SUCCESS
}
