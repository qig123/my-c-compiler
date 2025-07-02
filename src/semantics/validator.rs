//! src/semantics/validator.rs

// 【修改】 在 use 语句中明确列出所有需要的类型，包括 Empty
use crate::{
    common::UniqueIdGenerator,
    parser::{BlockItem, Declaration, Expression, Function, Program, Statement},
};
use std::collections::HashMap;

/// The Validator performs semantic analysis, specifically variable resolution.
/// It walks the AST, checking for errors like undeclared variables or
/// duplicate declarations, and transforms the AST to use unique variable names.
pub struct Validator<'a> {
    /// Maps user-defined variable names in the current scope to unique names.
    variable_map: HashMap<String, String>,
    id_generator: &'a mut UniqueIdGenerator,
}

impl<'a> Validator<'a> {
    /// Creates a new Validator.
    pub fn new(id_generator: &'a mut UniqueIdGenerator) -> Self {
        Validator {
            variable_map: HashMap::new(),
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
        let mut validated_body = Vec::new();
        for item in function.body {
            validated_body.push(self.validate_block_item(item)?);
        }

        Ok(Function {
            name: function.name,
            body: validated_body,
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
        // Rule: A variable cannot be declared more than once in the same scope.
        if self.variable_map.contains_key(&decl.name) {
            return Err(format!(
                "Duplicate variable declaration for '{}'",
                decl.name
            ));
        }

        let unique_name = self.generate_unique_name(&decl.name);
        // We need to clone decl.name because it's moved into insert, but we might need it for the unique name.
        self.variable_map
            .insert(decl.name.clone(), unique_name.clone());

        // Validate the initializer expression, if it exists.
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
        }
    }

    fn validate_expression(&mut self, expr: Expression) -> Result<Expression, String> {
        match expr {
            Expression::Constant(c) => Ok(Expression::Constant(c)),

            Expression::Var(name) => {
                if let Some(unique_name) = self.variable_map.get(&name) {
                    Ok(Expression::Var(unique_name.clone()))
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
        }
    }
}
