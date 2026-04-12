use std::io::{self, Write};

use lexer::Lexer;
use parser::{Expression, Parser};

fn parse_expr_from_line(str: &str) -> parser::FinalResult<Expression> {
    let mut parser = Parser::new(Lexer::from(str));
    let expr = parser.parse_expr();
    parser.finish(expr)
}

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    loop {
        print!("> ");
        io::stdout().flush()?;

        buffer.clear();
        let _: usize = io::stdin().read_line(&mut buffer)?;

        match parse_expr_from_line(&buffer) {
            Ok(expr) => println!("{expr:?}"),
            Err(err) => println!("error: {err}"),
        }
    }
}
