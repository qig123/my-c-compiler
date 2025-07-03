// // src/backend/emitter.rs

// use crate::ir::assembly::{
//     BinaryOperator, CondCode, Function, Instruction, Operand, Program, Register, UnaryOperator,
// };
// use std::fmt::Write;

// struct PlatformConfig {
//     local_label_prefix: &'static str,
//     global_label_prefix: &'static str,
// }

// impl PlatformConfig {
//     fn new() -> Self {
//         #[cfg(target_os = "macos")]
//         return PlatformConfig {
//             local_label_prefix: "L",
//             global_label_prefix: "_",
//         };

//         #[cfg(not(target_os = "macos"))]
//         return PlatformConfig {
//             local_label_prefix: ".L",
//             global_label_prefix: "",
//         };
//     }

//     fn format_local_label(&self, label: &str) -> String {
//         format!("{}{}", self.local_label_prefix, label)
//     }

//     fn format_global_label(&self, label: &str) -> String {
//         format!("{}{}", self.global_label_prefix, label)
//     }
// }

// /// 将汇编 AST 转换为最终的汇编代码字符串。
// pub fn emit_assembly(asm_program: Program) -> Result<String, Box<dyn std::error::Error>> {
//     let mut output = String::new();

//     // 发射函数部分
//     emit_function(&mut output, &asm_program.function)?;

//     // 根据项目要求，在 Linux 上添加 .section 指令
//     #[cfg(target_os = "linux")]
//     writeln!(&mut output, r#".section .note.GNU-stack,"",@progbits"#)?;

//     Ok(output)
// }

// /// 发射单个函数的汇编代码。
// fn emit_function(output: &mut String, func: &Function) -> Result<(), std::fmt::Error> {
//     let config = PlatformConfig::new();

//     let function_name = config.format_global_label(&func.name);

//     writeln!(output, ".globl {}", function_name)?;
//     writeln!(output, "{}:", function_name)?;
//     writeln!(output, "    pushq %rbp")?;
//     writeln!(output, "    movq %rsp, %rbp")?;

//     for instruction in &func.instructions {
//         match instruction {
//             Instruction::Mov { src, dst } => {
//                 writeln!(
//                     output,
//                     "    movl {}, {}",
//                     format_operand(src, false),
//                     format_operand(dst, false)
//                 )?;
//             }
//             Instruction::Unary { op, operand } => {
//                 writeln!(
//                     output,
//                     "    {} {}",
//                     format_unary_operator(op),
//                     format_operand(operand, false)
//                 )?;
//             }
//             Instruction::Binary { op, src, dst } => {
//                 writeln!(
//                     output,
//                     "    {} {}, {}",
//                     format_binary_operator(op),
//                     format_operand(src, false),
//                     format_operand(dst, false)
//                 )?;
//             }
//             Instruction::Idiv(operand) => {
//                 writeln!(output, "    idivl {}", format_operand(operand, false))?;
//             }
//             Instruction::Cdq => {
//                 writeln!(output, "    cdq")?;
//             }
//             Instruction::AllocateStack { bytes } => {
//                 writeln!(output, "    subq ${}, %rsp", bytes)?;
//             }
//             Instruction::Ret => {
//                 writeln!(output, "    movq %rbp, %rsp")?;
//                 writeln!(output, "    popq %rbp")?;
//                 writeln!(output, "    ret")?;
//             }
//             // --- 【新增指令的发射逻辑】 ---
//             Instruction::Cmp { src1, src2 } => {
//                 writeln!(
//                     output,
//                     "    cmpl {}, {}",
//                     format_operand(src1, false),
//                     format_operand(src2, false)
//                 )?;
//             }
//             Instruction::Label(name) => {
//                 writeln!(output, "{}:", config.format_local_label(name))?;
//             }
//             Instruction::Jmp(target) => {
//                 writeln!(output, "    jmp {}", config.format_local_label(target))?;
//             }
//             Instruction::JmpCC(cond, target) => {
//                 writeln!(
//                     output,
//                     "    j{} {}",
//                     format_cond_code(cond),
//                     config.format_local_label(target)
//                 )?;
//             }
//             Instruction::SetCC(cond, operand) => {
//                 // SetCC 操作的是 1 字节操作数，所以 is_byte_operand 为 true
//                 writeln!(
//                     output,
//                     "    set{} {}",
//                     format_cond_code(cond),
//                     format_operand(operand, true)
//                 )?;
//             }
//         }
//     }
//     Ok(())
// }
// /// 【新增】辅助函数：将 CondCode 转换为指令后缀。
// fn format_cond_code(cc: &CondCode) -> &'static str {
//     match cc {
//         CondCode::E => "e",
//         CondCode::NE => "ne",
//         CondCode::L => "l",
//         CondCode::LE => "le",
//         CondCode::G => "g",
//         CondCode::GE => "ge",
//     }
// }

// /// 【核心修改】辅助函数：将 Operand 格式化为汇编操作数。
// /// 新增了一个 `is_byte_operand` 参数来决定寄存器的格式。
// fn format_operand(op: &Operand, is_byte_operand: bool) -> String {
//     match op {
//         Operand::Imm(value) => format!("${}", value),
//         Operand::Reg(reg) => {
//             if is_byte_operand {
//                 // 1-byte register names
//                 match reg {
//                     Register::AX => "%al".to_string(),
//                     Register::DX => "%dl".to_string(),
//                     Register::R10 => "%r10b".to_string(),
//                     Register::R11 => "%r11b".to_string(),
//                 }
//             } else {
//                 // 4-byte register names
//                 match reg {
//                     Register::AX => "%eax".to_string(),
//                     Register::DX => "%edx".to_string(),
//                     Register::R10 => "%r10d".to_string(),
//                     Register::R11 => "%r11d".to_string(),
//                 }
//             }
//         }
//         Operand::Stack(offset) => format!("{}(%rbp)", offset),
//         Operand::Pseudo(name) => {
//             panic!(
//                 "Error: Pseudoregister '{}' was not replaced before code emission.",
//                 name
//             );
//         }
//     }
// }

// fn format_unary_operator(op: &UnaryOperator) -> &'static str {
//     match op {
//         UnaryOperator::Neg => "negl",
//         UnaryOperator::Not => "notl",
//     }
// }

// /// 【新增】辅助函数：将 BinaryOperator 枚举格式化为指令名。
// fn format_binary_operator(op: &BinaryOperator) -> &'static str {
//     match op {
//         BinaryOperator::Add => "addl",
//         BinaryOperator::Subtract => "subl",
//         BinaryOperator::Multiply => "imull",
//     }
// }
