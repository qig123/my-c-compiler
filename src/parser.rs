//! src/parser.rs

// 从我们的 lexer 模块中导入 Token 和 TokenType
use crate::{
    ast::unchecked::*,
    lexer::{Token, TokenType},
};

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

    // ===================================================================
    //  1. 公共 API 与顶层解析 (Public API & Top-Level Parsing)
    // ===================================================================
    //  语法层级: program -> declaration -> (function | variable)
    // ===================================================================

    /// 【主入口】解析整个 token 流，生成一个程序（Program）。
    /// <program> ::= {<declaration>}
    pub fn parse(&mut self) -> Result<Program, String> {
        let mut declarations = Vec::new();
        // 循环解析顶层声明，直到 token 流结束
        while self.peek().is_some() {
            declarations.push(self.parse_declaration()?);
        }
        Ok(Program { declarations })
    }

    /// 解析一个声明（函数或变量）。
    /// <declaration> ::= "int" <identifier> ( "(" ... | "=" ... | ";" )
    fn parse_declaration(&mut self) -> Result<Declaration, String> {
        self.expect_token(TokenType::KeywordInt)?;
        let name = self.expect_identifier()?;

        // 通过预读下一个 token 来区分是变量还是函数
        if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::OpenParen)
        {
            // 下一个是 '(', 这是一个函数声明
            self.parse_function_declaration(name)
        } else {
            // 否则，这是一个变量声明
            self.parse_variable_declaration(name)
        }
    }

    /// 解析一个函数声明 (已经消费了 "int" 和 identifier)。
    /// <function-declaration> ::= "(" <param-list> ")" ( <block> | ";" )
    fn parse_function_declaration(&mut self, name: String) -> Result<Declaration, String> {
        self.expect_token(TokenType::OpenParen)?;
        let params = self.parse_param_list()?;
        self.expect_token(TokenType::CloseParen)?;

        // 函数声明后面可以是函数体 '{...}' 或一个分号 ';' (函数原型)
        let body = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::OpenBrace)
        {
            Some(self.parse_block()?)
        } else {
            self.expect_token(TokenType::Semicolon)?;
            None
        };

        Ok(Declaration::Function { name, params, body })
    }

    /// 解析一个变量声明 (已经消费了 "int" 和 identifier)。
    /// <variable-declaration> ::= [ "=" <expression> ] ";"
    fn parse_variable_declaration(&mut self, name: String) -> Result<Declaration, String> {
        let init = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Assign)
        {
            self.consume(); // 消费 '='
            Some(self.parse_expression(0)?)
        } else {
            None
        };
        self.expect_token(TokenType::Semicolon)?;
        Ok(Declaration::Variable { name, init })
    }

    // ===================================================================
    //  2. 语句与代码块解析 (Statement & Block Parsing)
    // ===================================================================
    //  语法层级: block -> block_item -> statement -> (if, for, while...)
    // ===================================================================

    /// 解析一个由花括号包裹的代码块。
    /// <block> ::= "{" {<block-item>} "}"
    fn parse_block(&mut self) -> Result<Block, String> {
        self.expect_token(TokenType::OpenBrace)?;
        let mut items = Vec::new();
        while self
            .peek()
            .map_or(false, |t| t.token_type != TokenType::CloseBrace)
        {
            items.push(self.parse_block_item()?);
        }
        self.expect_token(TokenType::CloseBrace)?;
        Ok(Block { blocks: items })
    }

    /// 解析代码块中的一项（可以是声明或语句）。
    /// <block-item> ::= <statement> | <declaration>
    fn parse_block_item(&mut self) -> Result<BlockItem, String> {
        if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::KeywordInt)
        {
            // 'int' 关键字开头，必定是声明
            self.parse_declaration().map(BlockItem::D)
        } else {
            // 否则，是语句
            self.parse_statement().map(BlockItem::S)
        }
    }

    /// 解析一个语句。
    /// <statement> ::= <if-stmt> | <for-stmt> | <while-stmt> | <do-while-stmt>
    ///               | <return-stmt> | <break-stmt> | <continue-stmt>
    ///               | <block> | [<expression>] ";"
    fn parse_statement(&mut self) -> Result<Statement, String> {
        if let Some(token) = self.peek() {
            match token.token_type {
                TokenType::KeywordIf => self.parse_if_statement(),
                TokenType::KeywordFor => self.parse_for_statement(),
                TokenType::KeywordWhile => self.parse_while_statement(),
                TokenType::KeywordDo => self.parse_do_while_statement(),
                TokenType::KeywordReturn => {
                    self.consume(); // 消费 "return"
                    let exp = self.parse_expression(0)?;
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Return(exp))
                }
                TokenType::KeywordBreak => {
                    self.consume(); // 消费 "break"
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Break)
                }
                TokenType::KeywordContinue => {
                    self.consume(); // 消费 "continue"
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Continue)
                }
                TokenType::OpenBrace => self.parse_block().map(Statement::Compound),
                TokenType::Semicolon => {
                    self.consume(); // 消费 ";"
                    Ok(Statement::Empty)
                }
                _ => {
                    // 表达式语句
                    let exp = self.parse_expression(0)?;
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Expression(exp))
                }
            }
        } else {
            Err("Expected a statement, but found end of input.".to_string())
        }
    }

    /// 解析 if 语句。
    /// <if-stmt> ::= "if" "(" <expression> ")" <statement> [ "else" <statement> ]
    fn parse_if_statement(&mut self) -> Result<Statement, String> {
        self.expect_token(TokenType::KeywordIf)?;
        self.expect_token(TokenType::OpenParen)?;
        let condition = self.parse_expression(0)?;
        self.expect_token(TokenType::CloseParen)?;
        let then_stat = Box::new(self.parse_statement()?);

        let else_stat = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::KeywordElse)
        {
            self.consume(); // 消费 "else"
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            then_stat,
            else_stat,
        })
    }

    /// 解析 for 语句。
    /// <for-stmt> ::= "for" "(" ( <declaration> | [<expression>] ";" ) [<expression>] ";" [<expression>] ")" <statement>
    fn parse_for_statement(&mut self) -> Result<Statement, String> {
        self.expect_token(TokenType::KeywordFor)?;
        self.expect_token(TokenType::OpenParen)?;

        // 解析初始化部分
        let init = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::KeywordInt)
        {
            // for (int i = 0; ... )
            let decl = self.parse_declaration()?;
            // for 循环的初始化器中不允许函数声明
            if let Declaration::Function { .. } = &decl {
                return Err(
                    "Function declarations are not permitted in for loop initializers.".to_string(),
                );
            }
            Some(Box::new(BlockItem::D(decl)))
        } else if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Semicolon)
        {
            // for ( ; ... )
            self.consume(); // 消费 ";"
            None
        } else {
            // for (i = 0; ... )
            let expr = self.parse_expression(0)?;
            self.expect_token(TokenType::Semicolon)?;
            Some(Box::new(BlockItem::S(Statement::Expression(expr))))
        };

        // 解析条件部分
        let condition = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Semicolon)
        {
            None // for (...; ; ...)
        } else {
            Some(self.parse_expression(0)?)
        };
        self.expect_token(TokenType::Semicolon)?;

        // 解析迭代表达式部分
        let post = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::CloseParen)
        {
            None // for (...; ...; )
        } else {
            Some(self.parse_expression(0)?)
        };
        self.expect_token(TokenType::CloseParen)?;

        // 解析循环体
        let body = Box::new(self.parse_statement()?);

        Ok(Statement::For {
            init,
            condition,
            post,
            body,
        })
    }

    /// 解析 while 语句。
    /// <while-stmt> ::= "while" "(" <expression> ")" <statement>
    fn parse_while_statement(&mut self) -> Result<Statement, String> {
        self.expect_token(TokenType::KeywordWhile)?;
        self.expect_token(TokenType::OpenParen)?;
        let condition = self.parse_expression(0)?;
        self.expect_token(TokenType::CloseParen)?;
        let body = Box::new(self.parse_statement()?);
        Ok(Statement::While { condition, body })
    }

    /// 解析 do-while 语句。
    /// <do-while-stmt> ::= "do" <statement> "while" "(" <expression> ")" ";"
    fn parse_do_while_statement(&mut self) -> Result<Statement, String> {
        self.expect_token(TokenType::KeywordDo)?;
        let body = Box::new(self.parse_statement()?);
        self.expect_token(TokenType::KeywordWhile)?;
        self.expect_token(TokenType::OpenParen)?;
        let condition = self.parse_expression(0)?;
        self.expect_token(TokenType::CloseParen)?;
        self.expect_token(TokenType::Semicolon)?; // do-while 结尾必须有分号
        Ok(Statement::DoWhile { condition, body })
    }

    // ===================================================================
    //  3. 表达式解析 (Expression Parsing via Precedence Climbing)
    // ===================================================================
    //  语法层级: expression -> factor
    //  这是解析器的核心，使用优先级爬升法来处理不同优先级的二元运算符。
    // ===================================================================

    /// 使用“优先级爬升法”解析表达式。
    /// <expression> ::= <factor> { <binop> <expression> } | <assignment> | <conditional>
    fn parse_expression(&mut self, min_precedence: u8) -> Result<Expression, String> {
        let mut left = self.parse_factor()?;

        while let Some(next_token) = self.peek().cloned() {
            let precedence = Self::get_precedence(&next_token.token_type);

            // 如果下一个 token 不是运算符，或者其优先级低于当前最低优先级，则停止
            if precedence == 0 || precedence < min_precedence {
                break;
            }

            self.consume(); // 消费该运算符

            // 处理特殊的三元运算符 ?: (右结合)
            if next_token.token_type == TokenType::QuestionMark {
                let then_branch = self.parse_expression(0)?; // then 分支优先级重置
                self.expect_token(TokenType::Colon)?;
                // else 分支的右结合处理，递归时传入当前运算符的优先级
                let else_branch = self.parse_expression(precedence)?;
                left = Expression::Conditional {
                    condition: Box::new(left),
                    left: Box::new(then_branch),
                    right: Box::new(else_branch),
                };
                continue; // 继续循环，处理可能的更高优先级运算符
            }

            // 处理赋值运算符 = (右结合)
            let right = if next_token.token_type == TokenType::Assign {
                // 对于右结合运算符，递归调用的 min_precedence 与当前运算符的 precedence 相同
                self.parse_expression(precedence)?
            } else {
                // 对于左结合运算符，递归调用的 min_precedence 是当前 precedence + 1
                self.parse_expression(precedence + 1)?
            };

            // 构建 AST 节点
            if next_token.token_type == TokenType::Assign {
                left = Expression::Assign {
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                let op = self.token_to_binary_operator(&next_token.token_type)?;
                left = Expression::Binary {
                    operator: op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            }
        }

        Ok(left)
    }

    // ===================================================================
    //  4. 原子项与辅助解析 (Factors & Parsing Helpers)
    // ===================================================================
    //  语法层级: factor -> <int> | <identifier> | <func-call> | <unary-op> | ( <exp> )
    // ===================================================================

    /// 解析一个“因子”，即表达式中的最小单元。
    /// <factor> ::= <int> | <identifier> [ "(" <arg-list> ")" ] | <unop> <factor> | "(" <expression> ")"
    fn parse_factor(&mut self) -> Result<Expression, String> {
        let next_token = self
            .peek()
            .cloned()
            .ok_or_else(|| "Unexpected end of input, expected a factor.".to_string())?;

        match &next_token.token_type {
            TokenType::IntegerConstant(val) => {
                self.consume();
                Ok(Expression::Constant(*val))
            }
            TokenType::Identifier(name) => {
                // 需要预读一个 token 来判断是变量还是函数调用
                if self
                    .tokens
                    .get(self.position + 1)
                    .map_or(false, |t| t.token_type == TokenType::OpenParen)
                {
                    // 是函数调用
                    self.consume(); // 消费 identifier
                    self.consume(); // 消费 '('
                    let args = self.parse_argument_list()?;
                    self.expect_token(TokenType::CloseParen)?;
                    Ok(Expression::FunctionCall {
                        name: name.clone(),
                        args,
                    })
                } else {
                    // 是变量
                    self.consume();
                    Ok(Expression::Var(name.clone()))
                }
            }
            // 一元运算符
            TokenType::Minus | TokenType::Tilde | TokenType::Not => {
                self.consume();
                let operator = self.token_to_unary_operator(&next_token.token_type)?;
                // 一元运算符有最高优先级，因此直接递归解析其后的因子
                let expression = self.parse_factor()?;
                Ok(Expression::Unary {
                    operator,
                    expression: Box::new(expression),
                })
            }
            // 括号表达式
            TokenType::OpenParen => {
                self.consume(); // 消费 '('
                let inner_expression = self.parse_expression(0)?; // 括号内表达式优先级重置为0
                self.expect_token(TokenType::CloseParen)?;
                Ok(inner_expression)
            }
            _ => Err(format!(
                "Unexpected token {:?}, expected a factor.",
                next_token.token_type
            )),
        }
    }

    /// 解析函数参数列表 (声明时使用)。
    /// <param-list> ::= "void" | [ "int" <identifier> { "," "int" <identifier> } ]
    fn parse_param_list(&mut self) -> Result<Vec<String>, String> {
        if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::KeywordVoid)
        {
            self.consume(); // 消费 "void"
            if self
                .peek()
                .map_or(true, |t| t.token_type != TokenType::CloseParen)
            {
                return Err("Expected ')' after 'void' in parameter list.".to_string());
            }
            return Ok(Vec::new());
        }

        if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::CloseParen)
        {
            return Ok(Vec::new()); // 空参数列表
        }

        let mut params = Vec::new();
        // 第一个参数
        self.expect_token(TokenType::KeywordInt)?;
        params.push(self.expect_identifier()?);
        // 后续参数
        while self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Comma)
        {
            self.consume(); // 消费 ','
            self.expect_token(TokenType::KeywordInt)?;
            params.push(self.expect_identifier()?);
        }

        Ok(params)
    }

    /// 解析函数实参列表 (调用时使用)。
    /// <argument-list> ::= [ <expression> { "," <expression> } ]
    fn parse_argument_list(&mut self) -> Result<Vec<Expression>, String> {
        if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::CloseParen)
        {
            return Ok(Vec::new()); // 空参数列表
        }

        let mut args = Vec::new();
        // 第一个参数
        args.push(self.parse_expression(0)?);
        // 后续参数
        while self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Comma)
        {
            self.consume(); // 消费 ','
            args.push(self.parse_expression(0)?);
        }

        Ok(args)
    }

    // ===================================================================
    //  5. 底层工具函数 (Low-Level Utilities)
    // ===================================================================

    /// 查看当前位置的 token，但不消费它。
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    /// 消费并返回当前位置的 token，然后将位置向前移动一位。
    fn consume(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    /// 期望当前 token 是指定类型，如果是则消费它并返回，否则返回错误。
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

    /// 期望当前 token 是一个标识符，如果是则消费它并返回其名称，否则返回错误。
    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.peek().map(|t| &t.token_type) {
            Some(TokenType::Identifier(name)) => {
                let name_clone = name.clone();
                self.consume();
                Ok(name_clone)
            }
            Some(other) => Err(format!("Expected an identifier, but found {:?}", other)),
            None => Err("Expected an identifier, but found end of input.".to_string()),
        }
    }

    /// 获取一个二元运算符的优先级。
    fn get_precedence(token_type: &TokenType) -> u8 {
        match token_type {
            TokenType::Assign => 1,       // 右结合
            TokenType::QuestionMark => 3, // 右结合 (三元)
            TokenType::Or => 5,
            TokenType::And => 10,
            TokenType::Equal | TokenType::NotEqual => 30,
            TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual => 35,
            TokenType::Plus | TokenType::Minus => 45,
            TokenType::Asterisk | TokenType::Slash | TokenType::Percent => 50,
            _ => 0, // 0 表示不是二元运算符或不参与优先级比较
        }
    }

    /// 将 TokenType 转换为 BinaryOperator。
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

    /// 将 TokenType 转换为 UnaryOperator。
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
}

