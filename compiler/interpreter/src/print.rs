use std::io::{Error, Write};

use culpa::throws;

use crate::{Address, Memory, Value};

#[throws]
#[expect(private_bounds, reason = "implementation details")]
pub(super) fn print<'a>(out: &mut impl Write, heap: &Memory<'a>, arg: &impl Printable<'a>) {
    arg.print(out, heap, Recursion::Start)?;
    writeln!(out)?
}

// We have Haskell-like lists at home.
#[derive(Clone, Copy)]
enum Recursion<'a, 's> {
    Start,
    Node {
        address: Address<'a>,
        parent: &'s Self,
    },
}

impl<'a> Recursion<'a, '_> {
    fn find(self, target: Address<'a>) -> Option<usize> {
        let mut depth = 1;
        let mut current = self;
        while let Self::Node { parent, address } = current {
            if address == target {
                return Some(depth);
            }
            current = *parent;
            depth += 1;
        }
        None
    }
}

trait Printable<'a> {
    #[throws]
    fn print(&self, out: &mut impl Write, heap: &Memory<'a>, recursion: Recursion<'a, '_>);
}

impl<'a> Printable<'a> for Address<'a> {
    #[throws]
    fn print(&self, out: &mut impl Write, heap: &Memory<'a>, recursion: Recursion<'a, '_>) {
        match recursion.find(*self) {
            Some(depth) => write!(
                out,
                "/* repeated {depth} level{} above */",
                if depth == 1 { "" } else { "s" }
            )?,
            None => heap[*self].print(
                out,
                heap,
                Recursion::Node {
                    address: *self,
                    parent: &recursion,
                },
            )?,
        }
    }
}

impl<'a> Printable<'a> for Value<'a> {
    #[throws]
    fn print(&self, out: &mut impl Write, heap: &Memory<'a>, recursion: Recursion<'a, '_>) {
        match self {
            Value::Bool(value) => write!(out, "{value}")?,
            Value::Integer(value) => write!(out, "{value}")?,
            &Value::Real(value) => {
                if value.is_nan() {
                    write!(out, "NaN")?
                } else if value.is_infinite() {
                    write!(
                        out,
                        "{}Infinity",
                        if value.is_sign_positive() { '+' } else { '-' }
                    )?
                } else if value.fract() == 0.0 {
                    write!(out, "{value:?}")?
                } else {
                    write!(out, "{value}")?
                }
            }
            Value::Null => write!(out, "null")?,
            Value::Array { elements } => {
                write!(out, "[ ")?;
                for &address in elements {
                    address.print(out, heap, recursion)?;
                    write!(out, ", ")?
                }
                write!(out, "]")?
            }
            Value::Struct { fields } => {
                write!(out, "{{ ")?;

                let mut fields: Vec<_> = fields
                    .iter()
                    .map(|(&ident, &address)| (ident, address))
                    .collect();
                fields.sort_unstable_by_key(|&(key, _value)| key);

                for (name, address) in fields {
                    let name = name.name.as_str();
                    if name.contains('\'') {
                        // This seems to be the only way that we have to get an identifier
                        // that is not an identifier in ECMA (https://262.ecma-international.org/5.1/#sec-7.6).
                        // And looks like Rust's `Debug for str` will be enough here to do all the escaping.
                        write!(out, "{name:?}")?
                    } else {
                        write!(out, "{name}")?
                    }
                    write!(out, ": ")?;
                    address.print(out, heap, recursion)?;
                    write!(out, ", ")?;
                }

                write!(out, "}}")?
            }
        }
    }
}
