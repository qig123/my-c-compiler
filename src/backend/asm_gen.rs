// src/backend/asm_gen.rs

use crate::ir::{assembly, tacky};
use std::collections::HashMap;

/// 负责将 TACKY IR 转换为最终的汇编 AST。
/// 这个过程分为三个阶段，现在针对整个程序进行。
pub struct AsmGenerator {
    // 这个结构体仍然可以是无状态的
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
        let mut final_functions = Vec::new();

        // 遍历 TACKY 程序中的每一个函数
        for tacky_func in tacky_program.functions {
            // --- PASS 1: TACKY -> Assembly with Pseudoregisters ---
            // 将单个 TACKY 函数转换为汇编函数
            let mut asm_func = self.convert_tacky_to_asm_pass1(&tacky_func)?;

            // --- PASS 2: Replace Pseudoregisters -> Stack slots ---
            // 为当前函数分配栈空间，并返回所需字节数
            let stack_bytes_needed = self.replace_pseudo_with_stack_pass2(&mut asm_func)?;

            // --- PASS 3: Fix up instructions ---
            // 修复当前函数的指令，并添加函数序言/尾言所需的 AllocateStack
            self.fixup_instructions_pass3(&mut asm_func, stack_bytes_needed);

            final_functions.push(asm_func);
        }

