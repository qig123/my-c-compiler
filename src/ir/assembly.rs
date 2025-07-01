//  src/ir/assembly.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    AX,  // 代表 EAX/RAX
    R10, // 代表 R10D/R10
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Neg, // neg
    Not, // not
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(String), // 伪寄存器，如 "tmp.0"
    Stack(i32),     // 栈地址，如 -4, -8
}

#[derive(Debug)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Unary { op: UnaryOperator, operand: Operand },
    AllocateStack { bytes: u32 },
    Ret,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub struct Program {
    pub function: Function,
}
