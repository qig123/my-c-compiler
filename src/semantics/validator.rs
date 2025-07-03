//! src/semantics/validator.rs

use crate::{ast::unchecked::*, common::UniqueIdGenerator};
use std::collections::HashMap;
// 定义一个结构来存储标识符的详细信息
#[derive(Debug, Clone)]
struct IdentifierInfo {
    /// 解析后的唯一名称。对于全局实体，这与原始名称相同。
    unique_name: String,
    /// 是否具有外部链接 (即，是否是全局函数/变量)？
    has_external_linkage: bool,
    // 未来你可以在这里添加类型信息，用于类型检查 Pass
    // ty: CType,
}

pub struct Validator<'a> {
    scopes: Vec<HashMap<String, IdentifierInfo>>,
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
    /// Generates a new unique name for a variable.
    fn generate_unique_name(&mut self, original_name: &str) -> String {
        // 调用共享的生成器来获取下一个 ID
        let unique_id = self.id_generator.next();
        format!("{}.{}", original_name, unique_id)
    }

    /// The main entry point for validation.
    pub fn validate_program(&mut self, program: Program) -> Result<Program, String> {
        // 1. 进入全局作用域 (这是所有顶层声明所在的地方)
        self.enter_scope();
        let mut validated_decls = Vec::new();
        for decl in program.declarations {
            // 在全局作用域内验证每个声明
            let validated_decl = self.validate_declaration(decl, true)?; // true 表示在全局作用域
            validated_decls.push(validated_decl);
        }

        // 注意：全局作用域在整个验证过程中都存在，所以先不退出
        // self.exit_scope();

        Ok(Program {
            declarations: validated_decls,
        })
    }
    // 这个函数需要知道自己是否在处理一个全局声明
    fn validate_declaration(
        &mut self,
        decl: Declaration,
        is_global: bool,
    ) -> Result<Declaration, String> {
        match decl {
            Declaration::Function { name, params, body } => {
                // 如果不是在全局作用域，但遇到了函数定义，这是非法的嵌套函数
                if !is_global && body.is_some() {
                    return Err(format!(
                        "Nested function definitions are not allowed: '{}'",
                        name
                    ));
                }

                // 检查当前作用域是否已有同名且无链接的实体 (如局部变量)
                if let Some(map) = self.scopes.last() {
                    if let Some(prev_entry) = map.get(&name) {
                        if !prev_entry.has_external_linkage {
                            return Err(format!(
                                "Duplicate declaration: '{}' conflicts with a local variable.",
                                name
                            ));
                        }
                    }
                }

                // 函数具有外部链接，不重命名
                let info = IdentifierInfo {
                    unique_name: name.clone(),
                    has_external_linkage: true,
                };
                self.scopes.last_mut().unwrap().insert(name.clone(), info);

                // --- 【核心修改在这里】---

                // 1. 为函数参数和函数体创建一个共享的新作用域
                self.enter_scope();

                // 2. 验证并重命名参数，将它们加入这个新作用域
                let mut validated_params = Vec::new();
                for param_name in params {
                    // 检查参数是否在当前作用域（也就是参数列表自身）中重复
                    if self.scopes.last().unwrap().contains_key(&param_name) {
                        return Err(format!(
                            "Duplicate parameter name '{}' in function '{}'",
                            param_name, name
                        ));
                    }
                    let unique_param_name = self.generate_unique_name(&param_name);
                    let param_info = IdentifierInfo {
                        unique_name: unique_param_name.clone(),
                        has_external_linkage: false,
                    };
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(param_name.clone(), param_info); // 使用 clone
                    validated_params.push(unique_param_name);
                }

                // 3. 验证函数体 (如果存在的话)
                let validated_body = match body {
                    Some(block) => {
                        // 直接在参数所在的作用域中，验证函数体内的每一个项目
                        let mut validated_items = Vec::new();
                        for item in block.blocks {
                            // validate_block_item 会调用 validate_declaration(decl, false)
                            // 它会在参数所在的作用域中检查 'int a = 5'
                            // 此时，作用域中已经有参数 'a' 了，所以会触发重复声明错误
                            validated_items.push(self.validate_block_item(item)?);
                        }
                        Some(Block {
                            blocks: validated_items,
                        })
                    }
                    None => None,
                };

                // 4. 退出函数作用域
                self.exit_scope();

                Ok(Declaration::Function {
                    name,
                    params: validated_params,
                    body: validated_body,
                })
            }
            Declaration::Variable { name, init } => {
                // 与函数类似，检查当前作用域是否有冲突
                if self.scopes.last().unwrap().contains_key(&name) {
                    return Err(format!("Duplicate variable declaration for '{}'", name));
                }

                let unique_name;
                let has_linkage;

                if is_global {
                    // 全局变量，不重命名
                    unique_name = name.clone();
                    has_linkage = true;
                } else {
                    // 局部变量，生成唯一名称
                    unique_name = self.generate_unique_name(&name);
                    has_linkage = false;
                }

                let info = IdentifierInfo {
                    unique_name: unique_name.clone(),
                    has_external_linkage: has_linkage,
                };
                self.scopes.last_mut().unwrap().insert(name, info);

                // 验证初始化表达式
                let validated_init = match init {
                    Some(expr) => Some(self.validate_expression(expr)?),
                    None => None,
                };

                Ok(Declaration::Variable {
                    name: unique_name, // 使用新的（或原始的）名字
                    init: validated_init,
                })
            }
        }
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
                // 【核心修正】
                // 当我们在这里验证一个声明时，它肯定是在一个块内部，
                // 所以它是一个局部声明，is_global 应该是 false。
                let validated_decl = self.validate_declaration(decl, false)?;
                Ok(BlockItem::D(validated_decl))
            }
        }
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
                // 使用新的 find_variable 逻辑
                if let Some(info) = self.find_identifier(&name) {
                    // 使用 info 中的 unique_name
                    Ok(Expression::Var(info.unique_name))
                } else {
                    Err(format!("Use of undeclared variable '{}'", name))
                }
            }
            Expression::FunctionCall { name, args } => {
                // 查找函数名
                let resolved_name = if let Some(info) = self.find_identifier(&name) {
                    // 在这里可以做一个简单的类型检查：这个名字必须指向一个函数
                    if !info.has_external_linkage {
                        // 这是一个简化，假设只有函数才有链接。
                        // 更完整的检查应该在类型检查 Pass 中进行。
                        return Err(format!(
                            "'{}' is a variable and cannot be called as a function",
                            name
                        ));
                    }
                    info.unique_name // 对于函数，这个名字和原始名字一样
                } else {
                    return Err(format!("Call to undeclared function '{}'", name));
                };

                // 递归验证所有参数
                let mut validated_args = Vec::new();
                for arg in args {
                    validated_args.push(self.validate_expression(arg)?);
                }

                Ok(Expression::FunctionCall {
                    name: resolved_name,
                    args: validated_args,
                })
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
    fn find_identifier(&self, key: &str) -> Option<IdentifierInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|map| map.get(key).cloned())
    }
    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
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

        // --- 【核心修改】从 Program AST 中提取 main 函数体 ---
        let main_function = match &validated_ast.declarations[0] {
            Declaration::Function { name, body, .. } if name == "main" => {
                body.as_ref().expect("main function should have a body")
            }
            _ => panic!("Expected a function declaration for main"),
        };
        let function_body = &main_function.blocks;
        // --- 修改结束 ---

        // 后续的断言逻辑完全保持不变，因为它们是正确的
        // 1. int x = 1; -> decl "x.0"
        let decl_x0 = match &function_body[0] {
            BlockItem::D(Declaration::Variable { name, .. }) => name, // 匹配 Variable 变体
            _ => panic!("Expected variable declaration"),
        };
        assert_eq!(decl_x0, "x.0");

        // 2. int y = x; -> decl "y.1", init uses "x.0"
        let decl_y1 = match &function_body[1] {
            BlockItem::D(Declaration::Variable { name, init, .. }) => (name, init), // 同时获取名字和初始化器
            _ => panic!("Expected variable declaration"),
        };
        assert_eq!(decl_y1.0, "y.1");
        let init_y1 = decl_y1.1.as_ref().unwrap();
        assert_eq!(*init_y1, Expression::Var("x.0".to_string()));

        // 3. { ... } -> Compound Statement
        let compound_stmt = match &function_body[2] {
            BlockItem::S(Statement::Compound(b)) => b,
            _ => panic!("Expected compound statement"),
        };
        let inner_items = &compound_stmt.blocks;

        // 3a. int x = 2; -> decl "x.2"
        let decl_x2 = match &inner_items[0] {
            BlockItem::D(Declaration::Variable { name, .. }) => name,
            _ => panic!("Expected inner declaration"),
        };
        assert_eq!(decl_x2, "x.2");

        // 3b. y = x; -> Assignment, lhs is "y.1", rhs is "x.2"
        let assign_stmt = match &inner_items[1] {
            BlockItem::S(Statement::Expression(e)) => e,
            _ => panic!("Expected expression statement"),
        };
        if let Expression::Assign { left, right } = assign_stmt {
            // 【注意】赋值的左边也是一个 Expression::Var
            if let Expression::Var(var_name) = &**left {
                assert_eq!(var_name, "y.1");
            } else {
                panic!("Expected a variable on the left side of assignment");
            }
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

        // --- 【核心修改】从 Program AST 中提取 main 函数体 ---
        let main_function = match &validated_ast.declarations[0] {
            Declaration::Function { name, body, .. } if name == "main" => {
                body.as_ref().expect("main function should have a body")
            }
            _ => panic!("Expected a function declaration for main"),
        };
        let function_body = &main_function.blocks;
        // --- 修改结束 ---

        // 后续的断言逻辑也需要根据新的 Declaration::Variable 结构进行调整
        // 1. int a = 10; -> a.0
        let decl_a0 = &function_body[0];
        assert!(
            matches!(decl_a0, BlockItem::D(Declaration::Variable { name, .. }) if name == "a.0")
        );

        // 2. int i = 0; -> i.1 (外层 i)
        let decl_i1 = &function_body[1];
        assert!(
            matches!(decl_i1, BlockItem::D(Declaration::Variable { name, .. }) if name == "i.1")
        );

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
                if let BlockItem::D(Declaration::Variable { name, .. }) = &**init_item {
                    // 匹配 Variable
                    assert_eq!(*name, "i.2");
                } else {
                    panic!("Expected variable declaration in for init");
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
                if let Expression::Var(var_name) = &**left {
                    assert_eq!(var_name, "i.2");
                } else {
                    panic!("Expected a variable on the left side of assignment");
                }
            } else {
                panic!("Expected assignment in post-expression");
            }

            // 3d. for (...) { int b = i; } -> body 使用 i.2
            if let Statement::Compound(block) = &**body {
                if let BlockItem::D(decl_b) = &block.blocks[0] {
                    if let Declaration::Variable {
                        name: b_name,
                        init: b_init,
                        ..
                    } = decl_b
                    {
                        assert_eq!(*b_name, "b.3");
                        if let Some(Expression::Var(name)) = b_init {
                            assert_eq!(*name, "i.2");
                        } else {
                            panic!("Expected var in inner decl init");
                        }
                    } else {
                        panic!("Expected inner declaration to be a variable");
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
    //测试 1：简单的函数调用
    #[test]
    fn test_simple_function_call() {
        let source_code = r#"
        int add(int a, int b) {
            return a + b;
        }

        int main(void) {
            return add(1, 2);
        }
    "#;

        let validated_ast = validate_source(source_code).expect("Validation should succeed");

        // 检查 add 函数
        let add_func = match &validated_ast.declarations[0] {
            Declaration::Function {
                name, params, body, ..
            } if name == "add" => {
                assert_eq!(*name, "add"); // 函数名未变
                assert_eq!(params, &vec!["a.0".to_string(), "b.1".to_string()]); // 参数被重命名
                body.as_ref().unwrap()
            }
            _ => panic!("Expected add function"),
        };
        // 检查 add 函数的返回语句
        if let BlockItem::S(Statement::Return(expr)) = &add_func.blocks[0] {
            if let Expression::Binary { left, right, .. } = expr {
                assert_eq!(**left, Expression::Var("a.0".to_string()));
                assert_eq!(**right, Expression::Var("b.1".to_string()));
            } else {
                panic!("Expected binary expression in return");
            }
        } else {
            panic!("Expected return statement");
        }

        // 检查 main 函数
        let main_func = match &validated_ast.declarations[1] {
            Declaration::Function { name, body, .. } if name == "main" => body.as_ref().unwrap(),
            _ => panic!("Expected main function"),
        };
        // 检查 main 函数的返回语句
        if let BlockItem::S(Statement::Return(expr)) = &main_func.blocks[0] {
            if let Expression::FunctionCall { name, args } = expr {
                assert_eq!(*name, "add"); // 函数调用名未变
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], Expression::Constant(1));
                assert_eq!(args[1], Expression::Constant(2));
            } else {
                panic!("Expected function call in return");
            }
        } else {
            panic!("Expected return statement");
        }

        println!("--- Simple Function Call Test Passed! ---");
    }
    //测试 2：函数声明与定义分离
    #[test]
    fn test_function_declaration_and_definition() {
        let source_code = r#"
        int foo(void); // 声明

        int main(void) {
            return foo();
        }

        int foo(void) { // 定义
            return 42;
        }
    "#;

        let validated_ast = validate_source(source_code).expect("Validation should succeed");

        // 检查 AST 结构是否正确
        assert_eq!(validated_ast.declarations.len(), 3);

        // 1. foo 的声明
        match &validated_ast.declarations[0] {
            Declaration::Function { name, body, .. } if name == "foo" => {
                assert!(
                    body.is_none(),
                    "First foo should be a declaration without a body"
                );
            }
            _ => panic!("Expected foo declaration"),
        }

        // 2. main 的定义
        match &validated_ast.declarations[1] {
            Declaration::Function { name, body, .. } if name == "main" => {
                let main_body = body.as_ref().unwrap();
                if let BlockItem::S(Statement::Return(Expression::FunctionCall { name, .. })) =
                    &main_body.blocks[0]
                {
                    assert_eq!(*name, "foo"); // 确认调用了 foo
                } else {
                    panic!("Expected main to call foo");
                }
            }
            _ => panic!("Expected main definition"),
        }

        // 3. foo 的定义
        match &validated_ast.declarations[2] {
            Declaration::Function { name, body, .. } if name == "foo" => {
                assert!(
                    body.is_some(),
                    "Third declaration should be foo's definition with a body"
                );
            }
            _ => panic!("Expected foo definition"),
        }

        println!("--- Function Declaration/Definition Test Passed! ---");
    }
    //测试 3：检查错误情况 - 未声明的函数
    #[test]
    fn test_error_undeclared_function() {
        let source_code = r#"
        int main(void) {
            return undeclared_func();
        }
    "#;
        let result = validate_source(source_code);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Call to undeclared function 'undeclared_func'"));

        println!("--- Undeclared Function Error Test Passed! ---");
    }
    //测试 4：检查错误情况 - 重复的局部变量
    #[test]
    fn test_error_duplicate_local_variable() {
        let source_code = r#"
        int main(void) {
            int x = 1;
            int x = 2; // 非法
            return x;
        }
    "#;
        let result = validate_source(source_code);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Duplicate variable declaration for 'x'"));

        println!("--- Duplicate Local Variable Error Test Passed! ---");
    }
}
