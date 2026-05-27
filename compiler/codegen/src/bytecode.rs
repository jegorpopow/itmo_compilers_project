use ast::{
    ArrayRepresentation, BinaryOperator, BoolBinOp, EqBinOp, IntBinOp, RealBinOp,
    RecordRepresentation, Representation, TypeId, UnaryOperator,
};

use common::{Integer, Location, RawIdentifier, Real, VarLoc};

trait ToByteCode {
    type Output: Copy;
    #[must_use]
    fn to_bytecode(&self) -> Self::Output;
}

impl ToByteCode for Location {
    type Output = (u8, [u8; 2]);
    fn to_bytecode(&self) -> Self::Output {
        match self {
            Self::Global(v) => (0, v.to_le_bytes()),
            Self::Local(v) => (1, v.to_le_bytes()),
            Self::Argument(v) => (2, v.to_le_bytes()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    /// push int / bool onto stack
    IntConst {
        value: Integer,
    },
    /// push real onto stack
    RealConst {
        value: Real,
    },
    /// push null onto stack
    NullConst,
    /// push to stack
    Load {
        loc: Location,
    },
    /// pop from stack
    Store {
        loc: Location,
    },
    /// push address of variable to stack
    AddressOf {
        loc: Location,
    },
    /// duplicate stack top
    Dup,
    /// drop stack top
    Drop,
    // drop n elements from stack
    DropMany(VarLoc),
    /// swaps top and second elements of stack
    Swap,
    /// apply binary operator to stack top
    BinOp {
        op: BinaryOperator,
    },
    UnOp {
        op: UnaryOperator,
    },
    /// pop value and address from stack, write referenced value
    /// FIXME(Andrew Vlasenkov): ensure pop's order is correct in VM:
    /// value should be on top of a stack and address just below value
    StoreAddress,
    /// pop address from stack, read and push referenced value
    LoadAddress,
    /// allocate a record, push a reference to stack
    AllocRecord {
        type_id: TypeId,
        size: u64,
    },
    /// allocate an array, push a reference to stack
    AllocArray {
        type_id: TypeId,
        size: u64,
    }, // TODO: add TypeId ?
    AllocArrayDynamic {
        type_id: TypeId,
    },
    /// pop array ref from stack, push its size
    ArraySize, // TODO: add built-in function call
    /// pop element index and array ref from stack, push address of array[index]
    ElementAddress,
    /// pop record ref from stack, push its field address
    FieldAddress {
        field_offset: u64,
    },
    /// no-op
    Label {
        id: u64,
    },
    /// non-conditional jump
    Jump {
        label: u64,
    },
    /// conditional jump
    JumpZero {
        label: u64,
    },
    /// conditional jump
    JumpNotZero {
        label: u64,
    },
    /// leave function, the stack top is a return value
    Ret,
    /// call specified function
    Call {
        function_label: u64,
    },
    /// Print a stack top and drop it
    Print {
        type_id: TypeId,
    },
    /// Terminate program
    Panic {
        code: u64,
        line: u32,
        column: u16,
    },
    IntToBool, // All of it may be just a built-in call
    RealToInt, // All of it may be just a built-in call
    IntToReal, // All of it may be just a built-in call
}

impl ToByteCode for BinaryOperator {
    type Output = u8;

    fn to_bytecode(&self) -> Self::Output {
        match self {
            BinaryOperator::Eq(op) => match op {
                EqBinOp::Eq => 0x00,
                EqBinOp::Ne => 0x01,
            },

            BinaryOperator::Real(op) => match op {
                RealBinOp::Le => 0x10,
                RealBinOp::Lt => 0x11,
                RealBinOp::Gt => 0x12,
                RealBinOp::Ge => 0x13,

                RealBinOp::Add => 0x14,
                RealBinOp::Sub => 0x15,
                RealBinOp::Mul => 0x16,
                RealBinOp::Div => 0x17,
            },

            BinaryOperator::Int(op) => match op {
                IntBinOp::Le => 0x20,
                IntBinOp::Lt => 0x21,
                IntBinOp::Gt => 0x22,
                IntBinOp::Ge => 0x23,
                IntBinOp::Add => 0x24,
                IntBinOp::Sub => 0x25,
                IntBinOp::Mul => 0x26,
                IntBinOp::Div => 0x27,
                IntBinOp::Mod => 0x28,
            },

            BinaryOperator::Bool(op) => match op {
                BoolBinOp::And => 0x30,
                BoolBinOp::Or => 0x31,
                BoolBinOp::Xor => 0x32,
            },
        }
    }
}

#[expect(clippy::too_many_lines, reason = "Cause that lint is really stupid")]
impl ToByteCode for Instruction {
    type Output = Bytecode;
    fn to_bytecode(&self) -> Bytecode {
        let zero = Bytecode::default();
        match self {
            Instruction::Drop => Bytecode { opcode: 1, ..zero },
            Instruction::Dup => Bytecode { opcode: 2, ..zero },
            Instruction::Swap => Bytecode { opcode: 3, ..zero },

            Instruction::BinOp { op } => Bytecode {
                opcode: 4,
                subopcode: op.to_bytecode(),
                ..zero
            },
            Instruction::UnOp { op } => Bytecode {
                opcode: 5,
                subopcode: *op as u8,
                ..zero
            },

            Instruction::IntToBool => Bytecode { opcode: 6, ..zero },
            Instruction::RealToInt => Bytecode { opcode: 7, ..zero },
            Instruction::IntToReal => Bytecode { opcode: 8, ..zero },

            Instruction::IntConst { value } => Bytecode {
                opcode: 9,
                arg64: value.to_le_bytes(),
                ..zero
            },
            Instruction::RealConst { value } => Bytecode {
                opcode: 10,
                arg64: value.to_le_bytes(),
                ..zero
            },

            Instruction::NullConst => Bytecode { opcode: 28, ..zero },

            Instruction::DropMany(n) => Bytecode {
                opcode: 29,
                arg16: n.to_le_bytes(),
                ..zero
            },

            Instruction::Load { loc } => {
                let (subopcode, arg16) = loc.to_bytecode();
                Bytecode {
                    opcode: 11,
                    subopcode,
                    arg16,
                    ..zero
                }
            }
            Instruction::Store { loc } => {
                let (subopcode, arg16) = loc.to_bytecode();
                Bytecode {
                    opcode: 12,
                    subopcode,
                    arg16,
                    ..zero
                }
            }
            Instruction::AddressOf { loc } => {
                let (subopcode, arg16) = loc.to_bytecode();
                Bytecode {
                    opcode: 13,
                    subopcode,
                    arg16,
                    ..zero
                }
            }
            Instruction::StoreAddress => Bytecode { opcode: 14, ..zero },
            Instruction::LoadAddress => Bytecode { opcode: 15, ..zero },
            Instruction::AllocRecord {
                type_id: TypeId(type_id),
                size,
            } => Bytecode {
                opcode: 16,
                arg32: type_id.to_le_bytes(),
                arg64: size.to_le_bytes(),
                ..zero
            },
            Instruction::AllocArray {
                type_id: TypeId(type_id),
                size,
            } => Bytecode {
                opcode: 17,
                arg32: type_id.to_le_bytes(),
                arg64: size.to_le_bytes(),
                ..zero
            },
            Instruction::AllocArrayDynamic {
                type_id: TypeId(type_id),
            } => Bytecode {
                opcode: 30,
                arg32: type_id.to_le_bytes(),
                ..zero
            },
            Instruction::ArraySize => Bytecode { opcode: 18, ..zero },
            Instruction::ElementAddress => Bytecode { opcode: 19, ..zero },
            Instruction::FieldAddress { field_offset } => Bytecode {
                opcode: 20,
                arg64: field_offset.to_le_bytes(),
                ..zero
            },

            Instruction::Label { id } => Bytecode {
                opcode: 21,
                arg64: id.to_le_bytes(),
                ..zero
            },
            Instruction::Jump { label } => Bytecode {
                opcode: 22,
                arg64: label.to_le_bytes(),
                ..zero
            },
            Instruction::JumpZero { label } => Bytecode {
                opcode: 23,
                subopcode: 0,
                arg64: label.to_le_bytes(),
                ..zero
            },
            Instruction::JumpNotZero { label } => Bytecode {
                opcode: 23,
                subopcode: 1,
                arg64: label.to_le_bytes(),
                ..zero
            },

            Instruction::Call { function_label } => Bytecode {
                opcode: 24,
                arg64: function_label.to_le_bytes(),
                ..zero
            },
            Instruction::Ret => Bytecode { opcode: 25, ..zero },

            Instruction::Print {
                type_id: TypeId(type_id),
            } => Bytecode {
                opcode: 26,
                arg32: type_id.to_le_bytes(),
                ..zero
            },
            Instruction::Panic { code, line, column } => Bytecode {
                opcode: 27,
                arg64: code.to_le_bytes(),
                arg32: line.to_le_bytes(),
                arg16: column.to_le_bytes(),
                ..zero
            },
        }
    }
}

#[derive(Default, Clone, Copy)]
#[repr(C, align(8))]
struct Bytecode {
    opcode: u8,
    subopcode: u8,
    arg16: [u8; 2],
    arg32: [u8; 4],
    arg64: [u8; 8],
}

type BytecodeBytes = [u8; size_of::<Bytecode>()];

#[expect(clippy::tests_outside_test_module, reason = "simplicity")]
#[test]
fn bytecode_as_bytes() {
    use core::alloc::Layout;

    let src = Layout::new::<Bytecode>();
    let dst = Layout::new::<BytecodeBytes>();
    assert_eq!(src.size(), dst.size(), "size mismatch");
    assert!(src.align() >= dst.align(), "alignment mismatch")
}

impl Bytecode {
    #[must_use]
    const fn as_bytes(&self) -> &BytecodeBytes {
        let result = core::ptr::from_ref(self).cast::<BytecodeBytes>();
        // SAFETY: we are `repr(C)` with no niches
        unsafe { result.as_ref_unchecked() }
    }
}

#[derive(Debug)]
pub(crate) struct RTTI(pub Vec<Representation>);

#[derive(Debug)]
pub struct FunctionRecord {
    pub name: String,
    pub label_id: u64,
    pub args: Vec<TypeId>,
    pub result: TypeId,
}

#[derive(Debug)]
pub(crate) struct FunctionTable(pub Vec<FunctionRecord>);

#[derive(Debug)]
pub struct BytecodeFile {
    pub(crate) code: Vec<Instruction>,
    pub(crate) rtti: RTTI,
    pub(crate) function_table: FunctionTable,
    pub(crate) global_count: usize,
}

pub trait Serialize {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E>;
}

macro_rules! serialize_as_le_bytes {
    {$($t:ty),+} => {
        $(impl Serialize for $t {
            fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
                sink(&self.to_le_bytes())
            }
        })+
    };
}

serialize_as_le_bytes! { u8, u32, u64 }

impl Serialize for usize {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        u32::try_from(*self)
            .expect("usize too big to serialize")
            .serialize(sink)
    }
}

impl<T: Serialize> Serialize for [T] {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        self.len().serialize(sink)?;
        for item in self {
            item.serialize(sink)?
        }
        Ok(())
    }
}