        Ok(assembly::Program {
            functions: final_functions,
        })
    }

    // =================================================================
    // PASS 1: Convert TACKY to Assembly with Pseudoregisters
    // =================================================================

    fn convert_tacky_to_asm_pass1(
        &self,
        tacky_func: &tacky::Function,
    ) -> Result<assembly::Function, String> {
        let mut instructions = Vec::new();

        // 【核心修改】在函数体开始处，将所有参数复制到伪寄存器中
        self.copy_params_to_pseudo(&tacky_func.params, &mut instructions);

        // 遍历函数体中的每条 TACKY 指令
        for tacky_inst in &tacky_func.body {
            match tacky_inst {
                // --- 【核心修改】处理 FunCall ---
                tacky::Instruction::FunCall { name, args, dst } => {
                    self.convert_funcall(name, args, dst, &mut instructions);
                }

                // --- 简单直接的转换 (基本不变) ---
                tacky::Instruction::Return(val) => {
                    instructions.push(assembly::Instruction::Mov {
                        src: self.convert_tacky_val(val),
                        dst: assembly::Operand::Reg(assembly::Register::AX),
                    });
                    instructions.push(assembly::Instruction::Ret);
                }
                tacky::Instruction::Copy { src, dst } => {
                    instructions.push(assembly::Instruction::Mov {
                        src: self.convert_tacky_val(src),
                        dst: self.convert_tacky_val(dst),
                    });
                }
                tacky::Instruction::Jump(target) => {
                    instructions.push(assembly::Instruction::Jmp(target.clone()));
                }
                tacky::Instruction::Label(name) => {
                    instructions.push(assembly::Instruction::Label(name.clone()));
                }

                // --- 涉及比较和跳转的转换 (基本不变) ---
                tacky::Instruction::JumpIfZero { condition, target } => {
                    instructions.push(assembly::Instruction::Cmp {
                        src1: assembly::Operand::Imm(0),
                        src2: self.convert_tacky_val(condition),
                    });
                    instructions.push(assembly::Instruction::JmpCC(
                        assembly::CondCode::E,
                        target.clone(),
                    ));
                }
                tacky::Instruction::JumpIfNotZero { condition, target } => {
                    instructions.push(assembly::Instruction::Cmp {
                        src1: assembly::Operand::Imm(0),
                        src2: self.convert_tacky_val(condition),
                    });
                    instructions.push(assembly::Instruction::JmpCC(
                        assembly::CondCode::NE,
                        target.clone(),
                    ));
                }

                // --- 运算符转换 (基本不变) ---
                tacky::Instruction::Unary { op, src, dst } => {
                    self.convert_unary_op(op, src, dst, &mut instructions);
                }
                tacky::Instruction::Binary {
                    op,
                    src1,
                    src2,
                    dst,
                } => {
                    self.convert_binary_op(op, src1, src2, dst, &mut instructions);
                }
            }
        }
        Ok(assembly::Function {
            name: tacky_func.name.clone(),
            instructions,
        })
    }

    /// 【新增辅助函数】根据函数调用伪代码实现 FunCall 转换
    fn convert_funcall(
        &self,
        name: &str,
        args: &[tacky::Val],
        dst: &tacky::Val,
        instructions: &mut Vec<assembly::Instruction>,
    ) {
        let arg_registers = [
            assembly::Register::DI,
            assembly::Register::SI,
            assembly::Register::DX,
            assembly::Register::CX,
            assembly::Register::R8,
            assembly::Register::R9,
        ];

        let (register_args, stack_args) = if args.len() > 6 {
            args.split_at(6)
        } else {
            (args, &[][..])
        };

        // 1. 调整栈对齐
        let stack_padding = if !stack_args.is_empty() && stack_args.len() % 2 != 0 {
            8
        } else {
            0
        };

        if stack_padding > 0 {
            instructions.push(assembly::Instruction::AllocateStack {
                bytes: stack_padding,
            });
        }

        // 2. 通过寄存器传递参数
        for (i, arg) in register_args.iter().enumerate() {
            instructions.push(assembly::Instruction::Mov {
                src: self.convert_tacky_val(arg),
                dst: assembly::Operand::Reg(arg_registers[i]),
            });
        }

        // 3. 通过栈传递参数 (反向)
        for arg in stack_args.iter().rev() {
            let assembly_arg = self.convert_tacky_val(arg);
            // 根据伪代码，如果参数在内存中（现在是Pseudo），先移到AX
            match assembly_arg {
                assembly::Operand::Imm(_) | assembly::Operand::Reg(_) => {
                    instructions.push(assembly::Instruction::Push(assembly_arg));
                }
                _ => {
                    // Pseudo, or later Stack
                    instructions.push(assembly::Instruction::Mov {
                        src: assembly_arg,
                        dst: assembly::Operand::Reg(assembly::Register::AX),
                    });
                    instructions.push(assembly::Instruction::Push(assembly::Operand::Reg(
                        assembly::Register::AX,
                    )));
                }
            }
        }

        // 4. 发出 call 指令
        instructions.push(assembly::Instruction::Call(name.to_string()));

        // 5. 调整栈指针 (清理栈上参数和填充)
        let bytes_to_remove = (stack_args.len() * 8) as u32 + stack_padding;
        if bytes_to_remove > 0 {
            instructions.push(assembly::Instruction::DeallocateStack(bytes_to_remove));
        }

        // 6. 获取返回值
        instructions.push(assembly::Instruction::Mov {
            src: assembly::Operand::Reg(assembly::Register::AX),
            dst: self.convert_tacky_val(dst),
        });
    }

    /// 【新增辅助函数】将函数参数从寄存器/栈复制到伪寄存器中
    fn copy_params_to_pseudo(
        &self,
        params: &[String],
        instructions: &mut Vec<assembly::Instruction>,
    ) {
        let arg_registers = [
            assembly::Register::DI,
            assembly::Register::SI,
            assembly::Register::DX,
            assembly::Register::CX,
            assembly::Register::R8,
            assembly::Register::R9,
        ];

        for (i, param_name) in params.iter().enumerate() {
            let src_operand = if i < 6 {
                // 前 6 个参数来自寄存器
                assembly::Operand::Reg(arg_registers[i])
            } else {
                // 第 7 个参数在 16(%rbp), 第 8 个在 24(%rbp), ...
                // 偏移量 = 16 + (i - 6) * 8
                let offset = 16 + (i - 6) * 8;
                assembly::Operand::Stack(offset as i32)
            };

            instructions.push(assembly::Instruction::Mov {
                src: src_operand,
                dst: assembly::Operand::Pseudo(param_name.clone()),
            });
        }
    }

    // `convert_unary_op` 和 `convert_binary_op` 可以从原来的 `convert_tacky_to_asm_pass1` 中提取出来，
    // 以保持函数体整洁，但内容不变。
    fn convert_unary_op(
        &self,
        op: &tacky::UnaryOperator,
        src: &tacky::Val,
        dst: &tacky::Val,
        instructions: &mut Vec<assembly::Instruction>,
    ) {
        let dst_operand = self.convert_tacky_val(dst);
        match op {
            tacky::UnaryOperator::Not => {
                instructions.push(assembly::Instruction::Cmp {
                    src1: assembly::Operand::Imm(0),
                    src2: self.convert_tacky_val(src),
                });
                instructions.push(assembly::Instruction::Mov {
                    src: assembly::Operand::Imm(0),
                    dst: dst_operand.clone(),
                });
                instructions.push(assembly::Instruction::SetCC(
                    assembly::CondCode::E,
                    dst_operand,
                ));
            }
            tacky::UnaryOperator::Negate | tacky::UnaryOperator::Complement => {
                let asm_op = match op {
                    tacky::UnaryOperator::Negate => assembly::UnaryOperator::Neg,
                    tacky::UnaryOperator::Complement => assembly::UnaryOperator::Not,
                    _ => unreachable!(),
                };
                instructions.push(assembly::Instruction::Mov {
                    src: self.convert_tacky_val(src),
                    dst: dst_operand.clone(),
                });
                instructions.push(assembly::Instruction::Unary {
                    op: asm_op,
                    operand: dst_operand,
                });
            }
        }
    }

    fn convert_binary_op(
        &self,
        op: &tacky::BinaryOperator,
        src1: &tacky::Val,
        src2: &tacky::Val,
        dst: &tacky::Val,
        instructions: &mut Vec<assembly::Instruction>,
    ) {
        let dst_operand = self.convert_tacky_val(dst);
        let src1_operand = self.convert_tacky_val(src1);
        let src2_operand = self.convert_tacky_val(src2);

        match op {
            tacky::BinaryOperator::Equal
            | tacky::BinaryOperator::NotEqual
            | tacky::BinaryOperator::LessThan
            | tacky::BinaryOperator::LessOrEqual
            | tacky::BinaryOperator::GreaterThan
            | tacky::BinaryOperator::GreaterEqual => {
                let cond_code = match op {
                    tacky::BinaryOperator::Equal => assembly::CondCode::E,
                    tacky::BinaryOperator::NotEqual => assembly::CondCode::NE,
                    tacky::BinaryOperator::LessThan => assembly::CondCode::L,
                    tacky::BinaryOperator::LessOrEqual => assembly::CondCode::LE,
                    tacky::BinaryOperator::GreaterThan => assembly::CondCode::G,
                    tacky::BinaryOperator::GreaterEqual => assembly::CondCode::GE,
                    _ => unreachable!(),
                };
                instructions.push(assembly::Instruction::Cmp {
                    src1: src2_operand,
                    src2: src1_operand,
                });
                instructions.push(assembly::Instruction::Mov {
                    src: assembly::Operand::Imm(0),
                    dst: dst_operand.clone(),
                });
                instructions.push(assembly::Instruction::SetCC(cond_code, dst_operand));
            }
            tacky::BinaryOperator::Divide => {
                instructions.push(assembly::Instruction::Mov {
                    src: src1_operand,
                    dst: assembly::Operand::Reg(assembly::Register::AX),
                });
                instructions.push(assembly::Instruction::Cdq);
                instructions.push(assembly::Instruction::Idiv(src2_operand));
                instructions.push(assembly::Instruction::Mov {
                    src: assembly::Operand::Reg(assembly::Register::AX),
                    dst: dst_operand,
                });
            }
            tacky::BinaryOperator::Remainder => {
                instructions.push(assembly::Instruction::Mov {
                    src: src1_operand,
                    dst: assembly::Operand::Reg(assembly::Register::AX),
                });
                instructions.push(assembly::Instruction::Cdq);
                instructions.push(assembly::Instruction::Idiv(src2_operand));
                instructions.push(assembly::Instruction::Mov {
                    src: assembly::Operand::Reg(assembly::Register::DX),
                    dst: dst_operand,
                });
            }
            tacky::BinaryOperator::Add
            | tacky::BinaryOperator::Subtract
            | tacky::BinaryOperator::Multiply => {
                let asm_op = match op {
                    tacky::BinaryOperator::Add => assembly::BinaryOperator::Add,
                    tacky::BinaryOperator::Subtract => assembly::BinaryOperator::Subtract,
                    tacky::BinaryOperator::Multiply => assembly::BinaryOperator::Multiply,
                    _ => unreachable!(),
                };
                instructions.push(assembly::Instruction::Mov {
                    src: src1_operand,
                    dst: dst_operand.clone(),
                });
                instructions.push(assembly::Instruction::Binary {
                    op: asm_op,
                    src: src2_operand,
                    dst: dst_operand,
                });
            }
        }
    }

    /// 辅助函数：将 tacky::Val 转换为 assembly::Operand。 (不变)
    fn convert_tacky_val(&self, val: &tacky::Val) -> assembly::Operand {
        match val {
            tacky::Val::Constant(i) => assembly::Operand::Imm(*i),
            tacky::Val::Var(name) => assembly::Operand::Pseudo(name.clone()),
        }
    }

    // =================================================================
    // PASS 2: Replace Pseudoregisters with Stack Slots (基本不变, 但现在处理新指令)
    // =================================================================

    fn replace_pseudo_with_stack_pass2(
        &self,
        asm_func: &mut assembly::Function,
    ) -> Result<u32, String> {
        let mut var_map: HashMap<String, i32> = HashMap::new();
        let mut current_offset = 0;

        for inst in &mut asm_func.instructions {
            // 用一个闭包来简化重复代码
            let mut assign = |op: &mut assembly::Operand| {
                self.assign_stack_offset(op, &mut var_map, &mut current_offset);
            };

            match inst {
                assembly::Instruction::Mov { src, dst } => {
                    assign(src);
                    assign(dst);
                }
                assembly::Instruction::Unary { operand, .. } => {
                    assign(operand);
                }
                assembly::Instruction::Binary { src, dst, .. } => {
                    assign(src);
                    assign(dst);
                }
                assembly::Instruction::Idiv(operand) => {
                    assign(operand);
                }
                assembly::Instruction::Cmp { src1, src2 } => {
                    assign(src1);
                    assign(src2);
                }
                assembly::Instruction::SetCC(_, operand) => {
                    assign(operand);
                }
                // 【新增】处理 Push 指令
                assembly::Instruction::Push(operand) => {
                    assign(operand);
                }
                _ => {} // Ret, Cdq, Jmp, Label, Call, Allocate/DeallocateStack 等不含伪寄存器
            }
        }
        // 参数也计入栈大小，所以这个逻辑是正确的
        Ok(current_offset.abs() as u32)
    }

    /// 辅助函数：如果操作数是 Pseudo，就给它分配一个栈偏移量。(不变)
    fn assign_stack_offset(
        &self,
        op: &mut assembly::Operand,
        var_map: &mut HashMap<String, i32>,
        current_offset: &mut i32,
    ) {
        if let assembly::Operand::Pseudo(name) = op {
            let offset = *var_map.entry(name.clone()).or_insert_with(|| {
                *current_offset -= 4; // 每个变量/参数占 4 字节
                *current_offset
            });
            *op = assembly::Operand::Stack(offset);
        }
    }

    // =================================================================
    // PASS 3: Fix Up Instructions (基本不变，但要处理新指令)
    // =================================================================

    fn fixup_instructions_pass3(&self, asm_func: &mut assembly::Function, stack_bytes: u32) {
        let mut new_instructions = Vec::new();

        // 1. 添加 AllocateStack 指令
        if stack_bytes > 0 {
            // 【核心修改】向上取整到 16 的倍数
            let aligned_bytes = (stack_bytes + 15) & !15;
            new_instructions.push(assembly::Instruction::AllocateStack {
                bytes: aligned_bytes,
            });
        }

        for inst in &asm_func.instructions {
            match inst {
                // ... 所有之前的修复逻辑保持不变 ...
                assembly::Instruction::Mov {
                    src: assembly::Operand::Stack(src_offset),
                    dst: assembly::Operand::Stack(dst_offset),
                } => {
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Stack(*src_offset),
                        dst: assembly::Operand::Reg(assembly::Register::R10),
                    });
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Reg(assembly::Register::R10),
                        dst: assembly::Operand::Stack(*dst_offset),
                    });
                }
                assembly::Instruction::Binary {
                    op: op @ (assembly::BinaryOperator::Add | assembly::BinaryOperator::Subtract),
                    src: assembly::Operand::Stack(src_offset),
                    dst: assembly::Operand::Stack(dst_offset),
                } => {
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Stack(*src_offset),
                        dst: assembly::Operand::Reg(assembly::Register::R10),
                    });
                    new_instructions.push(assembly::Instruction::Binary {
                        op: *op,
                        src: assembly::Operand::Reg(assembly::Register::R10),
                        dst: assembly::Operand::Stack(*dst_offset),
                    });
                }
                assembly::Instruction::Binary {
                    op: assembly::BinaryOperator::Multiply,
                    src,
                    dst: assembly::Operand::Stack(dst_offset),
                } => {
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Stack(*dst_offset),
                        dst: assembly::Operand::Reg(assembly::Register::R11),
                    });
                    new_instructions.push(assembly::Instruction::Binary {
                        op: assembly::BinaryOperator::Multiply,
                        src: src.clone(),
                        dst: assembly::Operand::Reg(assembly::Register::R11),
                    });
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Reg(assembly::Register::R11),
                        dst: assembly::Operand::Stack(*dst_offset),
                    });
                }
                assembly::Instruction::Idiv(assembly::Operand::Imm(val)) => {
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Imm(*val),
                        dst: assembly::Operand::Reg(assembly::Register::R10),
                    });
                    new_instructions.push(assembly::Instruction::Idiv(assembly::Operand::Reg(
                        assembly::Register::R10,
                    )));
                }
                assembly::Instruction::Cmp { src1, src2 } => {
                    let mut s1 = src1.clone();
                    let mut s2 = src2.clone();
                    if let (assembly::Operand::Stack(o1), assembly::Operand::Stack(_)) = (&s1, &s2)
                    {
                        new_instructions.push(assembly::Instruction::Mov {
                            src: assembly::Operand::Stack(*o1),
                            dst: assembly::Operand::Reg(assembly::Register::R10),
                        });
                        s1 = assembly::Operand::Reg(assembly::Register::R10);
                    }
                    if let assembly::Operand::Imm(val) = &s2 {
                        new_instructions.push(assembly::Instruction::Mov {
                            src: assembly::Operand::Imm(*val),
                            dst: assembly::Operand::Reg(assembly::Register::R11),
                        });
                        s2 = assembly::Operand::Reg(assembly::Register::R11);
                    }
                    new_instructions.push(assembly::Instruction::Cmp { src1: s1, src2: s2 });
                }

                // 【新增】修复 push imm (x86_64 `pushq` 不直接支持32位立即数，需要先mov)
                assembly::Instruction::Push(assembly::Operand::Imm(val)) => {
                    new_instructions.push(assembly::Instruction::Mov {
                        src: assembly::Operand::Imm(*val),
                        dst: assembly::Operand::Reg(assembly::Register::R10),
                    });
                    new_instructions.push(assembly::Instruction::Push(assembly::Operand::Reg(
                        assembly::Register::R10,
                    )));
                }

                // 所有其他合法指令，直接复制
                _ => {
                    new_instructions.push(inst.clone());
                }
            }
        }

        asm_func.instructions = new_instructions;
    }
}
