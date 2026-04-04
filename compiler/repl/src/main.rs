use std::{
    io::{self, Write},
    rc::Rc,
};

use lexer::Lexer;
use parser::{Expression, IndexedIterator, Parser, ParserResult, TokenIterator as _};

fn parse_expr_from_line(str: &str) -> ParserResult<Rc<Expression>> {
    let tokens: Vec<_> = Lexer::from(str).collect();
    let mut parser = Parser::new();
    let start = IndexedIterator::from(tokens.as_slice());
    let (result, tail) = parser.parse_expr(start)?;
    assert_eq!(tail.current(), None, "Unparsed tokens");
    parser.finish(result)
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
            Err(err) => println!("error: {err}"),
        }
    }
}
