// src/ir/tacky.rs

//! 定义 TACKY (Three-Address Code, kind of) 中间表示的数据结构。

#[derive(Debug, Clone, Copy)] // Copy is possible since enums are simple
pub enum UnaryOperator {
    Complement, // ~ (ASDL: Complement)
    Negate,     // - (ASDL: Negate)
    Not,        // ! (ASDL: Not)  <-- 修改点
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,        // == (ASDL: Equal) <-- 修改点
    NotEqual,     // != (ASDL: NotEqual) <-- 修改点
    LessThan,     // < (ASDL: LessThan)
    LessOrEqual,  // <= (ASDL: LessOrEqual)
    GreaterThan,  // > (ASDL: GreaterThan)
    GreaterEqual, // >= (ASDL: GreaterOrEqual) <-- 拼写修正
}
// 注意：上面的 BinaryOperator 我也改成了 LessThan/LessOrEqual/GreaterThan，
// 这样更具描述性，但你用 Less/LessEqual/Greater 也可以，只要保持一致即可。
// ASDL 使用的是全名，所以我这里也用了全名。

/// TACKY 中的一个值，可以是一个常量或一个临时变量。
/// 对应 ASDL: val = Constant(int) | Var(identifier)
#[derive(Debug, Clone)]
pub enum Val {
    Constant(i32),
    Var(String), // Var 用 String 来存储变量名，如 "tmp0", "tmp1"
}

/// TACKY 中的一条指令。
/// 对应 ASDL: instruction = ...
#[derive(Debug)]
pub enum Instruction {
    Return(Val),
    Unary {
        op: UnaryOperator,
        src: Val,
        dst: Val,
    },
    Binary {
        op: BinaryOperator,
        src1: Val,
        src2: Val,
        dst: Val,
    },
    Copy {
        src: Val,
        dst: Val,
    },
    Jump(String),
    JumpIfZero {
        condition: Val,
        target: String,
    },
    JumpIfNotZero {
        condition: Val,
        target: String,
    },
    Label(String),
    FunCall {
        name: String,
        args: Vec<Val>,
        dst: Val,
    },
}

/// TACKY 中的一个函数定义。
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Instruction>,
}

/// TACKY 程序的根节点。s
#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}
