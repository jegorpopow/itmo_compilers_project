use common::RawIdentifier;
use std::collections::HashMap;
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
pub struct Interner {
    // representations : Vec<Representation>,
    representation_to_id: HashMap<Representation, TypeId>,
    current: u32,
}

impl Interner {
    #[must_use]
    pub fn new() -> Self {
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

    fn next_id(&mut self) -> TypeId {
        self.current += 1;
        TypeId(self.current - 1)
    }

    pub fn intern(&mut self, rep: Representation) -> TypeId {
        match self.representation_to_id.get(&rep) {
            Some(id) => *id,
            None => {
                let fresh = self.next_id();
                let _: Option<TypeId> = self.representation_to_id.insert(rep, fresh);
                fresh
            }
        }
    }

    #[must_use]
    #[expect(clippy::iter_over_hash_type, reason = "Why no?")]
    pub fn to_table(self) -> Vec<Representation> {
        let mut result = vec![Representation::NullRepresentation; self.representation_to_id.len()];

        for (rep, id) in self.representation_to_id {
            result[id.0 as usize] = rep
        }

        result
    }
}
