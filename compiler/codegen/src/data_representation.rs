use common::RawIdentifier;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hash;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeId(pub u32);

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

#[derive(Debug, Default)]
pub(crate) struct Interner {
    // representations : Vec<Representation>,
    representation_to_id: HashMap<Representation, TypeId>,
    current: u32,
}

impl Interner {
    #[must_use]
    pub(crate) fn new() -> Self {
        let mut interner = Interner {
            representation_to_id: HashMap::new(),
            current: 0,
        };

        assert_eq!(
            interner.intern(Representation::IntegerRepresentation),
            TypeId(0),
            "Integer type_id is 0"
        );
        assert_eq!(
            interner.intern(Representation::BooleanRepresentation),
            TypeId(1),
            "Boolean type_id is 1"
        );
        assert_eq!(
            interner.intern(Representation::RealRepresentation),
            TypeId(2),
            "Real type_id is 2"
        );
        assert_eq!(
            interner.intern(Representation::NullRepresentation),
            TypeId(3),
            "Null type_id is 3"
        );

        interner
    }

    pub(crate) fn intern(&mut self, rep: Representation) -> TypeId {
        *match self.representation_to_id.entry(rep) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let id = TypeId(self.current);
                self.current += 1;
                e.insert(id)
            }
        }
    }

    #[must_use]
    pub(crate) fn into_table(self) -> Vec<Representation> {
        let mut result = vec![Representation::NullRepresentation; self.representation_to_id.len()];
        #[expect(
            clippy::iter_over_hash_type,
            reason = "The result is still deterministic"
        )]
        for (rep, id) in self.representation_to_id {
            result[id.0 as usize] = rep
        }

        result
    }
}