impl Serialize for str {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        self.len().serialize(sink)?;
        sink(self.as_bytes())
    }
}

impl<T: Serialize, U: Serialize> Serialize for (T, U) {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        let (t, u) = self;
        t.serialize(sink)?;
        u.serialize(sink)?;
        Ok(())
    }
}

macro_rules! serialize_as_inner {
    {$($t:ty),+} => {
        $(impl Serialize for $t {
            fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
                let Self(inner) = self;
                inner.serialize(sink)
            }
        })+
    };
}

serialize_as_inner! { RTTI, FunctionTable, TypeId }

macro_rules! serialize_fields {
    {$( $t:ty { $( $field:ident, )+ }, )+} => {
        $(impl Serialize for $t {
            fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
                let Self { $($field,)+ } = self;
                $($field.serialize(sink)?;)+
                Ok(())
            }
        })+
    };
}

serialize_fields! {
    ArrayRepresentation { element, },
    RecordRepresentation { fields, },
    RawIdentifier { name, },
    FunctionRecord {
        name,
        label_id,
        args,
        result,
    },
}

impl Serialize for BytecodeFile {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        const MAGIC: u32 = 0x494D_564D;
        const VERSION: u32 = 2;

        MAGIC.serialize(sink)?;
        VERSION.serialize(sink)?;

        let Self {
            code,
            rtti,
            function_table,
            global_count,
        } = self;
        code.serialize(sink)?;
        rtti.serialize(sink)?;
        function_table.serialize(sink)?;
        global_count.serialize(sink)?;
        Ok(())
    }
}

impl Serialize for Instruction {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        self.to_bytecode().serialize(sink)
    }
}

impl Serialize for Bytecode {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        sink(self.as_bytes())
    }
}

impl Serialize for Representation {
    fn serialize<E>(&self, sink: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        match self {
            Self::IntegerRepresentation => sink(&[0, 0]),
            Self::BooleanRepresentation => sink(&[0, 1]),
            Self::RealRepresentation => sink(&[0, 2]),
            Self::NullRepresentation => sink(&[0, 3]),
            Self::RecordRepresentation(repr) => {
                sink(&[1])?;
                repr.serialize(sink)
            }
            Self::ArrayRepresentation(repr) => {
                sink(&[2])?;
                repr.serialize(sink)
            }
        }
    }
}
