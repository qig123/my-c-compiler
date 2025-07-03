//! src/semantics/validator.rs

// 【修改】 在 use 语句中明确列出所有需要的类型，包括 Empty
use crate::{ast::unchecked::*, common::UniqueIdGenerator};
use std::collections::HashMap;

/// The Validator performs semantic analysis, specifically variable resolution.
/// It walks the AST, checking for errors like undeclared variables or
/// duplicate declarations, and transforms the AST to use unique variable names.
pub struct Validator<'a> {
    /// Maps user-defined variable names in the current scope to unique names.
    scopes: Vec<HashMap<String, String>>,
    id_generator: &'a mut UniqueIdGenerator,
}

impl<'a> Validator<'a> {
    /// Creates a new Validator.
    pub fn new(id_generator: &'a mut UniqueIdGenerator) -> Self {
        Validator {
            scopes: Vec::new(),
            id_generator,
        }
    }

    /// The main entry point for validation.
    pub fn validate_program(&mut self, program: Program) -> Result<Program, String> {
        let validated_function = self.validate_function(program.function)?;
        Ok(Program {
            function: validated_function,
        })
    }

    /// Generates a new unique name for a variable.
    fn generate_unique_name(&mut self, original_name: &str) -> String {
        // 调用共享的生成器来获取下一个 ID
        let unique_id = self.id_generator.next();
        format!("{}.{}", original_name, unique_id)
    }

    fn validate_function(&mut self, function: Function) -> Result<Function, String> {
        // 注意：函数本身也构成一个作用域块
        let validated_body = self.validate_block(function.body)?;

        Ok(Function {
            name: function.name,
            body: validated_body,
        })
    }
    /// 验证一个块，处理作用域的进入和退出，并验证其所有子项。
    fn validate_block(&mut self, block: Block) -> Result<Block, String> {
        // 1. 进入新作用域
        self.enter_scope();

        // 2. 遍历并验证块内的所有项目
        let mut validated_items = Vec::new();
        for item in block.blocks {
            validated_items.push(self.validate_block_item(item)?);
        }

        // 3. 退出作用域
        self.exit_scope();

        // 4. 返回包含已验证项目的新的 Block
        Ok(Block {
            blocks: validated_items,
        })
    }

    fn validate_block_item(&mut self, item: BlockItem) -> Result<BlockItem, String> {
        match item {
            BlockItem::S(stmt) => {
                let validated_stmt = self.validate_statement(stmt)?;
                Ok(BlockItem::S(validated_stmt))
            }
            BlockItem::D(decl) => {
                let validated_decl = self.validate_declaration(decl)?;
                Ok(BlockItem::D(validated_decl))
            }
        }
    }

    fn validate_declaration(&mut self, decl: Declaration) -> Result<Declaration, String> {
        // 1. 先生成唯一的名称。这是一个可变借用，但它在这里就结束了。
        let unique_name = self.generate_unique_name(&decl.name);

        // 2. 现在，再开始对 scope 进行可变借用。
        let m = self.scopes.last_mut().unwrap();

        // 3. 检查和插入。
        if m.contains_key(&decl.name) {
            return Err(format!(
                "Duplicate variable declaration for '{}'",
                decl.name
            ));
        }
        m.insert(decl.name.clone(), unique_name.clone());

        // 4. 处理初始化器（这是一个新的可变借用，但之前的已经结束了）
        let validated_init = match decl.init {
            Some(expr) => Some(self.validate_expression(expr)?),
            None => None,
        };

        Ok(Declaration {
            name: unique_name,
            init: validated_init,
        })
    }

