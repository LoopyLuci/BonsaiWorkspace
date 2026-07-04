use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuirModule {
    pub functions: Vec<BuirFunction>,
    pub types: Vec<BuirType>,
    pub globals: Vec<BuirGlobal>,
    pub language: Language,
    pub source_hash: String,
    pub compiler_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuirGlobal {
    pub name: String,
    pub ty: BuirType,
    pub initializer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuirFunction {
    pub name: String,
    pub signature: BuirType,
    pub body: Option<SsaBody>,
    pub version: u64,
    pub effects: EffectSet,
    pub language: Language,
    pub symbol_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsaBody {
    pub blocks: Vec<BasicBlock>,
    pub parameters: Vec<BuirType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    Add { lhs: Value, rhs: Value, result: Value },
    Sub { lhs: Value, rhs: Value, result: Value },
    Mul { lhs: Value, rhs: Value, result: Value },
    Div { lhs: Value, rhs: Value, result: Value },
    Call { function: String, args: Vec<Value>, result: Value },
    Load { ptr: Value, result: Value },
    Store { ptr: Value, value: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminator {
    Return(Option<Value>),
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EffectSet {
    pub async_: bool,
    pub unsafe_: bool,
    pub io: bool,
    pub alloc: bool,
    pub noreturn: bool,
}

impl Default for EffectSet {
    fn default() -> Self {
        Self {
            async_: false,
            unsafe_: false,
            io: false,
            alloc: false,
            noreturn: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    C,
    Cpp,
    Go,
    Zig,
    Java,
    Kotlin,
    CSharp,
    JavaScript,
    TypeScript,
    Lua,
    Titan,
    Aether,
    Sylva,
    Axiom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuirType {
    Void,
    I32,
    I64,
    F32,
    F64,
    Bool,
    String,
    Pointer(Box<BuirType>),
    Struct {
        name: String,
        fields: Vec<BuirType>,
        version: u64,
    },
    Function {
        params: Vec<BuirType>,
        returns: Box<BuirType>,
    },
    Array(Box<BuirType>, u64),
    List(Box<BuirType>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Value {
    pub id: u32,
    pub ty: BuirType,
}
