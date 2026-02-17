///  Variable location and id
enum Location {
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

struct TypeId(u32);

enum Bytecode {
    /// push int / bool onto stack    
    IntConst {
        value: i64,
    },
    /// push real onto stack
    RealConst {
        value: f64,
    },
    /// push to stack
    Load {
        loc: Location,
    },
    /// pop from stack                 
    Store {
        loc: Location,
    },
    /// push address to stack
    AddresOf {
        loc: Location,
    },
    /// duplicate stack top                   
    Dup,
    /// drop stack top                                     
    Drop,
    /// apply binary operator to stack top                                   
    BinOp {
        op: BinaryOperator,
    },
    /// pop address from stack, pop and write refernced value               
    StoreAddress,
    /// pop address from stack, read and push refernced value             
    LoadAddress,
    /// allocate a record, push a reference to stack              
    AllocRecord {
        size: u64,
    }, // TODO: add TypeId ?
    /// allocate an array, push a reference to stack    
    AllocArray {
        element_size: u64,
        size: u64,
    }, // TODO: add TypeId ?
    /// pop array ref from stack, push its size
    ArraySize, // TODO: add built-in function call
    /// pop array ref and index from stack, push element               
    GetIndex,
    /// pop record ref from stack, push its field value                  
    GetField {
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
    /// condiditional jump    
    JumpZero {
        label: u64,
    },
    /// conditional jump
    JumpNotZero {
        label: u64,
    },
    /// enter function
    Enter {
        arguments_count: u64,
        local_count: u64,
    },
    /// leave function, the stack top is a return value
    Ret,
    /// call specified function
    Call {
        function_label: u64,
    },
    /// Print a value of spe
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
    fn encode(&self) -> Vec<u8> {
        let mut result = Vec::new();
        self.enocode_inline(&mut result);
        result
    }

    fn enocode_inline(&self, buffer: &mut Vec<u8>) {
        buffer.extend(self.encode())
    }
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
