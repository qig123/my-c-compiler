// src/backend/tacky_gen.rs

// 导入我们需要的数据结构
use crate::ir::tacky;
use crate::parser;

/// 负责将 C AST 转换为 TACKY IR 的生成器。
pub struct TackyGenerator {
    /// 用于生成唯一临时变量名的计数器。
    temp_counter: usize,
    /// 【新增】用于生成唯一标签名的计数器。
    label_counter: usize,
}

impl TackyGenerator {
    /// 创建一个新的 TackyGenerator 实例。
    pub fn new() -> Self {
        TackyGenerator {
            temp_counter: 0,
            label_counter: 0, // 【新增】初始化标签计数器
        }
    }

    /// 生成一个唯一的临时变量名，例如 "tmp.0", "tmp.1"。
    fn make_temporary(&mut self) -> String {
        let name = format!("tmp.{}", self.temp_counter);
        self.temp_counter += 1;
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
        }
    }

    /// 将一个语句 AST 节点转换为 TACKY 指令。 (无需修改)
    fn generate_tacky_for_statement(
        &mut self,
        stmt: &parser::Statement,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        // ... (保持不变)
        match stmt {
            parser::Statement::Return(exp) => {
                let return_val = self.generate_tacky_for_expression(exp, instructions)?;
                instructions.push(tacky::Instruction::Return(return_val));
                Ok(())
            }
        }
    }

    /// 将一个函数 AST 节点转换为 TACKY 函数。 (无需修改)
    fn generate_tacky_for_function(
        &mut self,
        func: &parser::Function,
    ) -> Result<tacky::Function, String> {
        // ... (保持不变)
        let mut instructions = Vec::new();
        self.generate_tacky_for_statement(&func.body, &mut instructions)?;
        Ok(tacky::Function {
            name: func.name.clone(),
            body: instructions,
        })
    }

    /// 主入口：将整个 C 程序 AST 转换为 TACKY 程序。 (无需修改)
    pub fn generate_tacky(&mut self, c_ast: parser::Program) -> Result<tacky::Program, String> {
        // ... (保持不变)
        let tacky_function = self.generate_tacky_for_function(&c_ast.function)?;
        Ok(tacky::Program {
            function: tacky_function,
        })
    }
}
