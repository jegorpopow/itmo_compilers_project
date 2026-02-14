enum Location {
    // Variable location
    Global(usize),
    Local(usize),
    Argument(usize),
}

enum BinaryOperator {
    RealAdd,
    RealSub,
    RealMul,
    RealDiv,
    RealLe,
    RealLg,
    RealGt,
    RealGe,
    RealEq,
    RealNeq,
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntMod,
    IntLe,
    IntLg,
    IntGt,
    IntGe,
    IntEq,
    IntNeq,
    BoolAnd,
    BoolXor,
    BoolOr,
}

enum Bytecode {
    Load { loc: Location },                      // push to stack
    Store { loc: Location },                     // pop from stack
    Dup,                                         // Duplicate stack top,
    Drop,                                        // Drops the stack top
    BinOp { op: BinaryOperator },                // Apply operator to stack top
    StoreAddress,              // pop address from stack, pop and write refernced value
    LoadAddress,               // pop address from stack, read and push refernced value
    AllocRecord { size: u64 }, // Allocate record
    AllocArray { element_size: u64, size: u64 }, // Allocate array
    ArraySize,                 // pop array ref from stack, push its size
    GetIndex,                  // pop array ref and index from stack, push element
    GetField { field_offset: u64 }, // pop record ref from stack, push its field value
    Label { id: u64 },         // No-op
    Jump { label: u64 },       // Non-conditional jump
    JumpZero { label: u64 },   // Condiditional jump
    JumpNotZero { label: u64 }, // Conditional jump
    IntToBool,                 // BoolToInt is no-op, since int and bool have similar reprsentation
    RealToInt,
    IntToReal,
    Call { function_label: u64 },
    Ret, // returns the stack top value from function
}

struct TypeId(u32);

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

trait Encodable {
    fn encode(self: &Self) -> Vec<u8>;
}

impl Encodable for Vec<Bytecode> {
    fn encode(&self) -> Vec<u8> {
        unimplemented!("meow")
    }
}

impl Encodable for RTTI {
    fn encode(&self) -> Vec<u8> {
        unimplemented!("meow")
    }
}

impl Encodable for FunctionTable {
    fn encode(&self) -> Vec<u8> {
        unimplemented!("meow")
    }
}

impl Encodable for Header {
    fn encode(&self) -> Vec<u8> {
        unimplemented!("meow")
    }
}
