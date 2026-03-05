#[allow(unused_results)]
use std::{
    io::{self, Write},
    rc::Rc,
};

use compiler::{
    lexer::Lexer,
    parse_tree::tree::Expression,
    parser::{IndexedIterator, Parser, PureParsingResult},
    tokens::Token,
};

fn parse_expr_from_line(str: String) -> PureParsingResult<Rc<Expression>> {
    let tokens: Vec<Token<'_>> = Lexer::from(str.as_str()).collect();
    let mut parser = Parser::new();
    let start = IndexedIterator::from(tokens.as_slice());
    parser.parse_expr(start).map(|(val, _)| val)
}

fn main() {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        let _ = io::stdin().read_line(&mut buffer).expect("IO Error");

        match parse_expr_from_line(buffer) {
            Ok(expr) => println!("{:?}", *expr),
            Err(err) => println!("IO error: {} @ {}", err.what, err.position),
        }
    }
}