// src/parser.rs -> tests 模块
#[cfg(test)]
mod tests {
    use super::*; // 导入父模块（也就是你的 parser）的所有内容
    use crate::lexer::Lexer;

    // --- 【重构】测试复合语句 ---
    #[test]
    fn test_compound_statement() {
        let source_code = r#"
            int main(void) {
                int a = 1;
                {
                    int b = 2;
                    return b;
                }
                return a;
            }
        "#;
        println!("--- Testing Compound Statement ---");
        println!("Source:\n{}", source_code);

        // 1. & 2. 词法分析和语法分析
        let tokens: Vec<Token> = Lexer::new(source_code).collect::<Result<_, _>>().unwrap();
        let program = Parser::new(&tokens).parse().expect("Parsing failed");

        // 3. 断言 AST 结构

        // 程序应该只包含一个顶层声明，即 main 函数
        assert_eq!(
            program.declarations.len(),
            1,
            "Program should contain one declaration"
        );

        // 获取 main 函数的声明
        if let Declaration::Function {
            name,
            body: Some(main_body),
            ..
        } = &program.declarations[0]
        {
            assert_eq!(name, "main", "Function name should be 'main'");

            // main 函数体应该有 3 个块项目
            assert_eq!(
                main_body.blocks.len(),
                3,
                "Function body should have 3 block items"
            );

            // 第一个项目：int a = 1; (这是一个变量声明)
            let first_item = &main_body.blocks[0];
            assert!(matches!(
                first_item,
                BlockItem::D(Declaration::Variable { .. })
            ));

            // 第二个项目：{...} (这是一个复合语句)
            let second_item = &main_body.blocks[1];
            if let BlockItem::S(Statement::Compound(inner_block)) = second_item {
                assert_eq!(
                    inner_block.blocks.len(),
                    2,
                    "Inner block should have 2 items"
                );

                // 复合语句的第一个项目：int b = 2;
                assert!(matches!(
                    &inner_block.blocks[0],
                    BlockItem::D(Declaration::Variable { .. })
                ));

                // 复合语句的第二个项目：return b;
                assert!(matches!(
                    &inner_block.blocks[1],
                    BlockItem::S(Statement::Return(_))
                ));
            } else {
                panic!(
                    "Second item should be a Compound statement. Got: {:?}",
                    second_item
                );
            }

            // 第三个项目：return a;
            let third_item = &main_body.blocks[2];
            assert!(matches!(third_item, BlockItem::S(Statement::Return(_))));
        } else {
            panic!(
                "Expected a function definition for 'main'. Got: {:?}",
                program.declarations[0]
            );
        }

        println!("\n--- Compound Statement Test Passed! ---");
    }

