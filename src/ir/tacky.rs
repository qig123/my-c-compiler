// src/ir/tacky.rs

//! 定义 TACKY (Three-Address Code, kind of) 中间表示的数据结构。

// 因为 Function 和 UnaryOperator 需要名字和类型，
// 我们需要从 parser 模块导入一些定义。
// 或者，为了解耦，我们可以在 TACKY 中重新定义它们。
// 这里我们选择重新定义，让 IR 模块完全独立。

#[derive(Debug)]
pub enum UnaryOperator {
    Complement, // ~ (ASDL: Complement)
    Negate,     // - (ASDL: Negate)
}

/// TACKY 中的一个值，可以是一个常量或一个临时变量。
/// 对应 ASDL: val = Constant(int) | Var(identifier)
#[derive(Debug, Clone)] // Clone 很方便，因为值经常被复用
pub enum Val {
    Constant(i32),
    Var(String), // Var 用 String 来存储变量名，如 "tmp0", "tmp1"
}

/// TACKY 中的一条指令。
/// 对应 ASDL: instruction = Return(val) | Unary(unary_operator, val src, val dst)
#[derive(Debug)]
pub enum Instruction {
    Return(Val),
    Unary {
        op: UnaryOperator,
        src: Val,
        dst: Val, // 注意：在生成时，dst 必须是一个 Var
    },
}

/// TACKY 中的一个函数定义。
/// 对应 ASDL: function_definition = Function(identifier, instruction* body)
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Vec<Instruction>, // instruction* 表示一个指令列表 (vector)
}

/// TACKY 程序的根节点。
/// 对应 ASDL: program = Program(function_definition)
#[derive(Debug)]
pub struct Program {
    pub function: Function,
}

// 为了方便，我们可以将 Program 重命名为 TackyProgram，
// 但遵循 ASDL，使用 Program 也是完全可以的，因为上下文很清晰 (tacky::Program)。
// 这里我将保持为 Program，与 ASDL 一致。