    // --- THIS IS THE CORRECTED FUNCTION ---
    fn validate_statement(&mut self, stmt: Statement) -> Result<Statement, String> {
        match stmt {
            Statement::Return(expr) => {
                let validated_expr = self.validate_expression(expr)?;
                Ok(Statement::Return(validated_expr))
            }
            Statement::Expression(expr) => {
                // This handles statements like "a = 5;"
                let validated_expr = self.validate_expression(expr)?;
                Ok(Statement::Expression(validated_expr))
            }
            Statement::Empty => {
                // This handles null statements like ";"
                // There is no expression to validate, so we just return it as is.
                Ok(Statement::Empty)
            }
            Statement::If {
                condition,
                then_stat,
                else_stat,
            } => {
                // 1. 验证条件表达式
                let validated_condition = self.validate_expression(condition)?;

                // 2. 验证 then 分支的语句
                // 注意：这里是递归调用 validate_statement
                let validated_then = self.validate_statement(*then_stat)?;

                // 3. 验证可选的 else 分支
                let validated_else = match else_stat {
                    Some(else_s) => {
                        // 如果存在 else 分支，同样递归验证它
                        let validated = self.validate_statement(*else_s)?;
                        Some(Box::new(validated))
                    }
                    None => None, // 如果不存在，就是 None
                };

                // 4. 用验证过的新部分重新组装成一个 If 语句
                Ok(Statement::If {
                    condition: validated_condition,
                    then_stat: Box::new(validated_then),
                    else_stat: validated_else,
                })
            }
            Statement::Compound(b) => {
                let validated_block = self.validate_block(b)?;
                Ok(Statement::Compound(validated_block))
            }
            Statement::While { condition, body } => {
                let validated_condition = self.validate_expression(condition)?;
                let validated_body = self.validate_statement(*body)?;
                Ok(Statement::While {
                    condition: validated_condition,
                    body: Box::new(validated_body),
                })
            }
            Statement::DoWhile { body, condition } => {
                let validated_condition = self.validate_expression(condition)?;
                let validated_body = self.validate_statement(*body)?;
                Ok(Statement::DoWhile {
                    condition: validated_condition,
                    body: Box::new(validated_body),
                })
            }
            Statement::Break => Ok(Statement::Break),
            Statement::Continue => Ok(Statement::Continue),
            Statement::For {
                init,
                condition,
                post,
                body,
            } => {
                self.enter_scope();
                // 1. 验证初始化部分 (它在这个新作用域内)
                let validated_init = match init {
                    Some(item) => Some(Box::new(self.validate_block_item(*item)?)),
                    None => None,
                };
                // 2. 验证条件部分 (可以访问初始化中声明的变量)
                let validated_condition = match condition {
                    Some(expr) => Some(self.validate_expression(expr)?),
                    None => None,
                };
                // 3. 验证循环后表达式
                let validated_post = match post {
                    Some(expr) => Some(self.validate_expression(expr)?),
                    None => None,
                };
                // 4. 验证循环体
                let validated_body = Box::new(self.validate_statement(*body)?);
                self.exit_scope(); // 销毁 for 循环的作用域
                Ok(Statement::For {
                    init: validated_init,
                    condition: validated_condition,
                    post: validated_post,
                    body: validated_body,
                })
            }
        }
    }