    // --- 【重构】测试所有循环和跳转语句 ---
    #[test]
    fn test_parsing_of_all_loop_and_jump_statements() {
        let source_code = r#"
            int main(void) {
                for (int i = 0; i < 10; i = i + 1) {
                    while (1) {
                        do {
                            if (i == 5)
                                continue;
                            break;
                        } while (i < 8);
                    }
                }
                return 0;
            }
        "#;
        println!("\n--- Testing All Loop and Jump Statements ---");
        println!("Source:\n{}", source_code);

        // 1. & 2. 词法分析和语法分析
        let tokens: Vec<Token> = Lexer::new(source_code).collect::<Result<_, _>>().unwrap();
        let program = Parser::new(&tokens).parse().expect("Parsing failed");
        println!("--- Successfully Parsed AST ---\n{:#?}", program);

        // 3. 断言 AST 结构
        assert_eq!(
            program.declarations.len(),
            1,
            "Program should contain one declaration"
        );

        if let Declaration::Function {
            body: Some(main_body),
            ..
        } = &program.declarations[0]
        {
            // main 函数体应该包含 `for` 循环和 `return 0;`
            assert_eq!(
                main_body.blocks.len(),
                2,
                "Function body should have a for-loop and a return statement"
            );

            // --- 断言 `for` 循环 ---
            // 注意：现在 for 循环是 BlockItem::S(Statement::For { ... })
            if let BlockItem::S(Statement::For { body, .. }) = &main_body.blocks[0] {
                // (为了简洁，我们只深入检查 body 部分，其他部分在之前的测试已覆盖)

                // --- 断言 `for` 循环体内的 `while` 循环 ---
                if let Statement::Compound(for_body_block) = &**body {
                    if let BlockItem::S(Statement::While {
                        body: while_body, ..
                    }) = &for_body_block.blocks[0]
                    {
                        // --- 断言 `while` 循环体内的 `do-while` 循环 ---
                        if let Statement::Compound(while_body_block) = &**while_body {
                            if let BlockItem::S(Statement::DoWhile {
                                body: do_while_body,
                                ..
                            }) = &while_body_block.blocks[0]
                            {
                                // --- 断言 `do-while` 循环体内的 `if` 和 `break` ---
                                if let Statement::Compound(do_while_body_block) = &**do_while_body {
                                    assert_eq!(
                                        do_while_body_block.blocks.len(),
                                        2,
                                        "Do-while body should contain an if statement and a break statement"
                                    );

                                    // 断言 if (i == 5) continue;
                                    let if_stmt = &do_while_body_block.blocks[0];
                                    assert!(
                                        matches!(if_stmt, BlockItem::S(Statement::If { then_stat, .. }) if matches!(**then_stat, Statement::Continue))
                                    );

                                    // 断言 break;
                                    let break_stmt = &do_while_body_block.blocks[1];
                                    assert!(matches!(break_stmt, BlockItem::S(Statement::Break)));
                                } else {
                                    panic!("Do-while body is not a compound statement");
                                }
                            } else {
                                panic!("Statement in while body is not a do-while loop");
                            }
                        } else {
                            panic!("While body is not a compound statement");
                        }
                    } else {
                        panic!("Statement in for body is not a while loop");
                    }
                } else {
                    panic!("For loop body is not a compound statement");
                }
            } else {
                panic!(
                    "First statement in function is not a for loop. Got: {:?}",
                    main_body.blocks[0]
                );
            }

            // --- 断言最后的 return 语句 ---
            let last_item = &main_body.blocks[1];
            assert!(matches!(last_item, BlockItem::S(Statement::Return(_))));
        } else {
            panic!("Expected a function definition");
        }

        println!("\n--- All Loop and Jump Statements Test Passed! ---");
    }

