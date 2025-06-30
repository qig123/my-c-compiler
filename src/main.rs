// src/main.rs

use clap::Parser;
use my_c_compiler::lexer;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A C compiler, written in Rust.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Stop after lexing and print tokens
    #[arg(long)]
    lex: bool,

    /// Stop after parsing and print AST
    #[arg(long)]
    parse: bool,

    /// Stop after assembly generation and print assembly
    #[arg(long)]
    codegen: bool,

    /// The C source file to compile
    input_file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 解析参数
    let cli = Cli::parse();

    // 2. 调用主流程函数，并处理其返回结果
    if let Err(e) = run_pipeline(&cli) {
        // 如果 run_pipeline 返回错误，打印错误信息
        eprintln!("\nCompilation failed: {}", e);
        // 并以非零状态码退出
        std::process::exit(1);
    }
    return Ok(());
}

/// 包含完整编译流程的主函数，它返回一个 Result。
fn run_pipeline(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    // --- 从这里开始，是原来 main 函数的大部分逻辑 ---

    // 1. 确定文件路径
    let input_path = &cli.input_file;
    if !input_path.exists() {
        return Err(format!("Input file not found: {}", input_path.display()).into());
    }

    let file_stem = input_path.file_stem().ok_or("Invalid input file name")?;
    let parent_dir = input_path.parent().unwrap_or_else(|| Path::new("."));

    let preprocessed_path = parent_dir.join(file_stem).with_extension("i");
    let assembly_path = parent_dir.join(file_stem).with_extension("s");
    let output_path = parent_dir.join(file_stem);

    // 2. STAGE 1: PREPROCESSING
    println!("1. Preprocessing {}...", input_path.display());
    preprocess(input_path, &preprocessed_path)?;
    println!(
        "   ✓ Preprocessing complete: {}",
        preprocessed_path.display()
    );

    // 3. READ PREPROCESSED SOURCE CODE
    let source_code = fs::read_to_string(&preprocessed_path)?;

    // --- 处理 --lex, --parse, --codegen 标志 ---
    // 注意：如果这些阶段成功，它们会提前返回 Ok(())
    if cli.lex {
        println!("\n2. Lexing stage requested...");
        // 将 lexer 迭代器转换为 Result<Vec<Token>, String>
        let tokens_result: Result<Vec<lexer::Token>, _> = lexer::Lexer::new(&source_code).collect();
        let tokens = tokens_result?; // 如果有错误，`?` 会在这里传播它

        println!("   ✓ Lexing successful.");
        println!("--- Generated Tokens ---");
        for token in tokens {
            println!("  {:?}", token);
        }
        println!("------------------------");
        println!("\nHalting as requested by --lex.");
        // 在成功时清理 .i 文件
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    // (将来在这里添加 --parse 和 --codegen 的逻辑)

    // --- 完整编译流程 ---

    // 4. STAGE 2: COMPILE TO ASSEMBLY
    println!("\n2. Compiling (lex, parse, codegen)...");
    let assembly_code = compile_to_assembly(&source_code)?;
    fs::write(&assembly_path, assembly_code)?;
    println!("   ✓ Compilation complete: {}", assembly_path.display());

    // 5. STAGE 3: ASSEMBLE & LINK
    println!("\n3. Assembling and linking...");
    assemble(&assembly_path, &output_path)?;
    println!(
        "   ✓ Assembling and linking complete: {}",
        output_path.display()
    );

    // --- 成功时的清理 ---
    // 只有在整个流程成功完成后，才删除中间文件
    fs::remove_file(&preprocessed_path)?;
    fs::remove_file(&assembly_path)?;

    println!(
        "\n✅ Success! Executable created at: {}",
        output_path.display()
    );

    Ok(())
}

// --- Helper Functions ---

/// Runs an external command and checks for errors.
fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;

    if !status.success() {
        return Err(format!("Command `{:?}` failed with status: {}", command, status).into());
    }
    Ok(())
}

/// Stage 1: Call `gcc` to preprocess the C source file.
fn preprocess(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("gcc");
    cmd.arg("-E").arg("-P").arg(input).arg("-o").arg(output);

    run_command(&mut cmd)
}

/// Stage 3: Call `gcc` to assemble and link the assembly file into an executable.
fn assemble(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("gcc");
    cmd.arg(input).arg("-o").arg(output);

    run_command(&mut cmd)
}

/// STUB: This is your compiler's main function.
/// It takes source code and generates assembly code.
fn compile_to_assembly(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    // We ignore the source for now and return hardcoded assembly.
    let _ = source;

    let assembly_code = r#"
.globl main
main:
  movl $2, %eax
  ret
"#;
    Ok(assembly_code.to_string())
}
