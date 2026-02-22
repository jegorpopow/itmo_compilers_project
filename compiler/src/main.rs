#![allow(unused, unreachable_pub)]

use::std::env;
use std::fs;
use std::error::Error;
use std::process::ExitCode;

use crate::lexer::tokenise;
use crate::tokens::dump_tokens;

mod ast;
mod bytecode;
mod tokens;
mod types;
mod lexer;
mod operators;


// TODO: create a Driver module

fn main() -> ExitCode {
    let args  : Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("No file provided");
        return ExitCode::from(1);
    }

    let source : String = fs::read_to_string(args[1].clone()).unwrap();
    let tokens = tokenise(&source).unwrap();
    println!("{}", dump_tokens(&tokens));
    return ExitCode::SUCCESS
}
