// src/backend/emitter.rs

use crate::ir::assembly::{
    BinaryOperator, CondCode, Function, Instruction, Operand, Program, Register, UnaryOperator,
};
use std::collections::HashSet; // 用于跟踪文件中定义的函数
use std::fmt::Write;

struct PlatformConfig {
    local_label_prefix: &'static str,
    global_label_prefix: &'static str,
    use_plt: bool,
}

impl PlatformConfig {
    fn new() -> Self {
        #[cfg(target_os = "macos")]
        return PlatformConfig {
            local_label_prefix: "L",
            global_label_prefix: "_",
            use_plt: false, // macOS 不使用 @PLT
        };

        #[cfg(not(target_os = "macos"))]
        return PlatformConfig {
            local_label_prefix: ".L",
            global_label_prefix: "",
            use_plt: true, // Linux 使用 @PLT
        };
    }

    fn format_local_label(&self, label: &str) -> String {
        format!("{}{}", self.local_label_prefix, label)
    }

    fn format_global_label(&self, label: &str) -> String {
        format!("{}{}", self.global_label_prefix, label)
    }
}

/// 将汇编 AST 转换为最终的汇编代码字符串。
pub fn emit_assembly(asm_program: Program) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();
    let config = PlatformConfig::new();

    // 【核心修改】创建一个集合，存储所有在当前文件中定义的函数名。
    // 这对于决定 `call` 指令是否需要 `@PLT` 后缀至关重要。
    let defined_functions: HashSet<_> = asm_program
        .functions
        .iter()
        .map(|f| f.name.clone())
        .collect();

    // 循环发射每个函数的代码
    for func in &asm_program.functions {
        emit_function(&mut output, func, &config, &defined_functions)?;
    }

    // 根据项目要求，在 Linux 上添加 .section 指令
    #[cfg(target_os = "linux")]
    writeln!(&mut output, r#".section .note.GNU-stack,"",@progbits"#)?;

    Ok(output)
}

/// 发射单个函数的汇编代码。
fn emit_function(
    output: &mut String,
    func: &Function,
    config: &PlatformConfig,
    defined_functions: &HashSet<String>, // 接收定义的函数集合
) -> Result<(), std::fmt::Error> {
    let function_name = config.format_global_label(&func.name);

    writeln!(output, ".globl {}", function_name)?;
    writeln!(output, "{}:", function_name)?;
    writeln!(output, "    pushq %rbp")?;
    writeln!(output, "    movq %rsp, %rbp")?;

    for instruction in &func.instructions {
        match instruction {
            // --- 指令发射逻辑，与之前类似 ---
            Instruction::Mov { src, dst } => {
                // movl 用于 4 字节操作
                writeln!(
                    output,
                    "    movl {}, {}",
                    format_operand(src, 4), // 使用 4 字节格式
                    format_operand(dst, 4)
                )?;
            }
            Instruction::Unary { op, operand } => {
                writeln!(
                    output,
                    "    {} {}",
                    format_unary_operator(op),
                    format_operand(operand, 4) // 一元操作通常是 4 字节
                )?;
            }
            Instruction::Binary { op, src, dst } => {
                writeln!(
                    output,
                    "    {} {}, {}",
                    format_binary_operator(op),
                    format_operand(src, 4),
                    format_operand(dst, 4)
                )?;
            }
            Instruction::Idiv(operand) => {
                writeln!(output, "    idivl {}", format_operand(operand, 4))?;
            }
            Instruction::Cdq => {
                writeln!(output, "    cdq")?;
            }
            Instruction::AllocateStack { bytes } => {
                writeln!(output, "    subq ${}, %rsp", bytes)?;
            }
            Instruction::Ret => {
                writeln!(output, "    movq %rbp, %rsp")?;
                writeln!(output, "    popq %rbp")?;
                writeln!(output, "    ret")?;
            }
            Instruction::Cmp { src1, src2 } => {
                writeln!(
                    output,
                    "    cmpl {}, {}",
                    format_operand(src1, 4),
                    format_operand(src2, 4)
                )?;
            }
            Instruction::Label(name) => {
                writeln!(output, "{}:", config.format_local_label(name))?;
            }
            Instruction::Jmp(target) => {
                writeln!(output, "    jmp {}", config.format_local_label(target))?;
            }
            Instruction::JmpCC(cond, target) => {
                writeln!(
                    output,
                    "    j{} {}",
                    format_cond_code(cond),
                    config.format_local_label(target)
                )?;
            }
            Instruction::SetCC(cond, operand) => {
                writeln!(
                    output,
                    "    set{} {}",
                    format_cond_code(cond),
                    format_operand(operand, 1) // SetCC 操作 1 字节
                )?;
            }

            // --- 【核心修改】处理新指令 ---
            Instruction::DeallocateStack(bytes) => {
                writeln!(output, "    addq ${}, %rsp", bytes)?;
            }
            Instruction::Push(operand) => {
                // pushq 操作 8 字节
                writeln!(output, "    pushq {}", format_operand(operand, 8))?;
            }
            Instruction::Call(name) => {
                let mut call_target = config.format_global_label(name);
                // 检查是否需要 @PLT
                if config.use_plt && !defined_functions.contains(name) {
                    write!(&mut call_target, "@PLT")?;
                }
                writeln!(output, "    call {}", call_target)?;
            }
        }
    }
    Ok(())
}

