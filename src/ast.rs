//src/ast.rs
pub mod unchecked {
    // --- AST Node Definitions ---
    // 这些定义与你之前的版本一致，并且是正确的。
    #[derive(Debug, PartialEq)] // 派生 PartialEq 以便测试
    pub struct Program {
        pub function: Function,
    }

    #[derive(Debug, PartialEq)]
    pub struct Function {
        pub name: String,
        pub body: Block,
    }
    #[derive(Debug, PartialEq)]
    pub struct Block {
        pub blocks: Vec<BlockItem>,
    }

    #[derive(Debug, PartialEq)]
    pub enum BlockItem {
        S(Statement),
        D(Declaration),
    }

    #[derive(Debug, PartialEq)]
    pub struct Declaration {
        // 【修改】字段设为 pub，以便在其他模块（如语义分析）中访问
        pub name: String,
        pub init: Option<Expression>,
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
        // --- 【新增】循环和跳转语句 (无标签) ---
        While {
            condition: Expression,
            body: Box<Statement>,
        },
        DoWhile {
            body: Box<Statement>,
            condition: Expression,
        },
        For {
            init: Option<Box<BlockItem>>, // For 循环的 init 可以是声明或表达式
            condition: Option<Expression>,
            post: Option<Expression>, // 循环后的表达式
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
        pub function: Function,
    }

    #[derive(Debug, PartialEq)]
    pub struct Function {
        pub name: String,
        pub body: Block,
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
        D(super::unchecked::Declaration), // Declaration 不包含 Statement，可以直接复用
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
