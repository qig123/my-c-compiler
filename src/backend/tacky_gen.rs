// src/backend/tacky_gen.rs

// 导入我们需要的数据结构
use crate::ir::tacky;
use crate::parser;

/// 负责将 C AST 转换为 TACKY IR 的生成器。
pub struct TackyGenerator {
    /// 用于生成唯一临时变量名的计数器。
    temp_counter: usize,
}

impl TackyGenerator {
    /// 创建一个新的 TackyGenerator 实例。
    pub fn new() -> Self {
        TackyGenerator { temp_counter: 0 }
    }

    /// 生成一个唯一的临时变量名，例如 "tmp.0", "tmp.1"。
    fn make_temporary(&mut self) -> String {
        let name = format!("tmp.{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    /// 将 parser AST 中的 UnaryOperator 转换为 tacky IR 中的 UnaryOperator。
    /// 这是一个简单的 1:1 映射。
    fn convert_unop(&self, op: &parser::UnaryOperator) -> tacky::UnaryOperator {
        match op {
            parser::UnaryOperator::Negation => tacky::UnaryOperator::Negate,
            parser::UnaryOperator::BitwiseComplement => tacky::UnaryOperator::Complement,
        }
    }

    /// 【核心】将一个表达式 AST 节点转换为 TACKY 指令列表。
    /// 这个函数有两个作用：
    /// 1. 将计算表达式所需的指令追加到 `instructions` 向量中。
    /// 2. 返回一个 `tacky::Val`，代表这个表达式最终计算结果存放的位置。
    fn generate_tacky_for_expression(
        &mut self,
        exp: &parser::Expression,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<tacky::Val, String> {
        match exp {
            // 规则 1: 常量表达式
            // - 不产生新指令。
            // - 直接返回一个 tacky::Val::Constant。
            parser::Expression::Constant(i) => Ok(tacky::Val::Constant(*i)),

            // 规则 2: 一元运算表达式
            parser::Expression::Unary {
                operator,
                expression,
            } => {
                // 1. 递归处理内部表达式，获取其结果存放的位置 (src)
                let src = self.generate_tacky_for_expression(expression, instructions)?;

                // 2. 为本次运算的结果创建一个新的临时变量 (dst)
                let dst_name = self.make_temporary();
                let dst = tacky::Val::Var(dst_name);

                // 3. 转换运算符
                let tacky_op = self.convert_unop(operator);

                // 4. 创建并追加 Unary 指令
                instructions.push(tacky::Instruction::Unary {
                    op: tacky_op,
                    src: src.clone(), // src 可能在别处还需要用，这里 clone
                    dst: dst.clone(), // dst 作为返回值，这里也 clone
                });

                // 5. 返回代表结果的临时变量
                Ok(dst)
            }
        }
    }

    /// 将一个语句 AST 节点转换为 TACKY 指令。
    fn generate_tacky_for_statement(
        &mut self,
        stmt: &parser::Statement,
        instructions: &mut Vec<tacky::Instruction>,
    ) -> Result<(), String> {
        match stmt {
            parser::Statement::Return(exp) => {
                // 1. 处理 return 语句中的表达式，获取其结果
                let return_val = self.generate_tacky_for_expression(exp, instructions)?;

                // 2. 创建并追加 Return 指令
                instructions.push(tacky::Instruction::Return(return_val));
                Ok(())
            }
        }
    }

    /// 将一个函数 AST 节点转换为 TACKY 函数。
    fn generate_tacky_for_function(
        &mut self,
        func: &parser::Function,
    ) -> Result<tacky::Function, String> {
        let mut instructions = Vec::new();

        // 处理函数体（目前只有一个语句）
        self.generate_tacky_for_statement(&func.body, &mut instructions)?;

        Ok(tacky::Function {
            name: func.name.clone(),
            body: instructions,
        })
    }

    /// 主入口：将整个 C 程序 AST 转换为 TACKY 程序。
    pub fn generate_tacky(&mut self, c_ast: parser::Program) -> Result<tacky::Program, String> {
        let tacky_function = self.generate_tacky_for_function(&c_ast.function)?;
        Ok(tacky::Program {
            function: tacky_function,
        })
    }
}
