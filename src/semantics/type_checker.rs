// src/semantics/type_checker.rs

use crate::ast::unchecked::*;
use std::collections::HashMap;

/// 表示 C 语言中的基本类型
#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Int,
    // 在这个阶段，我们只关心函数参数的数量
    Function { param_count: usize },
}

/// 符号表中存储的关于一个标识符的信息
#[derive(Debug, Clone)]
pub struct Symbol {
    /// 标识符的类型
    pub c_type: CType,
    /// 如果是函数，它是否已经被定义 (有函数体)
    pub defined: bool,
}

/// 类型检查器，它会构建并持有一个符号表
pub struct TypeChecker {
    /// 符号表，将标识符名称映射到其类型和定义状态
    /// 注意：这里的 key 是在标识符解析后可能被重命名的名字
    pub symbols: HashMap<String, Symbol>,
}
// 在 TypeChecker 定义之后

impl TypeChecker {
    /// 创建一个新的、空的 TypeChecker
    pub fn new() -> Self {
        TypeChecker {
            symbols: HashMap::new(),
        }
    }

    /// 类型检查的主入口。
    /// 它不返回新的 AST，如果成功，它会填充自身的符号表。
    /// 如果失败，它返回一个错误字符串。
    pub fn check_program(&mut self, prog: &Program) -> Result<(), String> {
        // 遍历所有顶层声明，填充符号表并进行检查
        for decl in &prog.declarations {
            self.check_declaration(decl)?;
        }

        // 成功，没有错误
        Ok(())
    }

    /// 检查一个声明（函数或变量）
    fn check_declaration(&mut self, decl: &Declaration) -> Result<(), String> {
        match decl {
            Declaration::Function { name, params, body } => {
                let param_count = params.len();
                let has_body = body.is_some();
                let fun_type = CType::Function { param_count };

                let mut already_defined = false;

                // 检查符号表中是否已存在该函数
                if let Some(old_symbol) = self.symbols.get(name) {
                    // 1. 检查类型是否兼容
                    if old_symbol.c_type != fun_type {
                        return Err(format!("Incompatible declaration for function '{}'", name));
                    }
                    already_defined = old_symbol.defined;
                }

                // 2. 检查是否重复定义
                if already_defined && has_body {
                    return Err(format!("Function '{}' is defined more than once", name));
                }

                // 3. 添加或更新符号表条目
                let new_symbol = Symbol {
                    c_type: fun_type,
                    defined: already_defined || has_body,
                };
                self.symbols.insert(name.clone(), new_symbol);

                // 4. 如果有函数体，深入检查
                if let Some(block) = body {
                    // 为函数体创建一个新的 "作用域"
                    // 在这个简化的类型检查器中，我们不处理作用域，
                    // 因为所有变量都已经是唯一名称了。
                    // 我们只需临时将参数添加到符号表中。
                    for param_name in params {
                        self.symbols.insert(
                            param_name.clone(),
                            Symbol {
                                c_type: CType::Int,
                                defined: true, // 参数总被视为已定义
                            },
                        );
                    }

                    self.check_block(block)?;

                    // 检查完函数体后，移除参数，防止它们污染全局符号表
                    for param_name in params {
                        self.symbols.remove(param_name);
                    }
                }
            }
            Declaration::Variable { name, init } => {
                // 标识符解析后，变量名已经是唯一的，所以我们直接添加
                self.symbols.insert(
                    name.clone(),
                    Symbol {
                        c_type: CType::Int,
                        defined: true,
                    },
                );

                // 检查初始化表达式
                if let Some(init_expr) = init {
                    self.check_expression(init_expr)?;
                }
            }
        }
        Ok(())
    }

    /// 检查一个块
    fn check_block(&mut self, block: &Block) -> Result<(), String> {
        for item in &block.blocks {
            self.check_block_item(item)?;
        }
        Ok(())
    }

    /// 检查块中的一项
    fn check_block_item(&mut self, item: &BlockItem) -> Result<(), String> {
        match item {
            BlockItem::S(stmt) => self.check_statement(stmt),
            BlockItem::D(decl) => self.check_declaration(decl),
        }
    }

    /// 检查一个语句
    fn check_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Return(expr) | Statement::Expression(expr) => self.check_expression(expr),
            Statement::If {
                condition,
                then_stat,
                else_stat,
            } => {
                self.check_expression(condition)?;
                self.check_statement(then_stat)?;
                if let Some(else_s) = else_stat {
                    self.check_statement(else_s)?;
                }
                Ok(())
            }
            Statement::Compound(block) => self.check_block(block),
            Statement::For {
                init,
                condition,
                post,
                body,
            } => {
                if let Some(init_item) = init {
                    self.check_block_item(init_item)?;
                }
                if let Some(cond_expr) = condition {
                    self.check_expression(cond_expr)?;
                }
                if let Some(post_expr) = post {
                    self.check_expression(post_expr)?;
                }
                self.check_statement(body)
            }
            Statement::While { condition, body } => {
                self.check_expression(condition)?;
                self.check_statement(body)
            }
            Statement::DoWhile { body, condition } => {
                self.check_statement(body)?;
                self.check_expression(condition)
            }
            // Empty, Break, Continue 不需要类型检查
            Statement::Empty | Statement::Break | Statement::Continue => Ok(()),
        }
    }

    /// 检查一个表达式
    fn check_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::Constant(_) => Ok(()), // 常量总是合法的
            Expression::Var(name) => {
                let symbol = self.symbols.get(name).ok_or_else(|| {
                    format!(
                        "Internal error: undeclared identifier '{}' after validation pass",
                        name
                    )
                })?;

                // 检查函数名是否被用作变量
                if matches!(symbol.c_type, CType::Function { .. }) {
                    return Err(format!("Function '{}' used as a variable", name));
                }
                Ok(())
            }
            Expression::FunctionCall { name, args } => {
                let symbol = self.symbols.get(name).ok_or_else(|| {
                    format!(
                        "Internal error: undeclared identifier '{}' after validation pass",
                        name
                    )
                })?;

                // 检查变量是否被用作函数
                match symbol.c_type {
                    CType::Int => Err(format!("Variable '{}' used as a function", name)),
                    CType::Function { param_count } => {
                        // 检查参数数量
                        if args.len() != param_count {
                            return Err(format!(
                                "Function '{}' called with {} arguments, but expects {}",
                                name,
                                args.len(),
                                param_count
                            ));
                        }
                        // 递归检查每个参数表达式
                        for arg in args {
                            self.check_expression(arg)?;
                        }
                        Ok(())
                    }
                }
            }
            Expression::Assign { left, right } => {
                // 对于赋值，左右两边都需要是 int (在这个阶段)
                // 标识符解析器已经确保了左边是 l-value (Var)
                self.check_expression(left)?;
                self.check_expression(right)?;
                Ok(())
            }
            Expression::Unary { expression, .. } => self.check_expression(expression),
            Expression::Binary { left, right, .. } => {
                self.check_expression(left)?;
                self.check_expression(right)?;
                Ok(())
            }
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                self.check_expression(condition)?;
                self.check_expression(left)?;
                self.check_expression(right)?;
                Ok(())
            }
        }
    }
}
