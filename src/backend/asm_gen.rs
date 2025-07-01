// src/backend/asm_gen.rs

use crate::ir::{assembly, tacky};
use std::collections::HashMap;

/// 负责将 TACKY IR 转换为最终的汇编 AST。
/// 这个过程分为三个阶段。
pub struct AsmGenerator {
    // 这个结构体现在是无状态的，因为所有状态都在方法调用之间传递。
}

impl AsmGenerator {
    pub fn new() -> Self {
        AsmGenerator {}
    }

    /// 主入口：将 TACKY 程序转换为汇编程序。
    pub fn generate_assembly(
        &mut self,
        tacky_program: tacky::Program,
    ) -> Result<assembly::Program, String> {
        // --- PASS 1: TACKY -> Assembly with Pseudoregisters ---
        let mut asm_func = self.convert_tacky_to_asm_pass1(&tacky_program.function)?;

        // --- PASS 2: Replace Pseudoregisters -> Stack slots ---
        let stack_bytes_needed = self.replace_pseudo_with_stack_pass2(&mut asm_func)?;

        // --- PASS 3: Fix up instructions ---
        self.fixup_instructions_pass3(&mut asm_func, stack_bytes_needed);

        Ok(assembly::Program { function: asm_func })
    }

    // =================================================================
    // PASS 1: Convert TACKY to Assembly with Pseudoregisters
    // =================================================================

    fn convert_tacky_to_asm_pass1(
        &self,
        tacky_func: &tacky::Function,
    ) -> Result<assembly::Function, String> {
        let mut instructions = Vec::new();
        for tacky_inst in &tacky_func.body {
            match tacky_inst {
                tacky::Instruction::Return(val) => {
                    instructions.push(assembly::Instruction::Mov {
                        src: self.convert_tacky_val(val),
                        dst: assembly::Operand::Reg(assembly::Register::AX),
                    });
                    instructions.push(assembly::Instruction::Ret);
                }
                tacky::Instruction::Unary { op, src, dst } => {
                    let asm_op = match op {
                        tacky::UnaryOperator::Complement => assembly::UnaryOperator::Not,
                        tacky::UnaryOperator::Negate => assembly::UnaryOperator::Neg,
                    };
                    instructions.push(assembly::Instruction::Mov {
                        src: self.convert_tacky_val(src),
                        dst: self.convert_tacky_val(dst),
                    });
                    instructions.push(assembly::Instruction::Unary {
                        op: asm_op,
                        operand: self.convert_tacky_val(dst),
                    });
                }
            }
        }

        Ok(assembly::Function {
            name: tacky_func.name.clone(),
            instructions,
        })
    }

    /// 辅助函数：将 tacky::Val 转换为 assembly::Operand。
    fn convert_tacky_val(&self, val: &tacky::Val) -> assembly::Operand {
        match val {
            tacky::Val::Constant(i) => assembly::Operand::Imm(*i),
            tacky::Val::Var(name) => assembly::Operand::Pseudo(name.clone()),
        }
    }

    // =================================================================
    // PASS 2: Replace Pseudoregisters with Stack Slots
    // =================================================================

    fn replace_pseudo_with_stack_pass2(
        &self,
        asm_func: &mut assembly::Function,
    ) -> Result<u32, String> {
        let mut var_map: HashMap<String, i32> = HashMap::new();
        let mut current_offset = 0;

        for inst in &mut asm_func.instructions {
            match inst {
                assembly::Instruction::Mov { src, dst } => {
                    self.assign_stack_offset(src, &mut var_map, &mut current_offset);
                    self.assign_stack_offset(dst, &mut var_map, &mut current_offset);
                }
                assembly::Instruction::Unary { operand, .. } => {
                    self.assign_stack_offset(operand, &mut var_map, &mut current_offset);
                }
                _ => {} // Ret, AllocateStack 不包含操作数
            }
        }
        Ok(current_offset.abs() as u32)
    }

    /// 辅助函数：如果操作数是 Pseudo，就给它分配一个栈偏移量。
    fn assign_stack_offset(
        &self,
        op: &mut assembly::Operand,
        var_map: &mut HashMap<String, i32>,
        current_offset: &mut i32,
    ) {
        if let assembly::Operand::Pseudo(name) = op {
            let offset = *var_map.entry(name.clone()).or_insert_with(|| {
                *current_offset -= 4; // 每个变量占 4 字节
                *current_offset
            });
            *op = assembly::Operand::Stack(offset);
        }
    }

    // =================================================================
    // PASS 3: Fix Up Instructions
    // =================================================================

    fn fixup_instructions_pass3(&self, asm_func: &mut assembly::Function, stack_bytes: u32) {
        let mut new_instructions = Vec::new();

        // 1. 添加 AllocateStack 指令
        if stack_bytes > 0 {
            // 确保栈分配是 16 字节对齐的（System V ABI 要求）
            let aligned_bytes = (stack_bytes + 15) & !15;
            new_instructions.push(assembly::Instruction::AllocateStack {
                bytes: aligned_bytes,
            });
        }

        // 2. 遍历并修复 mov mem, mem
        for inst in &asm_func.instructions {
            if let assembly::Instruction::Mov {
                src: assembly::Operand::Stack(src_offset),
                dst: assembly::Operand::Stack(dst_offset),
            } = inst
            {
                // 这是无效的 mov mem, mem, 需要修复
                // movl -4(%rbp), %r10d
                new_instructions.push(assembly::Instruction::Mov {
                    src: assembly::Operand::Stack(*src_offset),
                    dst: assembly::Operand::Reg(assembly::Register::R10),
                });
                // movl %r10d, -8(%rbp)
                new_instructions.push(assembly::Instruction::Mov {
                    src: assembly::Operand::Reg(assembly::Register::R10),
                    dst: assembly::Operand::Stack(*dst_offset),
                });
            } else {
                // 其他指令直接复制
                new_instructions.push(inst.clone());
            }
        }

        // 用修复后的指令列表替换原来的
        asm_func.instructions = new_instructions;
    }
}

// Instruction 没有实现 Clone，所以我们需要手动克隆
impl Clone for assembly::Instruction {
    fn clone(&self) -> Self {
        match self {
            Self::Mov { src, dst } => Self::Mov {
                src: src.clone(),
                dst: dst.clone(),
            },
            Self::Unary { op, operand } => Self::Unary {
                op: *op,
                operand: operand.clone(),
            },
            Self::AllocateStack { bytes } => Self::AllocateStack { bytes: *bytes },
            Self::Ret => Self::Ret,
        }
    }
}
