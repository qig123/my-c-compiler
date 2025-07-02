// src/backend/tacky_gen.rs

use crate::common::UniqueIdGenerator;
// 导入我们需要的数据结构
use crate::ir::tacky;
use crate::parser::{self, BlockItem, Expression, Statement};

/// 负责将 C AST 转换为 TACKY IR 的生成器。
pub struct TackyGenerator<'a> {
    /// 【新增】用于生成唯一标签名的计数器。
    label_counter: usize,
    id_generator: &'a mut UniqueIdGenerator,
}

impl<'a> TackyGenerator<'a> {
    /// 创建一个新的 TackyGenerator 实例。
    pub fn new(id_generator: &'a mut UniqueIdGenerator) -> Self {
        TackyGenerator {
            id_generator,
            label_counter: 0, // 【新增】初始化标签计数器
        }
    }

    /// 生成一个唯一的临时变量名，例如 "tmp.0", "tmp.1"。
    fn make_temporary(&mut self) -> String {
        let id = self.id_generator.next();
        let name = format!("tmp.{}", id);
        name
    }

    /// 【新增】生成一个唯一的标签名，例如 "_L0", "_L1"。
    /// 使用下划线和字母开头，确保是合法的汇编标签。
    fn make_label_with_prefix(&mut self, prefix: &str) -> String {
        let label = format!("_{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /// 将 parser AST 中的 UnaryOperator 转换为 tacky IR 中的 UnaryOperator。
    fn convert_unop(&self, op: &parser::UnaryOperator) -> tacky::UnaryOperator {
        match op {
            parser::UnaryOperator::Negate => tacky::UnaryOperator::Negate,
            parser::UnaryOperator::Complement => tacky::UnaryOperator::Complement,
            parser::UnaryOperator::Not => tacky::UnaryOperator::Not,
        }
    }

    /// 【修改】将 parser AST 中的 BinaryOperator 转换为 tacky IR 中的 BinaryOperator。
    /// 注意：这个函数只处理非短路的二元运算符。
    fn convert_binaryop(
        &self,
        op: &parser::BinaryOperator,
    ) -> Result<tacky::BinaryOperator, String> {
        match op {
            parser::BinaryOperator::Add => Ok(tacky::BinaryOperator::Add),
            parser::BinaryOperator::Subtract => Ok(tacky::BinaryOperator::Subtract),
            parser::BinaryOperator::Multiply => Ok(tacky::BinaryOperator::Multiply),
            parser::BinaryOperator::Divide => Ok(tacky::BinaryOperator::Divide),
            parser::BinaryOperator::Remainder => Ok(tacky::BinaryOperator::Remainder),
            // 新增的关系运算符
            parser::BinaryOperator::Equal => Ok(tacky::BinaryOperator::Equal),
            parser::BinaryOperator::NotEqual => Ok(tacky::BinaryOperator::NotEqual),
            parser::BinaryOperator::LessThan => Ok(tacky::BinaryOperator::LessThan),
            parser::BinaryOperator::LessOrEqual => Ok(tacky::BinaryOperator::LessOrEqual),
            parser::BinaryOperator::GreaterThan => Ok(tacky::BinaryOperator::GreaterThan),
            parser::BinaryOperator::GreaterOrEqual => Ok(tacky::BinaryOperator::GreaterEqual),
            // And 和 Or 是特殊情况，不应通过此函数处理
            parser::BinaryOperator::And | parser::BinaryOperator::Or => Err(
                "Logical AND/OR should be handled separately and not converted directly."
                    .to_string(),
            ),
        }
    }

    /// 【核心修改】将一个表达式 AST 节点转换为 TACKY 指令列表。
    fn generate_tacky_for_expression(
        &mut self,
        exp: &parser::Expression,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<tacky::Val, String> {
        match exp {
            // 经过语义分析后，变量名已经是唯一的了。
            Expression::Var(name) => Ok(tacky::Val::Var(name.clone())),
            Expression::Assign { left, right } => {
                // 赋值表达式的结果是右侧的值
                // 首先计算右侧表达式的值
                let rhs_val = self.generate_tacky_for_expression(right, instructions)?;

                // 左侧必须是变量（这在语义分析阶段已保证）
                if let Expression::Var(var_name) = &**left {
                    let dst_var = tacky::Val::Var(var_name.clone());
                    // 生成 Copy 指令
                    instructions.push(tacky::Instruction::Copy {
                        src: rhs_val.clone(),
                        dst: dst_var,
                    });
                    // 赋值表达式的值就是赋的值
                    Ok(rhs_val)
                } else {
                    // 理论上语义分析已经阻止了这种情况
                    Err("Invalid left-hand side in assignment.".to_string())
                }
            }

            parser::Expression::Constant(i) => Ok(tacky::Val::Constant(*i)),
            parser::Expression::Unary {
                operator,
                expression,
            } => {
                let src = self.generate_tacky_for_expression(expression, instructions)?;
                let dst_name = self.make_temporary();
                let dst = tacky::Val::Var(dst_name);
                let tacky_op = self.convert_unop(operator);
                instructions.push(tacky::Instruction::Unary {
                    op: tacky_op,
                    src: src.clone(),
                    dst: dst.clone(),
                });
                Ok(dst)
            }
            // 【修改】对二元运算符进行分支处理
            parser::Expression::Binary {
                operator,
                left,
                right,
            } => {
                match operator {
                    // --- Case 1: 逻辑与 (&&) ---
                    parser::BinaryOperator::And => {
                        // 创建最终存放结果的临时变量
                        let result_var_name = self.make_temporary();
                        let result_var = tacky::Val::Var(result_var_name);

                        // 创建两个需要的标签
                        let false_label = self.make_label_with_prefix("and_false");
                        let end_label = self.make_label_with_prefix("and_end");

                        // 1. 计算左侧表达式 e1
                        let v1 = self.generate_tacky_for_expression(left, instructions)?;
                        // 2. 如果 e1 为 0，短路，跳转到 false 分支
                        instructions.push(tacky::Instruction::JumpIfZero {
                            condition: v1,
                            target: false_label.clone(),
                        });

                        // 3. 计算右侧表达式 e2
                        let v2 = self.generate_tacky_for_expression(right, instructions)?;
                        // 4. 如果 e2 为 0，跳转到 false 分支
                        instructions.push(tacky::Instruction::JumpIfZero {
                            condition: v2,
                            target: false_label.clone(),
                        });

                        // 5. (True 分支) 如果代码执行到这里，说明 e1 和 e2 都非 0
                        instructions.push(tacky::Instruction::Copy {
                            src: tacky::Val::Constant(1),
                            dst: result_var.clone(),
                        });
                        // 6. 跳转到表达式末尾，跳过 false 分支的代码
                        instructions.push(tacky::Instruction::Jump(end_label.clone()));

                        // 7. (False 分支)
                        instructions.push(tacky::Instruction::Label(false_label));
                        instructions.push(tacky::Instruction::Copy {
                            src: tacky::Val::Constant(0),
                            dst: result_var.clone(),
                        });

                        // 8. 表达式结束的标签
                        instructions.push(tacky::Instruction::Label(end_label));

                        Ok(result_var)
                    }

                    // --- Case 2: 逻辑或 (||) ---
                    // 这个逻辑留给你自己实现，但结构与 && 非常相似
                    parser::BinaryOperator::Or => {
                        let result_var_name = self.make_temporary();
                        let result_var = tacky::Val::Var(result_var_name);

                        let true_label = self.make_label_with_prefix("or_true");
                        let end_label = self.make_label_with_prefix("or_end");

                        // 1. 计算 e1
                        let v1 = self.generate_tacky_for_expression(left, instructions)?;
                        // 2. 如果 e1 非 0，短路，跳转到 true 分支
                        instructions.push(tacky::Instruction::JumpIfNotZero {
                            condition: v1,
                            target: true_label.clone(),
                        });

                        // 3. 计算 e2
                        let v2 = self.generate_tacky_for_expression(right, instructions)?;
                        // 4. 如果 e2 非 0，跳转到 true 分支
                        instructions.push(tacky::Instruction::JumpIfNotZero {
                            condition: v2,
                            target: true_label.clone(),
                        });

                        // 5. (False 分支) 如果代码执行到这里，说明 e1 和 e2 都为 0
                        instructions.push(tacky::Instruction::Copy {
                            src: tacky::Val::Constant(0),
                            dst: result_var.clone(),
                        });
                        instructions.push(tacky::Instruction::Jump(end_label.clone()));

                        // 6. (True 分支)
                        instructions.push(tacky::Instruction::Label(true_label));
                        instructions.push(tacky::Instruction::Copy {
                            src: tacky::Val::Constant(1),
                            dst: result_var.clone(),
                        });

                        // 7. 表达式结束的标签
                        instructions.push(tacky::Instruction::Label(end_label));

                        Ok(result_var)
                    }

                    // --- Case 3: 其他所有标准二元运算符 ---
                    _ => {
                        // 递归处理左右子表达式
                        let src1 = self.generate_tacky_for_expression(left, instructions)?;
                        let src2 = self.generate_tacky_for_expression(right, instructions)?;

                        // 为结果创建临时变量
                        let dst_name = self.make_temporary();
                        let dst = tacky::Val::Var(dst_name);

                        // 转换运算符
                        let tacky_op = self.convert_binaryop(operator)?;

                        // 生成 Binary 指令
                        instructions.push(tacky::Instruction::Binary {
                            op: tacky_op,
                            src1: src1.clone(),
                            src2: src2.clone(),
                            dst: dst.clone(),
                        });

                        Ok(dst)
                    }
                }
            }
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                // 为最终结果创建一个临时变量
                let result_var = tacky::Val::Var(self.make_temporary());

                // 创建两个标签：一个用于 else 分支，一个用于结束
                let else_label = self.make_label_with_prefix("cond_else");
                let end_label = self.make_label_with_prefix("cond_end");

                // 1. 计算条件表达式
                let cond_val = self.generate_tacky_for_expression(condition, instructions)?;

                // 2. 如果条件为 0，跳转到 else 分支
                instructions.push(tacky::Instruction::JumpIfZero {
                    condition: cond_val,
                    target: else_label.clone(),
                });

                // 3. (Then 分支) 计算 then 表达式，并将结果存入 result_var
                let then_val = self.generate_tacky_for_expression(left, instructions)?;
                instructions.push(tacky::Instruction::Copy {
                    src: then_val,
                    dst: result_var.clone(),
                });
                // 4. 无条件跳转到末尾，跳过 else 分支
                instructions.push(tacky::Instruction::Jump(end_label.clone()));

                // 5. (Else 分支) 放置 else 标签
                instructions.push(tacky::Instruction::Label(else_label));
                // 6. 计算 else 表达式，并将结果存入 result_var
                let else_val = self.generate_tacky_for_expression(right, instructions)?;
                instructions.push(tacky::Instruction::Copy {
                    src: else_val,
                    dst: result_var.clone(),
                });

                // 7. 放置结束标签
                instructions.push(tacky::Instruction::Label(end_label));

                // 整个条件表达式的值就是 result_var
                Ok(result_var)
            }
        }
    }

    /// 【新增】为单个块项目生成 TACKY 指令
    fn generate_tacky_for_block_item(
        &mut self,
        item: &BlockItem,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        match item {
            // 如果是声明
            BlockItem::D(declaration) => {
                // 只处理有初始化器的声明
                if let Some(init_expr) = &declaration.init {
                    // 这等同于一个赋值语句: `var = init_expr`
                    let rhs_val = self.generate_tacky_for_expression(init_expr, instructions)?;
                    let dst_var = tacky::Val::Var(declaration.name.clone());
                    instructions.push(tacky::Instruction::Copy {
                        src: rhs_val,
                        dst: dst_var,
                    });
                }
                // 没有初始化器的声明 (e.g., "int a;") 在 TACKY 层面被忽略
                Ok(())
            }
            // 如果是语句
            BlockItem::S(statement) => self.generate_tacky_for_statement(statement, instructions),
        }
    }

    /// 【修改】将一个语句 AST 节点转换为 TACKY 指令。
    fn generate_tacky_for_statement(
        &mut self,
        stmt: &parser::Statement,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        match stmt {
            Statement::Return(exp) => {
                let return_val = self.generate_tacky_for_expression(exp, instructions)?;
                instructions.push(tacky::Instruction::Return(return_val));
                Ok(())
            }
            // 【新增】处理表达式语句
            Statement::Expression(exp) => {
                // 我们需要为表达式生成指令，但可以忽略其结果。
                // 例如，对于 "a * 2;"，我们仍然计算它，但结果不用于任何地方。
                self.generate_tacky_for_expression(exp, instructions)?;
                Ok(())
            }
            // 【新增】处理空语句
            Statement::Empty => {
                // 空语句不产生任何 TACKY 指令
                Ok(())
            }
            Statement::If {
                condition,
                then_stat,
                else_stat,
            } => {
                // 根据是否存在 else 分支来决定逻辑
                match else_stat {
                    // Case 1: if-else 语句
                    Some(else_s) => {
                        let else_label = self.make_label_with_prefix("else");
                        let end_label = self.make_label_with_prefix("if_end");

                        // 1. 计算条件
                        let cond_val =
                            self.generate_tacky_for_expression(condition, instructions)?;
                        // 2. 如果为 0，跳转到 else_label
                        instructions.push(tacky::Instruction::JumpIfZero {
                            condition: cond_val,
                            target: else_label.clone(),
                        });

                        // 3. (Then 分支) 生成 then 语句的指令
                        self.generate_tacky_for_statement(then_stat, instructions)?;
                        // 4. 执行完 then 后，无条件跳转到末尾
                        instructions.push(tacky::Instruction::Jump(end_label.clone()));

                        // 5. (Else 分支) 放置 else 标签
                        instructions.push(tacky::Instruction::Label(else_label));
                        self.generate_tacky_for_statement(else_s, instructions)?;

                        // 6. 放置结束标签
                        instructions.push(tacky::Instruction::Label(end_label));
                    }
                    // Case 2: 只有 if，没有 else
                    None => {
                        let end_label = self.make_label_with_prefix("if_end");

                        // 1. 计算条件
                        let cond_val =
                            self.generate_tacky_for_expression(condition, instructions)?;
                        // 2. 如果为 0，直接跳过 then 分支，跳转到末尾
                        instructions.push(tacky::Instruction::JumpIfZero {
                            condition: cond_val,
                            target: end_label.clone(),
                        });

                        // 3. (Then 分支) 生成 then 语句的指令
                        self.generate_tacky_for_statement(then_stat, instructions)?;

                        // 4. 放置结束标签
                        instructions.push(tacky::Instruction::Label(end_label));
                    }
                }
                Ok(())
            }
            _ => {
                panic!()
            }
        }
    }
    /// 将一个函数 AST 节点转换为 TACKY 函数。 (无需修改)
    fn generate_tacky_for_function(
        &mut self,
        func: &parser::Function,
    ) -> Result<tacky::Function, String> {
        let mut instructions = Vec::new();
        // 1. 遍历函数体中的所有块项目，并依次生成指令
        for item in &func.body.blocks {
            self.generate_tacky_for_block_item(item, &mut instructions)?;
        }

        // 2. 【关键】处理函数末尾没有 return 的情况
        // 检查最后一条指令是否是 Return
        if !matches!(instructions.last(), Some(tacky::Instruction::Return(_))) {
            // 如果不是，或者函数体为空，则隐式添加 "return 0;"
            instructions.push(tacky::Instruction::Return(tacky::Val::Constant(0)));
        }

        Ok(tacky::Function {
            name: func.name.clone(),
            body: instructions,
        })
    }

    /// 主入口：将整个 C 程序 AST 转换为 TACKY 程序。 (无需修改)
    pub fn generate_tacky(&mut self, c_ast: parser::Program) -> Result<tacky::Program, String> {
        let tacky_function = self.generate_tacky_for_function(&c_ast.function)?;
        Ok(tacky::Program {
            function: tacky_function,
        })
    }
}
