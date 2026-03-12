use core::fmt;

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    #[must_use] 
    pub fn begin() -> Self {
        Position { line: 1, column: 0 }
    }

    #[must_use]
    pub fn advance(&self, is_newline: bool) -> Self {
        if is_newline {
            Position {
                line: self.line + 1,
                column: 0,
            }
        } else {
            Position {
                line: self.line,
                column: self.column + 1,
            }
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { line, column } = self;
        write!(f, "{line}:{column}")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Extent {
    pub start: Position,
    pub end: Position,
}

impl fmt::Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { start, end } = self;
        write!(f, "{start}-{end}")
    }
}
