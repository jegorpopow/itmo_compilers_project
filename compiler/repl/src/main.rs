use std::{
    io::{self, Write},
    rc::Rc,
};

use lexer::{Lexer, Token};
use parser::{Expression, IndexedIterator, Parser, PureParsingResult};

fn parse_expr_from_line(str: &str) -> PureParsingResult<Rc<Expression>> {
    let tokens: Vec<Token<'_>> = Lexer::from(str).collect();
    let mut parser = Parser::new();
    let start = IndexedIterator::from(tokens.as_slice());
    parser.parse_expr(start).map(|(val, _)| val)
}

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    loop {
        print!("> ");
        io::stdout().flush()?;

        buffer.clear();
        let _: usize = io::stdin().read_line(&mut buffer)?;

        match parse_expr_from_line(&buffer) {
            Ok(expr) => println!("{:?}", *expr),
            Err(err) => println!("IO error: {} @ {}", err.what, err.position),
        }
    }
}
