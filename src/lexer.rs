//! src/lexer.rs

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // ... 保持不变 ...
    OpenParen,  // (
    CloseParen, // )
    OpenBrace,  // {
    CloseBrace, // }
    Semicolon,  // ;
    KeywordInt,
    KeywordVoid,
    KeywordReturn,
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
