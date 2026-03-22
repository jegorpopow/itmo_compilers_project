#![expect(unreachable_pub, reason = "WIP")]

use std::env;
use std::fs;
use std::process::ExitCode;

use crate::ast::convert::convert;
use crate::lexer::Lexer;
use crate::parser::{ParsingError, parse_program};
use crate::tokens::Token;

mod source_positions;

mod identifier;
mod loop_order;
mod operators;

mod bytecode;

mod tokens;

mod ast;
mod parse_tree;

mod lexer;
mod parser;

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

    match parse_program(tokens.as_slice()) {
        Ok((program, errs)) => {
            println!("Following errors occurred:");

            for ParsingError { what, position } in errs {
                println!("\t{what} @ {position}");
            }

            for decl in &program.0 {
                println!("{decl:?}");
            }

            match convert(&program) {
                Ok((program, _)) => {
                    for decl in &program.0 {
                        println!("{decl:?}");
                    }
                }
                Err(err) => {
                    println!("TypeCheck failed:");
                    println!("Error:\n\t{:?}", err.what);
                }
            }
        }
        Err(ParsingError { what, position }) => {
            println!("Error:\n\treason:{what}\n\tposition: {position}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
