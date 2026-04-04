use core::fmt::{self, Debug, Display};

#[derive(Debug, Clone, Copy)]
pub enum LoopOrder {
    Direct,
    Reversed,
}

#[derive(Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct RawIdentifier {
    pub name: String,
}

impl Debug for RawIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r\"{}\"", self.name)
    }
}

impl Display for RawIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct BindingId(pub usize);

impl Display for BindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(id) = self;
        Display::fmt(id, f)
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Identifier {
    pub raw: RawIdentifier,
    pub id: BindingId,
}

impl Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.raw.name, self.id)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw.name)
    }
}

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

impl Display for Position {
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

impl Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { start, end } = self;
        write!(f, "{start}-{end}")
    }
}

pub type VarLoc = u16;

/// Variable location and id
#[derive(Debug, Clone, Copy, Hash)]
pub enum Location {
    Global(VarLoc),
    Local(VarLoc),
    Argument(VarLoc),
}

pub type Integer = i64;
pub type Real = f64;

#[expect(clippy::inline_always, reason = "an `as` cast")]
#[inline(always)]
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "By design")]
pub const fn integer_to_real(i: Integer) -> Real {
    i as Real
}

#[expect(clippy::inline_always, reason = "an `as` cast")]
#[inline(always)]
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "By design")]
pub const fn real_to_integer(i: Real) -> Integer {
    i as Integer
}
