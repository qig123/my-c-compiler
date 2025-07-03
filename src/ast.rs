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
