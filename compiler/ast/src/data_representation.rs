use crate::Type;
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
    type_to_id: HashMap<Type, TypeId>,
    current: u32,
}

impl Interner {
    #[must_use]
    pub fn new() -> Self {
        let mut interner = Interner {
            representation_to_id: HashMap::new(),
            type_to_id: HashMap::new(),
            current: 0,
        };

        let id = interner.register_type(&Type::Int).unwrap_or_else(|id| id);
        assert_eq!(id, TypeId(0), "Integer type_id is 0");
        interner.intern_with_id(Representation::IntegerRepresentation, id);

        let id = interner.register_type(&Type::Bool).unwrap_or_else(|id| id);
        assert_eq!(id, TypeId(0), "Booolean type_id is 1");
        interner.intern_with_id(Representation::BooleanRepresentation, id);

        let id = interner.register_type(&Type::Real).unwrap_or_else(|id| id);
        assert_eq!(id, TypeId(0), "Real type_id is 3");
        interner.intern_with_id(Representation::RealRepresentation, id);

        let id = interner.register_type(&Type::Null).unwrap_or_else(|id| id);
        assert_eq!(id, TypeId(0), "Null type_id is 4");
        interner.intern_with_id(Representation::NullRepresentation, id);

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

    pub fn intern_with_id(&mut self, rep: Representation, id: TypeId) {
        match self.representation_to_id.get(&rep) {
            Some(id) => panic!("Internal error : ambitioous interning for {id:?} : {rep:?}"),
            None => {
                let _: Option<TypeId> = self.representation_to_id.insert(rep, id);
            }
        }
    }

    pub fn register_type(&mut self, ty: &Type) -> Result<TypeId, TypeId> {
        match self.type_to_id.get(ty) {
            Some(id) => Ok(*id),
            None => {
                let fresh = self.next_id();
                let _: Option<TypeId> = self.type_to_id.insert(ty.clone(), fresh);
                Err(fresh)
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
