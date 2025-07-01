//! src/parser.rs

// 从我们的 lexer 模块中导入 Token 和 TokenType
use crate::lexer::{Token, TokenType};

// --- AST Node Definitions ---
// 为了方便调试和查看，我们为所有 AST 节点派生 Debug trait。

#[derive(Debug)]
pub struct Program {
    pub function: Function,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Statement,
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
}

/// 代表一个一元运算符的类型
#[derive(Debug)]
pub enum UnaryOperator {
    Negate,     // - (从 Negation 修改)
    Complement, // ~ (从 BitwiseComplement 修改)
    Not,        // ! (从 Bang 修改)
}
#[derive(Debug)]
pub enum BinaryOperator {
    Add,
    Subtract, // 使用完整单词
    Multiply, // 使用完整单词
    Divide,   // 使用完整单词
    Remainder,
    And,            // &&
    Or,             // ||
    Equal,          // == (从 EqualEqual 修改)
    NotEqual,       // != (从 BangEqual 修改)
    LessThan,       // <  (从 Less 修改)
    LessOrEqual,    // <=
    GreaterThan,    // >  (从 Greater 修改)
    GreaterOrEqual, // >=
}

/// 代表一个表达式
#[derive(Debug)]
pub enum Expression {
    Constant(i32),
    // 它包含一个运算符和一个指向内部表达式的智能指针
    Unary {
        operator: UnaryOperator,
        expression: Box<Expression>, // 使用 Box 来处理递归定义
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

// ... (AST 定义之后) ...

pub struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl<'a> Parser<'a> {
    /// 创建一个新的 Parser 实例。
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    /// 解析整个程序，这是公共的入口点。
    pub fn parse(&mut self) -> Result<Program, String> {
        let function = self.parse_function()?;
        // 可以在这里检查是否还有多余的 token，以确保整个输入都被解析了
        if self.position < self.tokens.len() {
            let token = &self.tokens[self.position];
            return Err(format!(
                "Unexpected token {:?} on line {}",
                token.token_type, token.line
            ));
        }
        Ok(Program { function })
    }

    // --- Private Helper Methods ---

    /// 查看当前位置的 token，但不消费它。
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    /// 消费当前 token 并前进到下一个。
    fn consume(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    /// 期望并消费一个特定类型的 token。如果当前 token 不是预期类型，则返回错误。
    fn expect_token(&mut self, expected_type: TokenType) -> Result<&Token, String> {
        match self.peek() {
            Some(token) if token.token_type == expected_type => Ok(self.consume().unwrap()),
            Some(token) => Err(format!(
                "Expected token {:?}, but found {:?} on line {}",
                expected_type, token.token_type, token.line
            )),
            None => Err(format!(
                "Expected token {:?}, but found end of input.",
                expected_type
            )),
        }
    }

    /// 期望并消费一个标识符 token，返回其字符串值。
    fn expect_identifier(&mut self) -> Result<String, String> {
        if let Some(TokenType::Identifier(name)) = self.peek().map(|t| &t.token_type) {
            // 克隆 name
            let name_clone = name.clone();
            self.consume();
            return Ok(name_clone);
        }

        // 如果上面的 if 不匹配，说明 token 类型不对或没有 token
        let found_token = self.peek();
        Err(format!(
            "Expected an identifier, but found {:?} on line {}",
            found_token.map(|t| &t.token_type),
            found_token.map_or(0, |t| t.line)
        ))
    }

    // A more concise version for expect_integer_constant
    #[allow(dead_code)]
    fn expect_integer_constant(&mut self) -> Result<i32, String> {
        if let Some(TokenType::IntegerConstant(value)) = self.peek().map(|t| t.token_type.clone()) {
            // `value` 是 i32，是 Copy 类型，所以这里是复制
            self.consume();
            return Ok(value);
        }

        // 如果上面的 if 不匹配
        let found_token = self.peek();
        Err(format!(
            "Expected an integer constant, but found {:?} on line {}",
            found_token.map(|t| &t.token_type),
            found_token.map_or(0, |t| t.line)
        ))
    }
    // 获取 Token 对应的二元运算符和优先级 ---
    fn get_binary_operator_precedence(token_type: &TokenType) -> Option<(BinaryOperator, u8)> {
        match token_type {
            // 算术运算符
            TokenType::Plus => Some((BinaryOperator::Add, 45)),
            TokenType::Minus => Some((BinaryOperator::Subtract, 45)),
            TokenType::Asterisk => Some((BinaryOperator::Multiply, 50)),
            TokenType::Slash => Some((BinaryOperator::Divide, 50)),
            TokenType::Percent => Some((BinaryOperator::Remainder, 50)),

            // 关系运算符
            TokenType::Less => Some((BinaryOperator::LessThan, 35)),
            TokenType::LessEqual => Some((BinaryOperator::LessOrEqual, 35)),
            TokenType::Greater => Some((BinaryOperator::GreaterThan, 35)),
            TokenType::GreaterEqual => Some((BinaryOperator::GreaterOrEqual, 35)),

            // 相等运算符
            TokenType::Equal => Some((BinaryOperator::Equal, 30)),
            TokenType::NotEqual => Some((BinaryOperator::NotEqual, 30)),

            // 逻辑运算符
            TokenType::And => Some((BinaryOperator::And, 10)), // 修正 Bug (之前是 Add)
            TokenType::Or => Some((BinaryOperator::Or, 5)),    // 修正 Bug (之前是 Add)
            _ => None,
        }
    }

    // --- Recursive Descent Parsing Methods ---

    /// 解析一个函数定义。
    /// <function> ::= "int" <identifier> "(" "void" ")" "{" <statement> "}"
    fn parse_function(&mut self) -> Result<Function, String> {
        self.expect_token(TokenType::KeywordInt)?;
        let name = self.expect_identifier()?;
        self.expect_token(TokenType::OpenParen)?;
        self.expect_token(TokenType::KeywordVoid)?;
        self.expect_token(TokenType::CloseParen)?;
        self.expect_token(TokenType::OpenBrace)?;
        let body = self.parse_statement()?;
        self.expect_token(TokenType::CloseBrace)?;

        Ok(Function { name, body })
    }

    /// 解析一个语句。
    /// <statement> ::= "return" <exp> ";"
    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.expect_token(TokenType::KeywordReturn)?;
        let expression = self.parse_expression(0)?;
        self.expect_token(TokenType::Semicolon)?;

        Ok(Statement::Return(expression))
    }

    /// 【核心】使用优先级爬升法解析表达式。
    /// <exp> ::= <factor> { <binop> <exp> }
    fn parse_expression(&mut self, min_precedence: u8) -> Result<Expression, String> {
        // 表达式的左侧总是一个 factor
        let mut left = self.parse_factor()?;
        // 循环处理后续的 binop + exp
        while let Some(next_token) = self.peek() {
            // 检查下一个 token 是否是优先级足够的二元运算符
            if let Some((op, precedence)) =
                Self::get_binary_operator_precedence(&next_token.token_type)
            {
                if precedence >= min_precedence {
                    // 消费掉这个运算符
                    self.consume();

                    // 递归调用 parse_expression 来解析右侧
                    // 注意：右侧的最低优先级要比当前运算符高 1 (或更高，取决于结合性)
                    // 对于左结合，precedence + 1 是正确的
                    let right = self.parse_expression(precedence + 1)?;

                    // 将左右两边组合成一个新的 left
                    left = Expression::Binary {
                        operator: op,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                } else {
                    // 优先级不够，跳出循环
                    break;
                }
            } else {
                // 不是二元运算符，跳出循环
                break;
            }
        }

        Ok(left)
    }
    /// <factor> ::= <int> | <unop> <factor> | "(" <exp> ")"
    fn parse_factor(&mut self) -> Result<Expression, String> {
        let next_token = self
            .peek()
            .cloned()
            .ok_or_else(|| "Unexpected end of input, expected a factor.".to_string())?;

        match &next_token.token_type {
            // <factor> ::= <int>
            TokenType::IntegerConstant(val) => {
                self.consume();
                Ok(Expression::Constant(*val))
            }
            // <factor> ::= <unop> <factor>
            TokenType::Minus | TokenType::Tilde | TokenType::Not => {
                let operator = self.parse_unary_operator()?;
                // 递归调用 parse_factor，而不是 parse_expression
                let expression = self.parse_factor()?;
                Ok(Expression::Unary {
                    operator,
                    expression: Box::new(expression),
                })
            }
            // <factor> ::= "(" <exp> ")"
            TokenType::OpenParen => {
                self.consume();
                // 括号内部是一个完整的表达式，所以调用 parse_expression
                let inner_expression = self.parse_expression(0)?;
                self.expect_token(TokenType::CloseParen)?;
                Ok(inner_expression)
            }
            _ => Err(format!(
                "Unexpected token {:?}, expected a factor (integer, unary operator, or '(').",
                next_token.token_type
            )),
        }
    }
    /// <unop> ::= "-" | "~" | "!"
    fn parse_unary_operator(&mut self) -> Result<UnaryOperator, String> {
        if let Some(token) = self.consume() {
            match token.token_type {
                TokenType::Minus => Ok(UnaryOperator::Negate), // 修改
                TokenType::Tilde => Ok(UnaryOperator::Complement), // 修改
                TokenType::Not => Ok(UnaryOperator::Not),      // 修改
                _ => Err(format!(
                    "Expected unary operator, but found {:?} on line {}",
                    token.token_type, token.line
                )),
            }
        } else {
            Err("Expected unary operator, but found end of input.".to_string())
        }
    }
}
