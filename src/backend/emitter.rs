// src/backend/emitter.rs

use crate::ir::assembly::{Function, Instruction, Operand, Program, Register, UnaryOperator};
use std::fmt::Write;

/// 将汇编 AST 转换为最终的汇编代码字符串。
pub fn emit_assembly(asm_program: Program) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    // 发射函数部分
    emit_function(&mut output, &asm_program.function)?;

    // 根据项目要求，在 Linux 上添加 .section 指令
    #[cfg(target_os = "linux")]
    writeln!(&mut output, r#".section .note.GNU-stack,"",@progbits"#)?;

    Ok(output)
}

/// 发射单个函数的汇编代码。
fn emit_function(output: &mut String, func: &Function) -> Result<(), std::fmt::Error> {
    // 根据项目要求处理函数名
    #[cfg(target_os = "macos")]
    let function_name = format!("_{}", func.name);
    #[cfg(not(target_os = "macos"))]
    let function_name = func.name.as_str();

    // 1. 发射函数标签和全局声明
    writeln!(output, ".globl {}", function_name)?;
    writeln!(output, "{}:", function_name)?;

    // 2. 发射函数序言
    writeln!(output, "    pushq %rbp")?;
    writeln!(output, "    movq %rsp, %rbp")?;

    // 3. 遍历并转换每一条指令
    for instruction in &func.instructions {
        match instruction {
            Instruction::Mov { src, dst } => {
                writeln!(
                    output,
                    "    movl {}, {}",
                    format_operand(src),
                    format_operand(dst)
                )?;
            }
            Instruction::Unary { op, operand } => {
                writeln!(
                    output,
                    "    {} {}",
                    format_unary_operator(op),
                    format_operand(operand)
                )?;
            }
            Instruction::AllocateStack { bytes } => {
                writeln!(output, "    subq ${}, %rsp", bytes)?;
            }
            Instruction::Ret => {
                // 发射函数尾声
                writeln!(output, "    movq %rbp, %rsp")?;
                writeln!(output, "    popq %rbp")?;
                writeln!(output, "    ret")?;
            }
        }
    }
    Ok(())
}

/// 【新增】辅助函数：将 UnaryOperator 枚举格式化为指令名。
fn format_unary_operator(op: &UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Neg => "negl",
        UnaryOperator::Not => "notl",
    }
}

/// 【修改】辅助函数：将 Operand 枚举格式化为汇编操作数。
fn format_operand(op: &Operand) -> String {
    match op {
        Operand::Imm(value) => format!("${}", value),
        Operand::Reg(reg) => match reg {
            Register::AX => "%eax".to_string(),
            Register::R10 => "%r10d".to_string(),
        },
        Operand::Stack(offset) => format!("{}(%rbp)", offset),
        Operand::Pseudo(name) => {
            // Pseudo 操作数不应该到达代码发射阶段。
            // 如果到了，说明之前的编译趟有 bug。
            panic!(
                "Error: Pseudoregister '{}' was not replaced before code emission.",
                name
            );
        }
    }
}
