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

#[derive(Debug)]
pub enum Expression {
    Constant(i32),
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
        let expression = self.parse_expression()?;
        self.expect_token(TokenType::Semicolon)?;

        Ok(Statement::Return(expression))
    }

    /// 解析一个表达式。
    /// <exp> ::= <int>
    fn parse_expression(&mut self) -> Result<Expression, String> {
        let value = self.expect_integer_constant()?;
        Ok(Expression::Constant(value))
    }
}