    // --- 【新增】一个测试来验证函数声明和函数调用 ---
    #[test]
    fn test_function_declaration_and_call() {
        let source_code = r#"
            int add(int a, int b); 

            int main(void) {
                return add(2, 3); 
            }

            int add(int x, int y) {
                return x + y;
            }
        "#;
        println!("\n--- Testing Function Declaration and Call ---");
        println!("Source:\n{}", source_code);

        let tokens: Vec<Token> = Lexer::new(source_code).collect::<Result<_, _>>().unwrap();
        let program = Parser::new(&tokens).parse().expect("Parsing failed");

        // 断言顶层有 3 个声明
        assert_eq!(
            program.declarations.len(),
            3,
            "Program should have 3 top-level declarations"
        );

        // 1. `add` 的原型声明
        if let Declaration::Function {
            name,
            params,
            body: None,
        } = &program.declarations[0]
        {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
        } else {
            panic!("Expected a function prototype for 'add'.");
        }

        // 2. `main` 的定义
        if let Declaration::Function {
            name,
            body: Some(main_body),
            ..
        } = &program.declarations[1]
        {
            assert_eq!(name, "main");
            // 断言 main 的函数体
            if let BlockItem::S(Statement::Return(expr)) = &main_body.blocks[0] {
                assert!(matches!(expr, Expression::FunctionCall { name, .. } if name == "add"));
            } else {
                panic!("Expected a return statement with a function call");
            }
        } else {
            panic!("Expected a function definition for 'main'.");
        }

        // 3. `add` 的定义
        if let Declaration::Function {
            name,
            params,
            body: Some(_),
            ..
        } = &program.declarations[2]
        {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
        } else {
            panic!("Expected a function definition for 'add'.");
        }

        println!("\n--- Function Declaration and Call Test Passed! ---");
    }
}
