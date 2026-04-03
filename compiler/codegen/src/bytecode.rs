#![expect(dead_code, reason = "WIP")]

use core::alloc::Layout;

use common::{
    Integer, Location, Real,
    operators::{
        BoolBinOp, EqBinOp, IntBinOp, RealBinOp, SemanticBinaryOperator, SemanticUnaryOperator,
    },
};

trait Encode {
    type Output: Copy;
    #[must_use]
    fn encode(&self) -> Self::Output;
}

impl Encode for Location {
    type Output = (u8, [u8; 2]);
    fn encode(&self) -> Self::Output {
        match self {
            Self::Global(v) => (0, v.to_le_bytes()),
            Self::Local(v) => (1, v.to_le_bytes()),
            Self::Argument(v) => (2, v.to_le_bytes()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TypeId(pub u32);

#[derive(Debug)]
pub(crate) enum Instruction {
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
    DropMany(usize),
    /// swaps top and second elements of stack
    Swap,
    /// apply binary operator to stack top
    BinOp {
        op: SemanticBinaryOperator,
    },
    UnOp {
        op: SemanticUnaryOperator,
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
    },
    IntToBool, // All of it may be just a built-in call
    RealToInt, // All of it may be just a built-in call
    IntToReal, // All of it may be just a built-in call
}

impl Encode for SemanticBinaryOperator {
    type Output = u8;

    fn encode(&self) -> Self::Output {
        match self {
            SemanticBinaryOperator::Eq(op) => match op {
                EqBinOp::Eq => 0x00,
                EqBinOp::Ne => 0x01,
            },

            SemanticBinaryOperator::Real(op) => match op {
                RealBinOp::Le => 0x10,
                RealBinOp::Lt => 0x11,
                RealBinOp::Gt => 0x12,
                RealBinOp::Ge => 0x13,

                RealBinOp::Add => 0x14,
                RealBinOp::Sub => 0x15,
                RealBinOp::Mul => 0x16,
                RealBinOp::Div => 0x17,
            },

            SemanticBinaryOperator::Int(op) => match op {
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

            SemanticBinaryOperator::Bool(op) => match op {
                BoolBinOp::And => 0x30,
                BoolBinOp::Or => 0x31,
                BoolBinOp::Xor => 0x32,
            },
        }
    }
}

#[expect(clippy::too_many_lines, reason = "Cause that lint is really stupid")]
impl Encode for Instruction {
    type Output = Bytecode;
    fn encode(&self) -> Bytecode {
        let zero = Bytecode::default();
        match self {
            Instruction::Drop => Bytecode { opcode: 1, ..zero },
            Instruction::Dup => Bytecode { opcode: 2, ..zero },
            Instruction::Swap => Bytecode { opcode: 3, ..zero },

            Instruction::BinOp { op } => Bytecode {
                opcode: 4,
                subopcode: op.encode(),
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
                arg64: n.to_le_bytes(),
                ..zero
            },

            Instruction::Load { loc } => {
                let (subopcode, arg16) = loc.encode();
                Bytecode {
                    opcode: 11,
                    subopcode,
                    arg16,
                    ..zero
                }
            }
            Instruction::Store { loc } => {
                let (subopcode, arg16) = loc.encode();
                Bytecode {
                    opcode: 12,
                    subopcode,
                    arg16,
                    ..zero
                }
            }
            Instruction::AddressOf { loc } => {
                let (subopcode, arg16) = loc.encode();
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
            Instruction::Panic { code } => Bytecode {
                opcode: 27,
                arg64: code.to_le_bytes(),
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

fn into_halves<const H: usize, const N: usize>(s: &mut [u8; N]) -> [&mut [u8; H]; 2] {
    const { assert!(2 * H == N, "H should be exactly half of N") }
    let ([first_half, second_half], []) = s.as_chunks_mut::<H>() else {
        unreachable!()
    };
    [first_half, second_half]
}

impl Encode for Bytecode {
    type Output = [u8; 16];
    fn encode(&self) -> Self::Output {
        // TODO: all of this is just a convoluted memcpy, lmao
        debug_assert_eq!(
            Layout::new::<Self>(),
            Layout::new::<Self::Output>(),
            "this isn't a memcpy anymore"
        );

        let mut result = [0u8; 16];
        let [h, s64to128] = into_halves::<8, _>(&mut result);
        let [h, s32to64] = into_halves::<4, _>(h);
        let [h, s16to32] = into_halves::<2, _>(h);
        let [[s0to8], [s8to16]] = into_halves::<1, _>(h);
        let Bytecode {
            opcode,
            subopcode,
            arg16,
            arg32,
            arg64,
        } = *self;
        *s0to8 = opcode;
        *s8to16 = subopcode;
        *s16to32 = arg16;
        *s32to64 = arg32;
        *s64to128 = arg64;
        result
    }
}

struct RecordRTTI {
    id: TypeId,
    field_ids: Vec<TypeId>,
}

struct ArrayRTTI {
    id: TypeId,
    element_id: TypeId,
}

struct PrimitiveRTTI {
    id: TypeId,
}

enum RTTIElement {
    Record(RecordRTTI),
    Array(ArrayRTTI),
    Primitive(PrimitiveRTTI),
}

struct RTTI(Vec<RTTIElement>);

struct FunctionRecord {
    name: String,
    label_id: i64,
    args: Vec<TypeId>,
    result: TypeId,
}

struct FunctionTable(Vec<FunctionRecord>);

struct MemorySpan {
    offset: u32,
    length: u32,
}

struct Header {
    magic: u32,
    version: u32,
    code_span: MemorySpan,
    function_table_span: MemorySpan,
    rtti_span: MemorySpan,
    function_count: u32,
    global_count: u32,
}