    fn validate_expression(&mut self, expr: Expression) -> Result<Expression, String> {
        match expr {
            Expression::Constant(c) => Ok(Expression::Constant(c)),

            Expression::Var(name) => {
                if let Some(unique_name) = self.find_variable(&self.scopes, &name.clone()) {
                    Ok(Expression::Var(unique_name))
                } else {
                    Err(format!("Use of undeclared variable '{}'", name))
                }
            }

            Expression::Assign { left, right } => {
                if !matches!(*left, Expression::Var(_)) {
                    return Err(format!("Invalid l-value for assignment: {:?}", left));
                }

                let validated_left = self.validate_expression(*left)?;
                let validated_right = self.validate_expression(*right)?;

                Ok(Expression::Assign {
                    left: Box::new(validated_left),
                    right: Box::new(validated_right),
                })
            }

            Expression::Unary {
                operator,
                expression,
            } => {
                let validated_expr = self.validate_expression(*expression)?;
                Ok(Expression::Unary {
                    operator,
                    expression: Box::new(validated_expr),
                })
            }

            Expression::Binary {
                operator,
                left,
                right,
            } => {
                let validated_left = self.validate_expression(*left)?;
                let validated_right = self.validate_expression(*right)?;
                Ok(Expression::Binary {
                    operator,
                    left: Box::new(validated_left),
                    right: Box::new(validated_right),
                })
            }
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                let validated_cond = self.validate_expression(*condition)?;
                let validated_then = self.validate_expression(*left)?;
                let validated_else = self.validate_expression(*right)?;
                Ok(Expression::Conditional {
                    condition: Box::new(validated_cond),
                    left: Box::new(validated_then),
                    right: Box::new(validated_else),
                })
            }
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }
    fn find_variable(&self, variable_vec: &[HashMap<String, String>], key: &str) -> Option<String> {
        variable_vec
            .iter()
            .rev()
            .find_map(|map| map.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::UniqueIdGenerator;
    use crate::lexer::{Lexer, Token};
    use crate::parser::Parser;

    // 一个辅助函数，用于快速运行完整的 词法->语法->语义分析 流程
    fn validate_source(source: &str) -> Result<Program, String> {
        let lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer.collect::<Result<_, _>>().unwrap();
        println!("{:#?}", tokens); // 打印出 tokens 方便查看
        let ast = Parser::new(&tokens).parse().unwrap();
        let mut id_gen = UniqueIdGenerator::new();
        let mut validator = Validator::new(&mut id_gen);
        validator.validate_program(ast)
    }

    #[test]
    fn test_variable_shadowing_and_scopes() {
        let source_code = r#"
            int main(void) {
                int x = 1;      
                int y = x;     
                {
                    int x = 2;  
                    y = x;      
                }
                return x; 
            }
        "#;

        let validated_ast = validate_source(source_code).expect("Validation should succeed");

        // 我们来深入检查 AST，确保变量名被正确替换
        let function_body = &validated_ast.function.body.blocks;

        // 1. int x = 1; -> decl "x.0"
        let decl_x0 = match &function_body[0] {
            BlockItem::D(d) => d,
            _ => panic!("Expected declaration"),
        };
        assert_eq!(decl_x0.name, "x.0");

        // 2. int y = x; -> decl "y.1", init uses "x.0"
        let decl_y1 = match &function_body[1] {
            BlockItem::D(d) => d,
            _ => panic!("Expected declaration"),
        };
        assert_eq!(decl_y1.name, "y.1");
        let init_y1 = decl_y1.init.as_ref().unwrap();
        assert_eq!(*init_y1, Expression::Var("x.0".to_string()));

        // 3. { ... } -> Compound Statement
        let compound_stmt = match &function_body[2] {
            BlockItem::S(Statement::Compound(b)) => b,
            _ => panic!("Expected compound statement"),
        };
        let inner_items = &compound_stmt.blocks;

        // 3a. int x = 2; -> decl "x.2"
        let decl_x2 = match &inner_items[0] {
            BlockItem::D(d) => d,
            _ => panic!("Expected inner declaration"),
        };
        assert_eq!(decl_x2.name, "x.2");

        // 3b. y = x; -> Assignment, lhs is "y.1", rhs is "x.2"
        let assign_stmt = match &inner_items[1] {
            BlockItem::S(Statement::Expression(e)) => e,
            _ => panic!("Expected expression statement"),
        };
        if let Expression::Assign { left, right } = assign_stmt {
            assert_eq!(**left, Expression::Var("y.1".to_string()));
            assert_eq!(**right, Expression::Var("x.2".to_string()));
        } else {
            panic!("Expected assignment expression");
        }

        // 4. return x; -> Return, uses "x.0"
        let return_stmt = match &function_body[3] {
            BlockItem::S(Statement::Return(e)) => e,
            _ => panic!("Expected return statement"),
        };
        assert_eq!(*return_stmt, Expression::Var("x.0".to_string()));

        println!("--- Variable Shadowing Test Passed! ---");
    }
    #[test]
    fn test_loop_scoping_and_variables() {
        // 这个测试用例检查 for 循环的特殊作用域规则
        let source_code = r#"
        int main(void) {
            int a = 10;
            int i = 0; 
            for (int i = 0; i < a; i = i + 1) { 
                int b = i;
            }
            return i;
        }
    "#;

        let validated_ast = validate_source(source_code).expect("Validation should succeed");
        let function_body = &validated_ast.function.body.blocks;

        // 1. int a = 10; -> a.0
        let decl_a0 = &function_body[0];
        assert!(matches!(decl_a0, BlockItem::D(Declaration { name, .. }) if name == "a.0"));

        // 2. int i = 0; -> i.1 (外层 i)
        let decl_i1 = &function_body[1];
        assert!(matches!(decl_i1, BlockItem::D(Declaration { name, .. }) if name == "i.1"));

        // 3. for (...) { ... }
        if let BlockItem::S(Statement::For {
            init,
            condition,
            post,
            body,
        }) = &function_body[2]
        {
            // 3a. for(int i = 0; ...) -> init 声明了 i.2
            if let Some(init_item) = init {
                if let BlockItem::D(decl) = &**init_item {
                    assert_eq!(decl.name, "i.2");
                } else {
                    panic!("Expected declaration in for init");
                }
            } else {
                panic!("Expected for init");
            }

            // 3b. ...; i < a; ... -> condition 使用 i.2 和 a.0
            if let Some(Expression::Binary { left, right, .. }) = condition {
                assert_eq!(**left, Expression::Var("i.2".to_string()));
                assert_eq!(**right, Expression::Var("a.0".to_string()));
            } else {
                panic!("Expected binary expression in condition");
            }

            // 3c. ...; i = i + 1 -> post 使用 i.2
            if let Some(Expression::Assign { left, .. }) = post {
                assert_eq!(**left, Expression::Var("i.2".to_string()));
            } else {
                panic!("Expected assignment in post-expression");
            }

            // 3d. for (...) { int b = i; } -> body 使用 i.2
            if let Statement::Compound(block) = &**body {
                if let BlockItem::D(decl_b) = &block.blocks[0] {
                    assert_eq!(decl_b.name, "b.3");
                    if let Some(Expression::Var(name)) = &decl_b.init {
                        assert_eq!(*name, "i.2");
                    } else {
                        panic!("Expected var in inner decl init");
                    }
                } else {
                    panic!("Expected inner declaration");
                }
            } else {
                panic!("Expected compound statement for body");
            }
        } else {
            panic!("Expected a for loop");
        }

        // 4. return i; -> 使用外层的 i.1
        if let BlockItem::S(Statement::Return(expr)) = &function_body[3] {
            assert_eq!(*expr, Expression::Var("i.1".to_string()));
        } else {
            panic!("Expected a return statement");
        }

        println!("--- Loop Scoping Test Passed! ---");
    }
}
