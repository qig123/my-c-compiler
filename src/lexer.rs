//! src/lexer.rs

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // ... 保持不变 ...
    OpenParen,    // (
    CloseParen,   // )
    OpenBrace,    // {
    CloseBrace,   // }
    Semicolon,    // ;
    Minus,        // -
    Tilde,        // ~
    Decrement,    // --
    Plus,         // + (【修改】)
    Asterisk,     // * (【修改】)
    Slash,        // / (【修改】)
    Percent,      // % (【修改】)
    QuestionMark, // ?  <-- 新增
    Colon,        // :  <-- 新增
    // --- 第 4 章新增/修改的 Token ---
    Not,          // ! (从 Bang 修改)
    And,          // &&
    Or,           // ||
    Equal,        // == (从 EqualEqual 修改)
    NotEqual,     // != (从 BangEqual 修改)
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    Assign,       // =
    Comma,        //,
    KeywordInt,
    KeywordVoid,
    KeywordReturn,
    KeywordIf,
    KeywordElse,

    KeywordDo,
    KeywordWhile,
    KeywordFor,
    KeywordBreak,
    KeywordContinue,

    Identifier(String),
    IntegerConstant(i32),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
}

// 1. 定义 Lexer 结构体
pub struct Lexer<'a> {
    // 使用带生命周期的字符迭代器
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    // 将行号作为结构体的字段
    line: usize,
}

// 2. 为 Lexer 实现方法
impl<'a> Lexer<'a> {
    /// 创建一个新的 Lexer 实例。
    pub fn new(source: &'a str) -> Self {
        Lexer {
            chars: source.chars().peekable(),
            line: 1,
        }
    }

    /// 解析标识符或关键字（现在是方法）。
    fn lex_identifier_or_keyword(&mut self) -> TokenType {
        let mut identifier = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                identifier.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        match identifier.as_str() {
            "int" => TokenType::KeywordInt,
            "void" => TokenType::KeywordVoid,
            "return" => TokenType::KeywordReturn,
            "if" => TokenType::KeywordIf,
            "else" => TokenType::KeywordElse,
            "continue" => TokenType::KeywordContinue,
            "do" => TokenType::KeywordDo,
            "while" => TokenType::KeywordWhile,
            "for" => TokenType::KeywordFor,
            "break" => TokenType::KeywordBreak,
            _ => TokenType::Identifier(identifier),
        }
    }

    /// 解析整型常量（现在是方法）。
    fn lex_integer_constant(&mut self) -> Result<TokenType, String> {
        let mut number_str = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_digit(10) {
                number_str.push(c);
                self.chars.next();
            } else {
                break;
            }
        }

        if let Some(&next_char) = self.chars.peek() {
            if next_char.is_alphabetic() {
                let mut invalid_token = number_str;
                while let Some(&c) = self.chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        invalid_token.push(c);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                // 现在可以轻松访问 self.line！
                return Err(format!(
                    "Invalid token '{}' on line {}",
                    invalid_token, self.line
                ));
            }
        }

