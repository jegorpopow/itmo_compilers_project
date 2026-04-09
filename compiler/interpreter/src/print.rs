use std::io::{Error, Write};

use culpa::throws;

use crate::{Heap, Object, Place, Places, Primitive, Reference, Value};

#[throws]
#[expect(private_bounds, reason = "implementation details")]
pub(super) fn print<'a>(
    mut out: impl Write,
    heap: &Heap<'a>,
    places: &Places<'a>,
    arg: &impl Printable<'a>,
) {
    arg.print(
        &mut Context {
            out: &mut out,
            heap,
            places,
        },
        Recursion::Start,
    )?;
    writeln!(out)?
}

// We have Haskell-like lists at home.
#[derive(Clone, Copy)]
enum Recursion<'a, 's> {
    Start,
    Node {
        address: Reference<'a>,
        parent: &'s Self,
    },
}

struct Context<'a, 'r, W: Write> {
    out: W,
    heap: &'r Heap<'a>,
    places: &'r Places<'a>,
}

impl<'a> Recursion<'a, '_> {
    fn find(self, target: Reference<'a>) -> Option<usize> {
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
    fn print<W: Write>(&self, ctx: &mut Context<'a, '_, W>, parent: Recursion<'a, '_>);
}

impl<'a> Printable<'a> for Reference<'a> {
    #[throws]
    fn print<W: Write>(&self, ctx: &mut Context<'a, '_, W>, parent: Recursion<'a, '_>) {
        match parent.find(*self) {
            Some(depth) => write!(
                ctx.out,
                "/* repeated {depth} level{} above */",
                if depth == 1 { "" } else { "s" }
            )?,
            None => ctx.heap[*self].print(
                ctx,
                Recursion::Node {
                    address: *self,
                    parent: &parent,
                },
            )?,
        }
    }
}

impl<'a> Printable<'a> for Place<'a> {
    #[throws]
    fn print<W: Write>(&self, ctx: &mut Context<'a, '_, W>, parent: Recursion<'a, '_>) {
        ctx.places[*self].print(ctx, parent)?
    }
}

impl Printable<'_> for Primitive {
    #[throws]
    fn print<W: Write>(&self, ctx: &mut Context<'_, '_, W>, _: Recursion<'_, '_>) {
        let out = &mut ctx.out;
        match self {
            Self::Bool(value) => write!(out, "{value}")?,
            Self::Integer(value) => write!(out, "{value}")?,
            &Self::Real(value) => {
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
            Self::Null => write!(out, "null")?,
        }
    }
}

impl<'a> Printable<'a> for Value<'a> {
    #[throws]
    fn print<W: Write>(&self, ctx: &mut Context<'a, '_, W>, parent: Recursion<'a, '_>) {
        match self {
            Self::Primitive(primitive) => primitive.print(ctx, parent)?,
            Self::Reference(address) => address.print(ctx, parent)?,
        }
    }
}

impl<'a> Printable<'a> for Object<'a> {
    #[throws]
    fn print<W: Write>(&self, ctx: &mut Context<'a, '_, W>, parent: Recursion<'a, '_>) {
        match self {
            Self::Array { elements } => {
                write!(ctx.out, "[ ")?;
                for elem in elements {
                    elem.print(ctx, parent)?;
                    write!(ctx.out, ", ")?
                }
                write!(ctx.out, "]")?
            }
            Self::Struct { fields } => {
                write!(ctx.out, "{{ ")?;

                let mut fields: Vec<_> = fields
                    .iter()
                    .map(|(&ident, &address)| (ident, address))
                    .collect();
                fields.sort_unstable_by_key(|&(key, _value)| key);

                for (name, place) in fields {
                    let name = name.name.as_str();
                    if name.contains('\'') {
                        // This seems to be the only way that we have to get an identifier
                        // that is not an identifier in ECMA (https://262.ecma-international.org/5.1/#sec-7.6).
                        // And looks like Rust's `Debug for str` will be enough here to do all the escaping.
                        write!(ctx.out, "{name:?}")?
                    } else {
                        write!(ctx.out, "{name}")?
                    }
                    write!(ctx.out, ": ")?;
                    place.print(ctx, parent)?;
                    write!(ctx.out, ", ")?;
                }

                write!(ctx.out, "}}")?
            }
        }
    }
}
