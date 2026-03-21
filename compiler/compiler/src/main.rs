use std::env;
use std::fs;
use std::process::ExitCode;

use ast::convert;
use lexer::{Lexer, Token};
use parser::{ParsingError, parse_program};

mod bytecode;

// TODO: create a Driver module

fn main() -> ExitCode {
    let Some(file) = env::args().nth(1) else {
        println!("No file provided");
        return ExitCode::from(1);
    };

    let Ok(source) = fs::read_to_string(file) else {
        eprintln!("IO error: cannot read from input file");
        return ExitCode::FAILURE;
    };
    let tokens: Vec<Token<'_>> = Lexer::from(source.as_str()).collect();
    for token in &tokens {
        println!("{token}")
    }

    match parse_program(tokens.as_slice()) {
        Ok((program, errs)) => {
            println!("Following errors occurred:");

            for err in errs {
                println!("\t{err}");
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
                    println!("Error:\n\t{err}");
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
