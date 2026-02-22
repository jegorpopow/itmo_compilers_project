#![allow(unused, unreachable_pub)]

use core::error::Error;
use std::env;
use std::fs;
use std::process::ExitCode;

use crate::lexer::tokenize;
use crate::tokens::dump_tokens;

mod ast;
mod bytecode;
mod lexer;
mod operators;
mod tokens;
mod types;

// TODO: create a Driver module

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("No file provided");
        return ExitCode::from(1);
    }

    let source: String = fs::read_to_string(&args[1]).unwrap();
    let tokens = tokenize(&source).unwrap();
    println!("{}", dump_tokens(&tokens));
    ExitCode::SUCCESS
}
