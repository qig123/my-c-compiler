//! src/codegen.rs

use crate::parser;

// --- 1. 数据结构定义 (使用 Asm 前缀，并为 Register 添加类型) ---

#[derive(Debug)]
pub struct AsmProgram {
    pub function: AsmFunction,
}

#[derive(Debug)]
pub struct AsmFunction {
    pub name: String,
    pub instructions: Vec<AsmInstruction>,
}

#[derive(Debug)]
pub enum AsmInstruction {
    Mov { src: Operand, dst: Operand },
    Ret,
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterKind {
    EAX,
    // 未来可扩展
}

#[derive(Debug)]
pub enum Operand {
    Imm(i32),
    Reg(RegisterKind),
}

// --- 2. 代码生成器 (重命名并改进逻辑) ---

/// 从 C 的 AST 生成汇编的 AST
pub struct CodeGenerator {
    ast: parser::Program,
}

impl CodeGenerator {
    pub fn new(ast: parser::Program) -> Self {
        CodeGenerator { ast }
    }

    /// 生成整个程序的汇编表示
    pub fn generate(&self) -> Result<AsmProgram, String> {
        // 暂时仍用 String 作为错误类型
        let function = self.generate_function(&self.ast.function)?;
        Ok(AsmProgram { function })
    }

    /// 生成单个函数的汇编指令
    fn generate_function(&self, f: &parser::Function) -> Result<AsmFunction, String> {
        let mut instructions: Vec<AsmInstruction> = Vec::new();

        // 核心转换逻辑
        match &f.body {
            parser::Statement::Return(e) => {
                match e {
                    parser::Expression::Constant(i) => {
                        // `return <const>;` 对应 mov eax, <const>
                        instructions.push(AsmInstruction::Mov {
                            src: Operand::Imm(*i),
                            dst: Operand::Reg(RegisterKind::EAX), // 使用带类型的寄存器
                        });
                    }
                    _ => {
                        return Err(format!("unsupport type",));
                    }
                }
            }
        }

        instructions.push(AsmInstruction::Ret);

        Ok(AsmFunction {
            name: f.name.clone(),
            instructions,
        })
    }
}