        match number_str.parse::<i32>() {
            Ok(num) => Ok(TokenType::IntegerConstant(num)),
            Err(_) => Err(format!("Failed to parse number: {}", number_str)),
        }
    }

    /// 核心方法：解析下一个 token。
    /// 返回 Option<Result<Token, String>>，这是实现 Iterator 的标准模式。
    // in src/lexer.rs, inside impl<'a> Lexer<'a>

    /// 核心方法：解析下一个 token。
    fn next_token(&mut self) -> Option<Result<Token, String>> {
        // 首先，跳过所有空白字符和预处理器指令
        loop {
            match self.chars.peek() {
                // 处理换行符
                Some('\n') => {
                    self.line += 1;
                    self.chars.next();
                }
                // 处理其他空白字符
                Some(' ') | Some('\t') | Some('\r') => {
                    self.chars.next();
                }
                // *** 新增的逻辑：处理预处理器指令 ***
                Some('#') => {
                    // 消耗掉 '#'
                    self.chars.next();
                    // 消耗掉这一行的剩余所有字符，直到换行符或文件结尾
                    while let Some(c) = self.chars.peek() {
                        if *c == '\n' {
                            // 遇到换行符，让外层循环来处理它（增加行号）
                            break;
                        }
                        self.chars.next();
                    }
                }
                // 遇到非空白、非'#'的字符，说明是 token 的开始，跳出循环
                _ => break,
            }
        }

        // 查看下一个有效字符
        let c = self.chars.peek().cloned()?; // 如果是 None，则表示输入结束

        // 根据字符类型分派
        let result = match c {
            '(' => {
                self.chars.next();
                Ok(TokenType::OpenParen)
            }
            ')' => {
                self.chars.next();
                Ok(TokenType::CloseParen)
            }
            '{' => {
                self.chars.next();
                Ok(TokenType::OpenBrace)
            }
            '}' => {
                self.chars.next();
                Ok(TokenType::CloseBrace)
            }
            ';' => {
                self.chars.next();
                Ok(TokenType::Semicolon)
            }
            '~' => {
                self.chars.next();
                Ok(TokenType::Tilde)
            }
            '+' => {
                self.chars.next();
                Ok(TokenType::Plus)
            }
            '*' => {
                self.chars.next();
                Ok(TokenType::Asterisk)
            }
            '/' => {
                self.chars.next();
                Ok(TokenType::Slash)
            }
            '%' => {
                self.chars.next();
                Ok(TokenType::Percent)
            }
            '?' => {
                self.chars.next();
                Ok(TokenType::QuestionMark)
            }
            ':' => {
                self.chars.next();
                Ok(TokenType::Colon)
            }
            ',' => {
                self.chars.next();
                Ok(TokenType::Comma)
            }
            '-' => {
                self.chars.next();
                if self.chars.peek() == Some(&'-') {
                    self.chars.next();
                    Ok(TokenType::Decrement)
                } else {
                    Ok(TokenType::Minus)
                }
            }
            '&' => {
                self.chars.next();
                if self.chars.peek() == Some(&'&') {
                    self.chars.next();
                    Ok(TokenType::And)
                } else {
                    Err(format!(
                        "Unrecognized character '{}' on line {}",
                        c, self.line
                    ))
                }
            }
            '|' => {
                self.chars.next();
                if self.chars.peek() == Some(&'|') {
                    self.chars.next();
                    Ok(TokenType::Or)
                } else {
                    Err(format!(
                        "Unrecognized character '{}' on line {}",
                        c, self.line
                    ))
                }
            }
            '!' => {
                self.chars.next();
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    Ok(TokenType::NotEqual)
                } else {
                    Ok(TokenType::Not)
                }
            }
            '<' => {
                self.chars.next();
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    Ok(TokenType::LessEqual)
                } else {
                    Ok(TokenType::Less)
                }
            }
            '>' => {
                self.chars.next();
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    Ok(TokenType::GreaterEqual)
                } else {
                    Ok(TokenType::Greater)
                }
            }
            '=' => {
                self.chars.next();
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    Ok(TokenType::Equal)
                } else {
                    Ok(TokenType::Assign)
                }
            }
            'a'..='z' | 'A'..='Z' | '_' => Ok(self.lex_identifier_or_keyword()),

            '0'..='9' => self.lex_integer_constant(),

            _ => Err(format!(
                "Unrecognized character '{}' on line {}",
                c, self.line
            )),
        };

        // 将结果（无论成功或失败）包装起来返回
        Some(match result {
            Ok(token_type) => Ok(Token {
                token_type,
                line: self.line,
            }),
            Err(e) => Err(e),
        })
    }
}