/// 辅助函数：将 CondCode 转换为指令后缀。 (不变)
fn format_cond_code(cc: &CondCode) -> &'static str {
    match cc {
        CondCode::E => "e",
        CondCode::NE => "ne",
        CondCode::L => "l",
        CondCode::LE => "le",
        CondCode::G => "g",
        CondCode::GE => "ge",
    }
}

/// 【核心大修】辅助函数：将 Operand 格式化为汇编操作数。
/// 现在接收一个 `size_in_bytes` 参数。
fn format_operand(op: &Operand, size_in_bytes: u8) -> String {
    match op {
        Operand::Imm(value) => format!("${}", value),
        Operand::Reg(reg) => format_register(reg, size_in_bytes),
        Operand::Stack(offset) => format!("{}(%rbp)", offset),
        Operand::Pseudo(name) => {
            panic!(
                "Error: Pseudoregister '{}' was not replaced before code emission.",
                name
            );
        }
    }
}

/// 【新增辅助函数】根据大小格式化寄存器名称
fn format_register(reg: &Register, size_in_bytes: u8) -> String {
    let names = match reg {
        // (8-byte, 4-byte, 1-byte)
        Register::AX => ("%rax", "%eax", "%al"),
        Register::DX => ("%rdx", "%edx", "%dl"),
        Register::CX => ("%rcx", "%ecx", "%cl"),
        Register::DI => ("%rdi", "%edi", "%dil"),
        Register::SI => ("%rsi", "%esi", "%sil"),
        Register::R8 => ("%r8", "%r8d", "%r8b"),
        Register::R9 => ("%r9", "%r9d", "%r9b"),
        Register::R10 => ("%r10", "%r10d", "%r10b"),
        Register::R11 => ("%r11", "%r11d", "%r11b"),
    };

    let name_str = match size_in_bytes {
        8 => names.0,
        4 => names.1,
        1 => names.2,
        _ => panic!("Unsupported register size: {} bytes", size_in_bytes),
    };
    name_str.to_string()
}

/// 辅助函数：将 UnaryOperator 枚举格式化为指令名。(不变)
fn format_unary_operator(op: &UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Neg => "negl",
        UnaryOperator::Not => "notl",
    }
}

/// 辅助函数：将 BinaryOperator 枚举格式化为指令名。(不变)
fn format_binary_operator(op: &BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Add => "addl",
        BinaryOperator::Subtract => "subl",
        BinaryOperator::Multiply => "imull",
    }
}
