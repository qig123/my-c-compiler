// src/ir/assembly.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    AX,
    CX,
    DX,
    DI,
    SI,
    R8,
    R9,
    R10,
    R11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
}

// 【新增】条件码，用于 JmpCC 和 SetCC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondCode {
    E,  // Equal
    NE, // Not Equal
    G,  // Greater
    GE, // Greater or Equal
    L,  // Less
    LE, // Less or Equal
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(String),
    Stack(i32),
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        op: UnaryOperator,
        operand: Operand,
    },
    Binary {
        op: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    // 【新增】比较指令
    Cmp {
        src1: Operand,
        src2: Operand,
    },
    // 【新增】IDIV 指令现在是独立的
    Idiv(Operand),
    Cdq,
    Ret,
    // 【新增】跳转和标签指令
    Jmp(String),              // 无条件跳转
    JmpCC(CondCode, String),  // 条件跳转
    SetCC(CondCode, Operand), // 条件置位
    Label(String),            // 标签定义
    AllocateStack {
        bytes: u32,
    }, // 这个从PASS 3移动到这里更合适
    DeallocateStack(u32),
    Push(Operand),
    Call(String),
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}
