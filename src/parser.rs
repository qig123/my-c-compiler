//! src/parser.rs

// 从我们的 lexer 模块中导入 Token 和 TokenType
use crate::lexer::{Token, TokenType};

// --- AST Node Definitions ---
// 这些定义与你之前的版本一致，并且是正确的。
#[derive(Debug, PartialEq)] // 派生 PartialEq 以便测试
pub struct Program {
    pub function: Function,
}

#[derive(Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub body: Vec<BlockItem>,
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
}

// ... (AST 定义之后) ...

pub struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let function = self.parse_function()?;
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

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn consume(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }

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

    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.peek().map(|t| &t.token_type) {
            Some(TokenType::Identifier(name)) => {
                let name_clone = name.clone();
                self.consume();
                Ok(name_clone)
            }
            Some(other_token) => Err(format!(
                "Expected an identifier, but found {:?}",
                other_token
            )),
            None => Err("Expected an identifier, but found end of input.".to_string()),
        }
    }

    // --- 【新增】获取二元运算符的优先级 ---
    // 我们将把这个逻辑移到 parse_expression 内部，但保留这个辅助函数以供参考
    fn get_precedence(token_type: &TokenType) -> u8 {
        match token_type {
            TokenType::Assign => 1,
            TokenType::Or => 5,
            TokenType::And => 10,
            TokenType::Equal | TokenType::NotEqual => 30,
            TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual => 35,
            TokenType::Plus | TokenType::Minus => 45,
            TokenType::Asterisk | TokenType::Slash | TokenType::Percent => 50,
            _ => 0, // 不是二元运算符
        }
    }

    // --- Recursive Descent Parsing Methods ---

    /// 【修改】解析一个函数定义。
    /// <function> ::= "int" <identifier> "(" "void" ")" "{" {<block-item>} "}"
    fn parse_function(&mut self) -> Result<Function, String> {
        self.expect_token(TokenType::KeywordInt)?;
        let name = self.expect_identifier()?;
        self.expect_token(TokenType::OpenParen)?;
        self.expect_token(TokenType::KeywordVoid)?;
        self.expect_token(TokenType::CloseParen)?;
        self.expect_token(TokenType::OpenBrace)?;

        let mut body = Vec::new();
        // 循环解析 block-item，直到遇到 '}'
        while self
            .peek()
            .map_or(false, |t| t.token_type != TokenType::CloseBrace)
        {
            body.push(self.parse_block_item()?);
        }

        self.expect_token(TokenType::CloseBrace)?;

        Ok(Function { name, body })
    }

    /// 【新增】解析一个块项目。
    /// <block-item> ::= <statement> | <declaration>
    fn parse_block_item(&mut self) -> Result<BlockItem, String> {
        // 通过预读第一个 token 来判断是声明还是语句
        if let Some(token) = self.peek() {
            if token.token_type == TokenType::KeywordInt {
                // 'int' 关键字开头，是声明
                let declaration = self.parse_declaration()?;
                Ok(BlockItem::D(declaration))
            } else {
                // 否则，是语句
                let statement = self.parse_statement()?;
                Ok(BlockItem::S(statement))
            }
        } else {
            Err("Expected a statement or declaration, but found end of input.".to_string())
        }
    }

    /// 【新增】解析一个声明。
    /// <declaration> ::= "int" <identifier> ["=" <exp>] ";"
    fn parse_declaration(&mut self) -> Result<Declaration, String> {
        self.expect_token(TokenType::KeywordInt)?;
        let name = self.expect_identifier()?;

        let init;
        // 检查可选的初始化器
        if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Assign)
        {
            self.consume(); // 消费 '='
            init = Some(self.parse_expression(0)?);
        } else {
            init = None;
        }

        self.expect_token(TokenType::Semicolon)?;
        Ok(Declaration { name, init })
    }

    /// 【修改】解析一个语句。
    /// <statement> ::= "return" <exp> ";" | [<exp>] ";"
    fn parse_statement(&mut self) -> Result<Statement, String> {
        if let Some(token) = self.peek() {
            match token.token_type {
                TokenType::KeywordReturn => {
                    self.consume(); // 消费 "return"
                    let exp = self.parse_expression(0)?;
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Return(exp))
                }
                // 【修改】明确处理空语句的情况
                TokenType::Semicolon => {
                    self.consume(); // 消费 ";"
                    Ok(Statement::Empty) // 返回 Empty 变体
                }
                _ => {
                    // 表达式语句：<exp> ;
                    let exp = self.parse_expression(0)?;
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Expression(exp)) // 返回 Expression 变体
                }
            }
        } else {
            Err("Expected a statement, but found end of input.".to_string())
        }
    }

    /// 【核心修改】使用优先级爬升法解析表达式，支持右结合赋值。
    fn parse_expression(&mut self, min_precedence: u8) -> Result<Expression, String> {
        let mut left = self.parse_factor()?;

        while let Some(next_token) = self.peek().cloned() {
            let precedence = Self::get_precedence(&next_token.token_type);
            if precedence == 0 || precedence < min_precedence {
                break; // 不是二元运算符或优先级不够
            }

            // 消费掉这个运算符
            self.consume();

            // 检查结合性
            if next_token.token_type == TokenType::Assign {
                // 右结合
                // 对于右结合运算符，递归调用的 min_precedence 与当前运算符的 precedence 相同
                let right = self.parse_expression(precedence)?;
                left = Expression::Assign {
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                // 左结合
                // 对于左结合运算符，递归调用的 min_precedence 是当前 precedence + 1
                let op = self.token_to_binary_operator(&next_token.token_type)?;
                let right = self.parse_expression(precedence + 1)?;
                left = Expression::Binary {
                    operator: op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            }
        }

        Ok(left)
    }

    /// 【新增】辅助函数，将 TokenType 转换为 BinaryOperator
    fn token_to_binary_operator(&self, token_type: &TokenType) -> Result<BinaryOperator, String> {
        match token_type {
            TokenType::Plus => Ok(BinaryOperator::Add),
            TokenType::Minus => Ok(BinaryOperator::Subtract),
            TokenType::Asterisk => Ok(BinaryOperator::Multiply),
            TokenType::Slash => Ok(BinaryOperator::Divide),
            TokenType::Percent => Ok(BinaryOperator::Remainder),
            TokenType::And => Ok(BinaryOperator::And),
            TokenType::Or => Ok(BinaryOperator::Or),
            TokenType::Equal => Ok(BinaryOperator::Equal),
            TokenType::NotEqual => Ok(BinaryOperator::NotEqual),
            TokenType::Less => Ok(BinaryOperator::LessThan),
            TokenType::LessEqual => Ok(BinaryOperator::LessOrEqual),
            TokenType::Greater => Ok(BinaryOperator::GreaterThan),
            TokenType::GreaterEqual => Ok(BinaryOperator::GreaterOrEqual),
            _ => Err(format!("Not a binary operator token: {:?}", token_type)),
        }
    }

    /// 【修改】解析一个因子。
    /// <factor> ::= <int> | <identifier> | <unop> <factor> | "(" <exp> ")"
    fn parse_factor(&mut self) -> Result<Expression, String> {
        let next_token = self
            .consume()
            .cloned()
            .ok_or_else(|| "Unexpected end of input, expected a factor.".to_string())?;

        match &next_token.token_type {
            // <factor> ::= <int>
            TokenType::IntegerConstant(val) => Ok(Expression::Constant(*val)),

            // <factor> ::= <identifier>
            TokenType::Identifier(name) => Ok(Expression::Var(name.clone())),

            // <factor> ::= <unop> <factor>
            TokenType::Minus | TokenType::Tilde | TokenType::Not => {
                let operator = self.token_to_unary_operator(&next_token.token_type)?;
                let expression = self.parse_factor()?;
                Ok(Expression::Unary {
                    operator,
                    expression: Box::new(expression),
                })
            }
            // <factor> ::= "(" <exp> ")"
            TokenType::OpenParen => {
                let inner_expression = self.parse_expression(0)?;
                self.expect_token(TokenType::CloseParen)?;
                Ok(inner_expression)
            }
            _ => Err(format!(
                "Unexpected token {:?}, expected a factor (integer, identifier, unary operator, or '(').",
                next_token.token_type
            )),
        }
    }

    /// 【修改】解析一元运算符，现在从 token 类型直接转换
    fn token_to_unary_operator(&self, token_type: &TokenType) -> Result<UnaryOperator, String> {
        match token_type {
            TokenType::Minus => Ok(UnaryOperator::Negate),
            TokenType::Tilde => Ok(UnaryOperator::Complement),
            TokenType::Not => Ok(UnaryOperator::Not),
            _ => Err(format!(
                "Expected unary operator, but found {:?}",
                token_type
            )),
        }
    }

    // 原始的 parse_unary_operator 不再需要，因为逻辑已合并到 parse_factor 中
}
