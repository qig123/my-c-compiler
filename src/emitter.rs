// 【 src/emitter.rs

use crate::codegen::{AsmInstruction, AsmProgram, Operand, RegisterKind};
use std::fmt::Write;

/// 将汇编 AST 转换为最终的汇编代码字符串。
///
/// # Arguments
/// * `asm_ast` - 要发射的汇编程序 AST。
///
/// # Returns
/// A `Result` containing the assembly code string or an error.
pub fn emit_assembly(asm_ast: AsmProgram) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    // 根据项目要求处理函数名
    // 在 macOS 上，所有 C 函数名前面都需要加一个下划线。
    #[cfg(target_os = "macos")]
    let function_name = format!("_{}", asm_ast.function.name);
    #[cfg(not(target_os = "macos"))]
    let function_name = asm_ast.function.name.as_str();

    // 发射函数定义
    // .globl <name>  -> 使函数名对链接器可见
    // <name>:        -> 定义函数标签
    writeln!(&mut output, ".globl {}", function_name)?;
    writeln!(&mut output, "{}:", function_name)?;

    // 遍历并转换每一条指令
    for instruction in asm_ast.function.instructions {
        match instruction {
            AsmInstruction::Mov { src, dst } => {
                // 指令需要缩进，使其更具可读性
                writeln!(
                    &mut output,
                    "    movl {}, {}",
                    format_operand(&src),
                    format_operand(&dst)
                )?;
            }
            AsmInstruction::Ret => {
                writeln!(&mut output, "    ret")?;
            }
        }
    }

    // 根据项目要求，在 Linux 上添加 .section 指令
    #[cfg(target_os = "linux")]
    writeln!(&mut output, r#"    .section .note.GNU-stack,"",@progbits"#)?;

    Ok(output)
}

/// 一个辅助函数，用于将 `Operand` 格式化为汇编操作数。
fn format_operand(op: &Operand) -> String {
    match op {
        Operand::Imm(value) => format!("${}", value),
        Operand::Reg(kind) => match kind {
            RegisterKind::EAX => "%eax".to_string(),
        },
    }
}