// 3. 为 Lexer 实现 Iterator trait
impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Token, TokenType};

    #[test]
    fn test_lex_loop_and_jump_keywords() {
        // 1. Arrange: 准备一个包含所有新关键字的C代码片段。
        // 我们使用一个 for 循环，内部嵌套一个 do-while 循环。
        // 在循环中，我们使用 if 来触发 break 和 continue。
        let source_code = r#"
            for (i = 0; i < 10; i = i + 1) {
                do {
                    if (a == 5) {
                        break;
                    }
                    continue;
                } while (x > 0);
            }
        "#;

        // 2. Arrange: 定义我们期望从 Lexer 中得到的 Token 序列。
        // 注意：我们也要正确地指定每个 Token 所在的行号。
        let expected_tokens = vec![
            // line 2: for (i = 0; i < 10; i = i + 1) {
            Token {
                token_type: TokenType::KeywordFor,
                line: 2,
            },
            Token {
                token_type: TokenType::OpenParen,
                line: 2,
            },
            Token {
                token_type: TokenType::Identifier("i".to_string()),
                line: 2,
            },
            Token {
                token_type: TokenType::Assign,
                line: 2,
            },
            Token {
                token_type: TokenType::IntegerConstant(0),
                line: 2,
            },
            Token {
                token_type: TokenType::Semicolon,
                line: 2,
            },
            Token {
                token_type: TokenType::Identifier("i".to_string()),
                line: 2,
            },
            Token {
                token_type: TokenType::Less,
                line: 2,
            },
            Token {
                token_type: TokenType::IntegerConstant(10),
                line: 2,
            },
            Token {
                token_type: TokenType::Semicolon,
                line: 2,
            },
            Token {
                token_type: TokenType::Identifier("i".to_string()),
                line: 2,
            },
            Token {
                token_type: TokenType::Assign,
                line: 2,
            },
            Token {
                token_type: TokenType::Identifier("i".to_string()),
                line: 2,
            },
            Token {
                token_type: TokenType::Plus,
                line: 2,
            },
            Token {
                token_type: TokenType::IntegerConstant(1),
                line: 2,
            },
            Token {
                token_type: TokenType::CloseParen,
                line: 2,
            },
            Token {
                token_type: TokenType::OpenBrace,
                line: 2,
            },
            // line 3: do {
            Token {
                token_type: TokenType::KeywordDo,
                line: 3,
            },
            Token {
                token_type: TokenType::OpenBrace,
                line: 3,
            },
            // line 4: if (a == 5) {
            Token {
                token_type: TokenType::KeywordIf,
                line: 4,
            },
            Token {
                token_type: TokenType::OpenParen,
                line: 4,
            },
            Token {
                token_type: TokenType::Identifier("a".to_string()),
                line: 4,
            },
            Token {
                token_type: TokenType::Equal,
                line: 4,
            },
            Token {
                token_type: TokenType::IntegerConstant(5),
                line: 4,
            },
            Token {
                token_type: TokenType::CloseParen,
                line: 4,
            },
            Token {
                token_type: TokenType::OpenBrace,
                line: 4,
            },
            // line 5: break;
            Token {
                token_type: TokenType::KeywordBreak,
                line: 5,
            },
            Token {
                token_type: TokenType::Semicolon,
                line: 5,
            },
            // line 6: }
            Token {
                token_type: TokenType::CloseBrace,
                line: 6,
            },
            // line 7: continue;
            Token {
                token_type: TokenType::KeywordContinue,
                line: 7,
            },
            Token {
                token_type: TokenType::Semicolon,
                line: 7,
            },
            // line 8: } while (x > 0);
            Token {
                token_type: TokenType::CloseBrace,
                line: 8,
            },
            Token {
                token_type: TokenType::KeywordWhile,
                line: 8,
            },
            Token {
                token_type: TokenType::OpenParen,
                line: 8,
            },
            Token {
                token_type: TokenType::Identifier("x".to_string()),
                line: 8,
            },
            Token {
                token_type: TokenType::Greater,
                line: 8,
            },
            Token {
                token_type: TokenType::IntegerConstant(0),
                line: 8,
            },
            Token {
                token_type: TokenType::CloseParen,
                line: 8,
            },
            Token {
                token_type: TokenType::Semicolon,
                line: 8,
            },
            // line 9: }
            Token {
                token_type: TokenType::CloseBrace,
                line: 9,
            },
        ];

        // 3. Act: 创建 Lexer 实例并收集所有 Tokens。
        // 我们使用 .unwrap() 是因为我们确信这个测试代码是有效的，不会产生词法错误。
        let lexer = Lexer::new(source_code);
        let actual_tokens: Vec<Token> = lexer.map(|result| result.unwrap()).collect();

        // 4. Assert: 比较实际生成的 Tokens 和我们期望的 Tokens。
        assert_eq!(actual_tokens, expected_tokens);
    }
}
