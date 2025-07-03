//src/ast.rs
pub mod unchecked {
    // Program 现在包含一个声明列表
    #[derive(Debug, PartialEq)]
    pub struct Program {
        pub declarations: Vec<Declaration>,
    }

    // Declaration 枚举现在是顶层项目之一
    #[derive(Debug, PartialEq)]
    pub enum Declaration {
        // 函数声明/定义
        Function {
            name: String,
            params: Vec<String>, // 参数列表
            body: Option<Block>, // Option<Block> 可以区分声明和定义
        },
        // 变量声明 (用于全局变量)
        Variable {
            name: String,
            init: Option<Expression>,
        },
    }
    // Block 和 BlockItem 的定义是正确的
    #[derive(Debug, PartialEq)]
    pub struct Block {
        pub blocks: Vec<BlockItem>,
    }

    #[derive(Debug, PartialEq)]
    pub enum BlockItem {
        S(Statement),
        D(Declaration),
    }

    // ForInit 的表示方式 (Option<Box<BlockItem>>) 是正确的，无需修改 Statement
    #[derive(Debug, PartialEq)]
    pub enum Statement {
        Return(Expression),
        Expression(Expression),
        Empty, // 对应 Null statement
        If {
            condition: Expression,
            then_stat: Box<Statement>,
            else_stat: Option<Box<Statement>>,
        },
        Compound(Block),
        While {
            condition: Expression,
            body: Box<Statement>,
        },
        DoWhile {
            body: Box<Statement>,
            condition: Expression,
        },
        For {
            init: Option<Box<BlockItem>>,
            condition: Option<Expression>,
            post: Option<Expression>,
            body: Box<Statement>,
        },
        Break,
        Continue,
    }

    #[derive(Debug, PartialEq)]
    pub enum UnaryOperator {
        Negate,
        Complement,
        Not,
    }

    #[derive(Debug, PartialEq)]
    pub enum BinaryOperator {
        Add,
        Subtract,
        Multiply,
        Divide,
        Remainder,
        And,
        Or,
        Equal,
        NotEqual,
        LessThan,
        LessOrEqual,
        GreaterThan,
        GreaterOrEqual,
    }

    #[derive(Debug, PartialEq)]
    pub enum Expression {
        Constant(i32),
        Unary {
            operator: UnaryOperator,
            expression: Box<Expression>,
        },
        Binary {
            operator: BinaryOperator,
            left: Box<Expression>,
            right: Box<Expression>,
        },
        Var(String),
        Assign {
            left: Box<Expression>,
            right: Box<Expression>,
        },
        Conditional {
            condition: Box<Expression>,
            left: Box<Expression>,
            right: Box<Expression>,
        },
        FunctionCall {
            name: String,
            args: Vec<Expression>,
        },
    }
}

// src/ast.rs

// ... pub mod unchecked { ... } ...

// 我建议将 CheckAst 改名为更具描述性的 checked
pub mod checked {
    // 我们可以复用很多 unchecked 中的类型
    // 注意：我们需要一个转换函数，所以我们不能直接复用所有类型。
    // 我们需要创建新的 checked 版本的 struct 和 enum。
    // 为了避免名称冲突，我们可以用 `use ... as ...` 或在模块内定义。

    // 定义循环 ID 的类型
    pub type LoopId = usize;

    // Expression 和 Operator 可以直接复用，因为它们不包含 Statement
    // 为了简单起见，我们可以在这里重新声明它们，或者在转换时处理
    pub use super::unchecked::{BinaryOperator, Expression, UnaryOperator};

    #[derive(Debug, PartialEq)]
    pub struct Program {
        // Program 现在也包含一个声明列表
        pub declarations: Vec<Declaration>,
    }

    #[derive(Debug, PartialEq)]
    pub enum Declaration {
        Function {
            name: String,
            params: Vec<String>,
            // 函数体是 checked::Block
            body: Option<Block>,
        },
        Variable {
            name: String,
            // 注意：init 表达式也应该是 checked 的，
            // 但因为 Expression 没有子 Statement，所以可以直接复用
            init: Option<Expression>,
        },
    }

    // --- 【核心变化】---
    // 所有包含 Statement 的结构都需要一个 checked 版本

    #[derive(Debug, PartialEq)]
    pub struct Block {
        pub blocks: Vec<BlockItem>,
    }

    #[derive(Debug, PartialEq)]
    pub enum BlockItem {
        S(Statement),
        // 局部变量声明
        D(Declaration),
    }

    #[derive(Debug, PartialEq)]
    pub enum Statement {
        Return(Expression),
        Expression(Expression),
        Empty,
        If {
            condition: Expression,
            then_stat: Box<Statement>,
            else_stat: Option<Box<Statement>>,
        },
        Compound(Block),

        // --- 【带标签的循环和跳转】 ---
        While {
            condition: Expression,
            body: Box<Statement>,
            id: LoopId, // 新增 ID
        },
        DoWhile {
            body: Box<Statement>,
            condition: Expression,
            id: LoopId, // 新增 ID
        },
        For {
            // 注意：init 部分也需要是 checked 版本
            init: Option<Box<BlockItem>>,
            condition: Option<Expression>,
            post: Option<Expression>,
            body: Box<Statement>,
            id: LoopId, // 新增 ID
        },
        Break {
            target_id: LoopId, // 指向目标循环
        },
        Continue {
            target_id: LoopId, // 指向目标循环
        },
    }
}
