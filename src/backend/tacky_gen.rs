// src/backend/tacky_gen.rs

use crate::ast::checked::{self, Block};
// 导入我们需要的数据结构
use crate::common::UniqueIdGenerator;
use crate::ir::tacky;

const LOOP_START_PREFIX: &str = "loop_start";
const CONTINUE_LABEL_PREFIX: &str = "continue";
const BREAK_LABEL_PREFIX: &str = "break";
/// 负责将 C AST 转换为 TACKY IR 的生成器。
pub struct TackyGenerator<'a> {
    /// 用于生成唯一标签名的计数器。
    label_counter: usize,
    id_generator: &'a mut UniqueIdGenerator,
}

impl<'a> TackyGenerator<'a> {
    /// 创建一个新的 TackyGenerator 实例。
    pub fn new(id_generator: &'a mut UniqueIdGenerator) -> Self {
        TackyGenerator {
            id_generator,
            label_counter: 0, // 初始化标签计数器
        }
    }

    /// 生成一个唯一的临时变量名，例如 "tmp.0", "tmp.1"。
    fn make_temporary(&mut self) -> String {
        let id = self.id_generator.next();
        let name = format!("tmp.{}", id);
        name
    }

    /// 生成一个唯一的标签名，例如 "_L0", "_L1"。
    /// 使用下划线和字母开头，确保是合法的汇编标签。
    fn make_label_with_prefix(&mut self, prefix: &str) -> String {
        let label = format!("_{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }
    fn make_label_with_id(&mut self, prefix: &str, id: usize) -> String {
        let label = format!("_{}_{}", prefix, id);
        label
    }

    /// 将 checked AST 中的 UnaryOperator 转换为 tacky IR 中的 UnaryOperator。
    fn convert_unop(&self, op: &checked::UnaryOperator) -> tacky::UnaryOperator {
        match op {
            checked::UnaryOperator::Negate => tacky::UnaryOperator::Negate,
            checked::UnaryOperator::Complement => tacky::UnaryOperator::Complement,
            checked::UnaryOperator::Not => tacky::UnaryOperator::Not,
        }
    }

    /// 将 checked AST 中的 BinaryOperator 转换为 tacky IR 中的 BinaryOperator。
    /// 注意：这个函数只处理非短路的二元运算符。
    fn convert_binaryop(
        &self,
        op: &checked::BinaryOperator,
    ) -> Result<tacky::BinaryOperator, String> {
        match op {
            checked::BinaryOperator::Add => Ok(tacky::BinaryOperator::Add),
            checked::BinaryOperator::Subtract => Ok(tacky::BinaryOperator::Subtract),
            checked::BinaryOperator::Multiply => Ok(tacky::BinaryOperator::Multiply),
            checked::BinaryOperator::Divide => Ok(tacky::BinaryOperator::Divide),
            checked::BinaryOperator::Remainder => Ok(tacky::BinaryOperator::Remainder),
            checked::BinaryOperator::Equal => Ok(tacky::BinaryOperator::Equal),
            checked::BinaryOperator::NotEqual => Ok(tacky::BinaryOperator::NotEqual),
            checked::BinaryOperator::LessThan => Ok(tacky::BinaryOperator::LessThan),
            checked::BinaryOperator::LessOrEqual => Ok(tacky::BinaryOperator::LessOrEqual),
            checked::BinaryOperator::GreaterThan => Ok(tacky::BinaryOperator::GreaterThan),
            checked::BinaryOperator::GreaterOrEqual => Ok(tacky::BinaryOperator::GreaterEqual),
            // And 和 Or 是特殊情况，不应通过此函数处理
            checked::BinaryOperator::And | checked::BinaryOperator::Or => Err(
                "Logical AND/OR should be handled separately and not converted directly."
                    .to_string(),
            ),
        }
    }
    fn generate_tacky_for_block(
        &mut self,
        block: &checked::Block,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        for item in &block.blocks {
            self.generate_tacky_for_block_item(item, instructions)?;
        }
        Ok(())
    }

    /// 将一个表达式 AST 节点转换为 TACKY 指令列表。
    fn generate_tacky_for_expression(
        &mut self,
        exp: &checked::Expression,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<tacky::Val, String> {
        match exp {
            checked::Expression::Var(name) => Ok(tacky::Val::Var(name.clone())),
            checked::Expression::Assign { left, right } => {
                let rhs_val = self.generate_tacky_for_expression(right, instructions)?;

                if let checked::Expression::Var(var_name) = &**left {
                    let dst_var = tacky::Val::Var(var_name.clone());
                    instructions.push(tacky::Instruction::Copy {
                        src: rhs_val.clone(),
                        dst: dst_var,
                    });
                    Ok(rhs_val)
                } else {
                    Err("Invalid left-hand side in assignment.".to_string())
                }
            }
            checked::Expression::Constant(i) => Ok(tacky::Val::Constant(*i)),
            checked::Expression::Unary {
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
            checked::Expression::Binary {
                operator,
                left,
                right,
            } => match operator {
                checked::BinaryOperator::And => {
                    let result_var_name = self.make_temporary();
                    let result_var = tacky::Val::Var(result_var_name);
                    let false_label = self.make_label_with_prefix("and_false");
                    let end_label = self.make_label_with_prefix("and_end");
                    let v1 = self.generate_tacky_for_expression(left, instructions)?;
                    instructions.push(tacky::Instruction::JumpIfZero {
                        condition: v1,
                        target: false_label.clone(),
                    });
                    let v2 = self.generate_tacky_for_expression(right, instructions)?;
                    instructions.push(tacky::Instruction::JumpIfZero {
                        condition: v2,
                        target: false_label.clone(),
                    });
                    instructions.push(tacky::Instruction::Copy {
                        src: tacky::Val::Constant(1),
                        dst: result_var.clone(),
                    });
                    instructions.push(tacky::Instruction::Jump(end_label.clone()));
                    instructions.push(tacky::Instruction::Label(false_label));
                    instructions.push(tacky::Instruction::Copy {
                        src: tacky::Val::Constant(0),
                        dst: result_var.clone(),
                    });
                    instructions.push(tacky::Instruction::Label(end_label));
                    Ok(result_var)
                }
                checked::BinaryOperator::Or => {
                    let result_var_name = self.make_temporary();
                    let result_var = tacky::Val::Var(result_var_name);
                    let true_label = self.make_label_with_prefix("or_true");
                    let end_label = self.make_label_with_prefix("or_end");
                    let v1 = self.generate_tacky_for_expression(left, instructions)?;
                    instructions.push(tacky::Instruction::JumpIfNotZero {
                        condition: v1,
                        target: true_label.clone(),
                    });
                    let v2 = self.generate_tacky_for_expression(right, instructions)?;
                    instructions.push(tacky::Instruction::JumpIfNotZero {
                        condition: v2,
                        target: true_label.clone(),
                    });
                    instructions.push(tacky::Instruction::Copy {
                        src: tacky::Val::Constant(0),
                        dst: result_var.clone(),
                    });
                    instructions.push(tacky::Instruction::Jump(end_label.clone()));
                    instructions.push(tacky::Instruction::Label(true_label));
                    instructions.push(tacky::Instruction::Copy {
                        src: tacky::Val::Constant(1),
                        dst: result_var.clone(),
                    });
                    instructions.push(tacky::Instruction::Label(end_label));
                    Ok(result_var)
                }
                _ => {
                    let src1 = self.generate_tacky_for_expression(left, instructions)?;
                    let src2 = self.generate_tacky_for_expression(right, instructions)?;
                    let dst_name = self.make_temporary();
                    let dst = tacky::Val::Var(dst_name);
                    let tacky_op = self.convert_binaryop(operator)?;
                    instructions.push(tacky::Instruction::Binary {
                        op: tacky_op,
                        src1: src1.clone(),
                        src2: src2.clone(),
                        dst: dst.clone(),
                    });
                    Ok(dst)
                }
            },
            checked::Expression::Conditional {
                condition,
                left,
                right,
            } => {
                let result_var = tacky::Val::Var(self.make_temporary());
                let else_label = self.make_label_with_prefix("cond_else");
                let end_label = self.make_label_with_prefix("cond_end");
                let cond_val = self.generate_tacky_for_expression(condition, instructions)?;
                instructions.push(tacky::Instruction::JumpIfZero {
                    condition: cond_val,
                    target: else_label.clone(),
                });
                let then_val = self.generate_tacky_for_expression(left, instructions)?;
                instructions.push(tacky::Instruction::Copy {
                    src: then_val,
                    dst: result_var.clone(),
                });
                instructions.push(tacky::Instruction::Jump(end_label.clone()));
                instructions.push(tacky::Instruction::Label(else_label));
                let else_val = self.generate_tacky_for_expression(right, instructions)?;
                instructions.push(tacky::Instruction::Copy {
                    src: else_val,
                    dst: result_var.clone(),
                });
                instructions.push(tacky::Instruction::Label(end_label));
                Ok(result_var)
            }
            // =========================================================
            //  【核心修改点】处理函数调用
            // =========================================================
            checked::Expression::FunctionCall { name, args } => {
                // 1. 为每个参数表达式生成指令，并收集结果 Val
                let mut arg_vals = Vec::new();
                for arg_expr in args {
                    let param_val = self.generate_tacky_for_expression(arg_expr, instructions)?;
                    arg_vals.push(param_val);
                }

                // 2. 创建一个新的临时变量来存储函数的返回值。
                //    这就是 FunCall 指令的 `dst`。
                let result_dst_name = self.make_temporary();
                let result_dst = tacky::Val::Var(result_dst_name);

                // 3. 生成 FunCall 指令
                instructions.push(tacky::Instruction::FunCall {
                    name: name.clone(),
                    args: arg_vals,
                    dst: result_dst.clone(), // 使用我们创建的临时变量作为目标
                });

                // 4. 整个函数调用表达式的值，就是存储返回值的那个临时变量。
                Ok(result_dst)
            }
        }
    }

    /// 为单个块项目生成 TACKY 指令
    fn generate_tacky_for_block_item(
        &mut self,
        item: &checked::BlockItem,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        match item {
            checked::BlockItem::D(declaration) => {
                match declaration {
                    // 函数声明/定义不会出现在函数体内部（标准C），
                    // 顶层函数定义在 generate_tacky 中单独处理。
                    checked::Declaration::Function { .. } => {
                        // 此处无需处理
                    }
                    checked::Declaration::Variable { name, init } => {
                        // 只处理有初始化器的声明
                        if let Some(init_expr) = init {
                            // 这等同于一个赋值语句: `var = init_expr`
                            let rhs_val =
                                self.generate_tacky_for_expression(init_expr, instructions)?;
                            let dst_var = tacky::Val::Var(name.clone());
                            instructions.push(tacky::Instruction::Copy {
                                src: rhs_val,
                                dst: dst_var,
                            });
                        }
                    }
                }
                // 没有初始化器的声明 (e.g., "int a;") 在 TACKY 层面被忽略
                Ok(())
            }
            checked::BlockItem::S(statement) => {
                self.generate_tacky_for_statement(statement, instructions)
            }
        }
    }

    /// 将一个语句 AST 节点转换为 TACKY 指令。
    fn generate_tacky_for_statement(
        &mut self,
        stmt: &checked::Statement,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        match stmt {
            checked::Statement::Return(exp) => {
                let return_val = self.generate_tacky_for_expression(exp, instructions)?;
                instructions.push(tacky::Instruction::Return(return_val));
                Ok(())
            }
            checked::Statement::Expression(exp) => {
                // 我们需要为表达式生成指令，但可以忽略其结果。
                self.generate_tacky_for_expression(exp, instructions)?;
                Ok(())
            }
            checked::Statement::Empty => {
                // 空语句不产生任何 TACKY 指令
                Ok(())
            }
            checked::Statement::If {
                condition,
                then_stat,
                else_stat,
            } => {
                match else_stat {
                    Some(else_s) => {
                        let else_label = self.make_label_with_prefix("else");
                        let end_label = self.make_label_with_prefix("if_end");
                        let cond_val =
                            self.generate_tacky_for_expression(condition, instructions)?;
                        instructions.push(tacky::Instruction::JumpIfZero {
                            condition: cond_val,
                            target: else_label.clone(),
                        });
                        self.generate_tacky_for_statement(then_stat, instructions)?;
                        instructions.push(tacky::Instruction::Jump(end_label.clone()));
                        instructions.push(tacky::Instruction::Label(else_label));
                        self.generate_tacky_for_statement(else_s, instructions)?;
                        instructions.push(tacky::Instruction::Label(end_label));
                    }
                    None => {
                        let end_label = self.make_label_with_prefix("if_end");
                        let cond_val =
                            self.generate_tacky_for_expression(condition, instructions)?;
                        instructions.push(tacky::Instruction::JumpIfZero {
                            condition: cond_val,
                            target: end_label.clone(),
                        });
                        self.generate_tacky_for_statement(then_stat, instructions)?;
                        instructions.push(tacky::Instruction::Label(end_label));
                    }
                }
                Ok(())
            }
            checked::Statement::Compound(b) => self.generate_tacky_for_block(b, instructions),
            &checked::Statement::Break { target_id } => {
                instructions.push(tacky::Instruction::Jump(
                    self.make_label_with_id(BREAK_LABEL_PREFIX, target_id),
                ));
                Ok(())
            }
            &checked::Statement::Continue { target_id } => {
                instructions.push(tacky::Instruction::Jump(
                    self.make_label_with_id(CONTINUE_LABEL_PREFIX, target_id),
                ));
                Ok(())
            }

            &checked::Statement::DoWhile {
                ref body,
                ref condition,
                id,
            } => {
                let start_label = self.make_label_with_id(LOOP_START_PREFIX, id);
                let continue_label = self.make_label_with_id(CONTINUE_LABEL_PREFIX, id);
                let break_label = self.make_label_with_id(BREAK_LABEL_PREFIX, id);
                instructions.push(tacky::Instruction::Label(start_label.clone()));
                self.generate_tacky_for_statement(&*body, instructions)?;
                instructions.push(tacky::Instruction::Label(continue_label));
                let cond_val = self.generate_tacky_for_expression(&condition, instructions)?;
                instructions.push(tacky::Instruction::JumpIfNotZero {
                    condition: cond_val,
                    target: start_label,
                });
                instructions.push(tacky::Instruction::Label(break_label));
                Ok(())
            }
            &checked::Statement::While {
                ref condition,
                ref body,
                id,
            } => {
                let continue_label = self.make_label_with_id(CONTINUE_LABEL_PREFIX, id);
                let break_label = self.make_label_with_id(BREAK_LABEL_PREFIX, id);
                instructions.push(tacky::Instruction::Label(continue_label.clone()));
                let cond_val = self.generate_tacky_for_expression(&condition, instructions)?;
                instructions.push(tacky::Instruction::JumpIfZero {
                    condition: cond_val,
                    target: break_label.clone(),
                });
                self.generate_tacky_for_statement(&*body, instructions)?;
                instructions.push(tacky::Instruction::Jump(continue_label));
                instructions.push(tacky::Instruction::Label(break_label));

                Ok(())
            }
            &checked::Statement::For {
                ref init,
                ref condition,
                ref post,
                ref body,
                id,
            } => {
                let start_label = self.make_label_with_id(LOOP_START_PREFIX, id);
                let continue_label = self.make_label_with_id(CONTINUE_LABEL_PREFIX, id);
                let break_label = self.make_label_with_id(BREAK_LABEL_PREFIX, id);
                if let Some(init_item) = init {
                    self.generate_tacky_for_block_item(init_item, instructions)?;
                }
                instructions.push(tacky::Instruction::Label(start_label.clone()));
                if let Some(cond_expr) = condition {
                    let cond_val = self.generate_tacky_for_expression(cond_expr, instructions)?;
                    instructions.push(tacky::Instruction::JumpIfZero {
                        condition: cond_val,
                        target: break_label.clone(),
                    });
                }
                self.generate_tacky_for_statement(body, instructions)?;
                instructions.push(tacky::Instruction::Label(continue_label));
                if let Some(post_expr) = post {
                    self.generate_tacky_for_expression(post_expr, instructions)?;
                }
                instructions.push(tacky::Instruction::Jump(start_label));
                instructions.push(tacky::Instruction::Label(break_label));
                Ok(())
            }
        }
    }
    /// 将一个函数 AST 节点转换为 TACKY 函数。
    fn generate_tacky_for_function(
        &mut self,
        name: String,
        params: Vec<String>,
        body: Option<Block>,
    ) -> Result<Option<tacky::Function>, String> {
        // 只处理函数定义（有函数体），忽略函数声明
        if let Some(b) = body {
            let mut instructions = Vec::new();
            self.generate_tacky_for_block(&b, &mut instructions)?;

            // 确保函数总有返回值
            if !instructions
                .last()
                .map_or(false, |inst| matches!(inst, tacky::Instruction::Return(_)))
            {
                instructions.push(tacky::Instruction::Return(tacky::Val::Constant(0)));
            }
            Ok(Some(tacky::Function {
                name,
                params,
                body: instructions,
            }))
        } else {
            // 函数声明（无函数体）在 TACKY 生成阶段被丢弃
            Ok(None)
        }
    }

    /// 主入口：将整个 C 程序 AST 转换为 TACKY 程序。
    pub fn generate_tacky(&mut self, c_ast: checked::Program) -> Result<tacky::Program, String> {
        let mut funs = Vec::new();
        for d in c_ast.declarations {
            match d {
                checked::Declaration::Function { name, params, body } => {
                    // generate_tacky_for_function 会处理 body 是否为 Some
                    if let Some(tacky_function) =
                        self.generate_tacky_for_function(name, params, body)?
                    {
                        funs.push(tacky_function);
                    }
                }
                // 顶层变量声明在 TACKY 阶段被忽略
                checked::Declaration::Variable { .. } => {}
            }
        }
        Ok(tacky::Program { functions: funs })
    }
}
