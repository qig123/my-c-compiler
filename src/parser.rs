//! src/parser.rs

// 从我们的 lexer 模块中导入 Token 和 TokenType
use crate::{
    ast::unchecked::*,
    lexer::{Token, TokenType},
};

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
            TokenType::QuestionMark => 3,
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

        // 【修改】直接调用新的辅助函数
        let body = self.parse_block()?;

        Ok(Function { name, body })
    }
    /// 【新增】解析一个由花括号包裹的块。
    /// <block> ::= "{" {<block-item>} "}"
    fn parse_block(&mut self) -> Result<Block, String> {
        self.expect_token(TokenType::OpenBrace)?; // 期望并消费 '{'

        let mut items = Vec::new();
        // 循环解析 block-item，直到遇到 '}'
        while self
            .peek()
            .map_or(false, |t| t.token_type != TokenType::CloseBrace)
        {
            items.push(self.parse_block_item()?);
        }

        self.expect_token(TokenType::CloseBrace)?; // 期望并消费 '}'

        Ok(Block { blocks: items })
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
    /// <statement> ::= "return" <exp> ";" | [<exp>] ";" |"if" "(" <exp> ")" <statement> ["else" <statement>] || <block>
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
                TokenType::KeywordIf => {
                    self.consume(); //消费if
                    self.expect_token(TokenType::OpenParen)?;
                    let c = self.parse_expression(0)?;
                    self.expect_token(TokenType::CloseParen)?;
                    let then_s = self.parse_statement()?;
                    let else_s;
                    if let Some(token) = self.peek() {
                        if token.token_type == TokenType::KeywordElse {
                            self.consume();
                            else_s = Some(Box::new(self.parse_statement()?));
                        } else {
                            else_s = None;
                        }
                    } else {
                        else_s = None;
                    }
                    return Ok(Statement::If {
                        condition: c,
                        then_stat: Box::new(then_s),
                        else_stat: else_s,
                    });
                }
                TokenType::OpenBrace => {
                    let block = self.parse_block()?;
                    Ok(Statement::Compound(block))
                }
                TokenType::KeywordFor => self.parse_for_statement(),
                TokenType::KeywordWhile => self.parse_while_statement(),
                TokenType::KeywordDo => self.parse_do_while_statement(),
                TokenType::KeywordBreak => {
                    self.consume(); // consume 'break'
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Break)
                }
                TokenType::KeywordContinue => {
                    self.consume();
                    self.expect_token(TokenType::Semicolon)?;
                    Ok(Statement::Continue)
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
    // "while" "(" <exp> ")" <statement>
    fn parse_while_statement(&mut self) -> Result<Statement, String> {
        self.consume(); // consume 'while'
        self.expect_token(TokenType::OpenParen)?;
        let condition = self.parse_expression(0)?;
        self.expect_token(TokenType::CloseParen)?;
        let body = self.parse_statement()?;
        Ok(Statement::While {
            condition,
            body: Box::new(body),
        })
    }

    // "do" <statement> "while" "(" <exp> ")" ";"
    fn parse_do_while_statement(&mut self) -> Result<Statement, String> {
        self.consume(); // consume 'do'
        let body = self.parse_statement()?;
        self.expect_token(TokenType::KeywordWhile)?;
        self.expect_token(TokenType::OpenParen)?;
        let condition = self.parse_expression(0)?;
        self.expect_token(TokenType::CloseParen)?;

        // 【修复 Bug 2】消费最后的那个分号！
        self.expect_token(TokenType::Semicolon)?;

        // 【修复 Bug 1】返回正确的 Statement::DoWhile 节点
        Ok(Statement::DoWhile {
            condition,
            body: Box::new(body),
        })
    }
    // for (<declaration>|<exp>; <exp>; <exp>) <statement>

    fn parse_for_statement(&mut self) -> Result<Statement, String> {
        self.consume(); // consume 'for'
        self.expect_token(TokenType::OpenParen)?;

        // 1. 解析初始化部分
        let init = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Semicolon)
        {
            None // 空的初始化
        } else if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::KeywordInt)
        {
            // 是一个声明
            Some(Box::new(self.parse_declaration().map(BlockItem::D)?))
        } else {
            // 是一个表达式
            Some(Box::new(
                self.parse_expression(0)
                    .map(Statement::Expression)
                    .map(BlockItem::S)?,
            ))
        };

        // C 语言中，for(exp;) 是合法的，但 for(declaration) 不带分号是非法的。
        // 如果是表达式，后面必须跟分号。如果是声明，分号已经在 parse_declaration 中消费了。
        if !matches!(init, Some(ref b) if matches!(**b, BlockItem::D(_))) {
            self.expect_token(TokenType::Semicolon)?;
        }

        // 2. 解析条件部分
        let condition = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::Semicolon)
        {
            None // 空条件
        } else {
            Some(self.parse_expression(0)?)
        };
        self.expect_token(TokenType::Semicolon)?;

        // 3. 解析循环后表达式
        let post = if self
            .peek()
            .map_or(false, |t| t.token_type == TokenType::CloseParen)
        {
            None // 空的 post-expression
        } else {
            Some(self.parse_expression(0)?)
        };
        self.expect_token(TokenType::CloseParen)?;

        // 4. 解析循环体
        let body = self.parse_statement()?;

        Ok(Statement::For {
            init,
            condition,
            post,
            body: Box::new(body),
        })
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
            } else if next_token.token_type == TokenType::QuestionMark {
                //右结合
                // `left` 已经是我们的 condition 部分
                // `?` 已经被消费了
                // 解析 "then" 分支
                let then_branch = self.parse_expression(0)?;
                // 期望一个冒号
                self.expect_token(TokenType::Colon)?;

                // 解析 "else" 分支，使用 '?' 的优先级进行右结合处理
                let else_branch = self.parse_expression(precedence)?;

                // 组装成 Conditional 节点
                left = Expression::Conditional {
                    condition: Box::new(left),
                    left: Box::new(then_branch),
                    right: Box::new(else_branch),
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
    /// 【修改】解析一个因子。 (更健壮的版本)
    /// <factor> ::= <int> | <identifier> | <unop> <factor> | "(" <exp> ")"
    fn parse_factor(&mut self) -> Result<Expression, String> {
        // 先 peek() 查看下一个 token 是什么
        let next_token = self
            .peek()
            .cloned()
            .ok_or_else(|| "Unexpected end of input, expected a factor.".to_string())?;

        match &next_token.token_type {
            // <factor> ::= <int>
            TokenType::IntegerConstant(val) => {
                self.consume(); // 匹配成功，现在消费它
                Ok(Expression::Constant(*val))
            }

            // <factor> ::= <identifier>
            TokenType::Identifier(name) => {
                self.consume(); // 匹配成功，现在消费它
                Ok(Expression::Var(name.clone()))
            }

            // <factor> ::= <unop> <factor>
            TokenType::Minus | TokenType::Tilde | TokenType::Not => {
                self.consume(); // 消费一元运算符
                let operator = self.token_to_unary_operator(&next_token.token_type)?;
                // 递归调用 parse_factor 来解析后面的因子
                let expression = self.parse_factor()?;
                Ok(Expression::Unary {
                    operator,
                    expression: Box::new(expression),
                })
            }

            // <factor> ::= "(" <exp> ")"
            TokenType::OpenParen => {
                self.consume(); // 消费 '('
                let inner_expression = self.parse_expression(0)?;
                self.expect_token(TokenType::CloseParen)?; // 消费 ')'
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

#[cfg(test)]
mod tests {
    use super::*; // 导入父模块（也就是你的 parser）的所有内容
    use crate::lexer::Lexer; // 导入 Lexer

    // 在这里写我们的调试测试
    #[test]
    fn debug_parsing_of_parenthesized_expression_statement() {
        // 1. 定义一个最小化的、能复现问题的 C 代码字符串
        // 这个例子 "(a);" 是一个合法的表达式语句，它会暴露你 parse_factor 中的 bug。
        let source_code = "int main(void) {
    if (1)
        return c;
    int c = 0;
}";

        // 2. 像你的 main.rs 一样，先进行词法分析
        println!("--- Lexing source code ---");
        let lexer = Lexer::new(source_code);
        let tokens: Vec<Token> = lexer.collect::<Result<_, _>>().unwrap();
        println!("{:#?}", tokens); // 打印出 tokens 方便查看

        // 3. 创建 Parser 并调用 parse 方法
        println!("\n--- Parsing tokens ---");
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        // 4. 断言结果并打印
        // 我们期望它能成功解析。如果失败，测试会 panic 并打印出详细的错误信息。
        // 这就是我们想要的调试入口！
        match result {
            Ok(ast) => {
                println!("\n--- Successfully Parsed AST ---");
                println!("{:#?}", ast);
                // 如果成功了，我们可以断言它成功了
                assert!(true);
            }
            Err(e) => {
                // 如果失败了，为了调试，我们故意让测试失败并打印错误
                panic!("\n--- PARSING FAILED! ---\nError: {}", e);
            }
        }
    }

    #[test]
    fn test_compound_statement() {
        // 一个包含复合语句的 C 代码
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

        // 1. 词法分析
        let lexer = Lexer::new(source_code);
        let tokens: Vec<Token> = lexer.collect::<Result<_, _>>().expect("Lexing failed");

        // 2. 语法分析
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("Parsing failed");

        // 3. 断言 AST 结构
        // 我们来验证解析出的 AST 是否符合我们的预期

        // main 函数应该有 3 个块项目
        assert_eq!(
            program.function.body.blocks.len(),
            3,
            "Function body should have 3 block items"
        );

        // 第一个项目：int a = 1; (这是一个声明)
        let first_item = &program.function.body.blocks[0];
        assert!(
            matches!(first_item, BlockItem::D(_)),
            "First item should be a Declaration"
        );

        // 第二个项目：{...} (这是一个复合语句)
        let second_item = &program.function.body.blocks[1];
        if let BlockItem::S(Statement::Compound(inner_block)) = second_item {
            // 复合语句内部应该有 2 个块项目
            assert_eq!(
                inner_block.blocks.len(),
                2,
                "Inner block should have 2 items"
            );

            // 复合语句的第一个项目：int b = 2;
            assert!(
                matches!(inner_block.blocks[0], BlockItem::D(_)),
                "Inner block's first item should be a Declaration"
            );

            // 复合语句的第二个项目：return b;
            assert!(
                matches!(inner_block.blocks[1], BlockItem::S(Statement::Return(_))),
                "Inner block's second item should be a Return statement"
            );
        } else {
            panic!(
                "Second item in function body should be a Compound statement. Got: {:?}",
                second_item
            );
        }

        // 第三个项目：return a; (这是一个返回语句)
        let third_item = &program.function.body.blocks[2];
        assert!(
            matches!(third_item, BlockItem::S(Statement::Return(_))),
            "Third item should be a Return statement"
        );

        println!("\n--- Compound Statement Test Passed! ---");
    }
    // 在 src/parser.rs 的 tests 模块中添加这个新测试
    // 在 src/parser.rs 的 tests 模块中

    #[test]
    fn test_parsing_of_all_loop_and_jump_statements() {
        // 1. Arrange
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

        // 2. Act
        let lexer = Lexer::new(source_code);
        let tokens: Vec<Token> = lexer.collect::<Result<_, _>>().expect("Lexing failed");

        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("Parsing failed");
        println!("--- Successfully Parsed AST ---\n{:#?}", program);

        // 3. Assert
        assert_eq!(
            program.function.body.blocks.len(),
            2,
            "Function body should have a for-loop and a return statement"
        );

        // --- 断言 `for` 循环 ---
        if let BlockItem::S(Statement::For {
            init,
            condition,
            post,
            body,
        }) = &program.function.body.blocks[0]
        {
            // 对于 Box<T>，我们需要解引用一次 (`**b`) 才能访问到 T
            assert!(
                matches!(init, Some(b) if matches!(**b, BlockItem::D(_))),
                "For loop init should be a declaration"
            );
            // 对于 Option<T>，我们不需要解引用
            assert!(
                matches!(condition, Some(Expression::Binary { .. })),
                "For loop condition should be a binary expression"
            );
            assert!(
                matches!(post, Some(Expression::Assign { .. })),
                "For loop post-expression should be an assignment"
            );

            // --- 断言 `for` 循环体内的 `while` 循环 ---
            // body 是 &Box<Statement>，解引用一次得到 &Statement
            if let Statement::Compound(for_body_block) = &**body {
                assert_eq!(
                    for_body_block.blocks.len(),
                    1,
                    "For loop body should contain one statement (the while loop)"
                );

                if let BlockItem::S(Statement::While {
                    condition: while_cond,
                    body: while_body,
                }) = &for_body_block.blocks[0]
                {
                    // 【修复】while_cond 是 &Expression，不需要解引用或只需一次
                    assert!(
                        matches!(while_cond, Expression::Constant(1)),
                        "While condition should be the constant 1"
                    );

                    // --- 断言 `while` 循环体内的 `do-while` 循环 ---
                    if let Statement::Compound(while_body_block) = &**while_body {
                        assert_eq!(
                            while_body_block.blocks.len(),
                            1,
                            "While loop body should contain one statement (the do-while loop)"
                        );

                        if let BlockItem::S(Statement::DoWhile {
                            body: do_while_body,
                            condition: do_while_cond,
                        }) = &while_body_block.blocks[0]
                        {
                            // 【修复】do_while_cond 是 &Expression
                            assert!(
                                matches!(do_while_cond, Expression::Binary { .. }),
                                "Do-while condition should be a binary expression"
                            );

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
                                    matches!(if_stmt, BlockItem::S(Statement::If {
                                     condition: _,
                                     then_stat,
                                     else_stat: None,
                                     // 【修复】then_stat 是 &Box<Statement>，解引用一次得到 &Statement
                                 }) if matches!(**then_stat, Statement::Continue)),
                                    "First statement in do-while should be 'if (...) continue;'"
                                );

                                // 断言 break;
                                let break_stmt = &do_while_body_block.blocks[1];
                                assert!(
                                    matches!(break_stmt, BlockItem::S(Statement::Break)),
                                    "Second statement in do-while should be a break statement"
                                );
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
                program.function.body.blocks[0]
            );
        }

        // --- 断言最后的 return 语句 ---
        let last_item = &program.function.body.blocks[1];
        assert!(
            matches!(last_item, BlockItem::S(Statement::Return(_))),
            "The last statement should be a return"
        );

        println!("\n--- All Loop and Jump Statements Test Passed! ---");
    }
}
