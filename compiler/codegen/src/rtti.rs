use ast::{ArrayDescription, FieldDescription, RecordDescription, Type};
use common::{Identifier, RawIdentifier};
use core::mem;
use indexmap::IndexMap;
use indexmap::map::Entry;
use std::hash::Hash;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeId(pub usize);

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Representation {
    IntegerRepresentation,
    BooleanRepresentation,
    RealRepresentation,
    NullRepresentation,
    RecordRepresentation(RecordRepresentation),
    ArrayRepresentation(ArrayRepresentation),
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct RecordRepresentation {
    pub fields: Vec<(RawIdentifier, TypeId)>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct ArrayRepresentation {
    pub element: TypeId,
}

#[derive(Debug)]
pub(crate) struct Interner {
    arena: IndexMap<Rc<Type>, Representation>,
}

impl Interner {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            arena: [
                (Type::null(), Representation::NullRepresentation),
                (Type::int(), Representation::IntegerRepresentation),
                (Type::bool(), Representation::BooleanRepresentation),
                (Type::real(), Representation::RealRepresentation),
            ]
            .into_iter()
            .collect(),
        }
    }

    pub(crate) fn intern<'t>(
        &mut self,
        ty: &Rc<Type>,
        resolve_alias: &impl Fn(&Identifier) -> &'t Rc<Type>,
    ) -> TypeId {
        const PLACEHOLDER: Representation = Representation::NullRepresentation;

        if let Type::Alias(ident) = ty.as_ref() {
            return self.intern(resolve_alias(ident), resolve_alias);
        }
        let vacant_entry = match self.arena.entry(Rc::clone(ty)) {
            Entry::Occupied(e) => return TypeId(e.index()),
            Entry::Vacant(e) => e,
        };

        let result = TypeId(vacant_entry.insert_entry(PLACEHOLDER).index());

        let representation = match ty.as_ref() {
            Type::Alias(_) => unreachable!("Aliases are resolved above"),
            Type::Null | Type::Int | Type::Bool | Type::Real => {
                unreachable!("This should have been pre-allocated")
            }
            Type::Record(RecordDescription { fields }) => {
                Representation::RecordRepresentation(RecordRepresentation {
                    fields: fields
                        .iter()
                        .map(|FieldDescription { name, t }| {
                            (name.clone(), self.intern(t, resolve_alias))
                        })
                        .collect(),
                })
            }
            Type::Array(ArrayDescription {
                t: element,
                length: _,
            }) => Representation::ArrayRepresentation(ArrayRepresentation {
                element: self.intern(element, resolve_alias),
            }),
        };

        let prev = mem::replace(&mut self.arena[result.0], representation);
        debug_assert_eq!(prev, PLACEHOLDER, "Duplicate definitions for {result:?}");
        result
    }

    #[must_use]
    pub(crate) fn into_table(self) -> Vec<Representation> {
        let Self { arena } = self;
        arena.into_values().collect()
    }
}
