// src/main.rs

use clap::Parser;
use my_c_compiler::lexer::{self, Token};
use my_c_compiler::parser;
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

// 包含完整编译流程的主函数，它返回一个 Result。
fn run_pipeline(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    // 1. STAGE 1: PREPROCESSING
    println!("1. Preprocessing {}...", cli.input_file.display());
    // ... 路径计算 ...
    let input_path = &cli.input_file;
    if !input_path.exists() {
        return Err(format!("Input file not found: {}", input_path.display()).into());
    }
    let file_stem = input_path.file_stem().ok_or("Invalid input file name")?;
    let parent_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
    let preprocessed_path = parent_dir.join(file_stem).with_extension("i");

    preprocess(input_path, &preprocessed_path)?;
    println!(
        "   ✓ Preprocessing complete: {}",
        preprocessed_path.display()
    );
    let source_code = fs::read_to_string(&preprocessed_path)?;

    // 2. STAGE 2: LEXING
    println!("\n2. Lexing source code...");
    let lexer = lexer::Lexer::new(&source_code);
    let tokens: Vec<Token> = lexer.collect::<Result<_, _>>()?;
    println!("   ✓ Lexing successful, found {} tokens.", tokens.len());

    // --lex 标志检查：在词法分析后窥视并退出
    if cli.lex {
        println!("--- Generated Tokens ---");
        for token in &tokens {
            println!("  {:?}", token);
        }
        println!("------------------------");
        println!("\nHalting as requested by --lex.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    println!("\n3. Parsing tokens...");
    let mut parser = parser::Parser::new(&tokens);
    let ast = parser.parse()?; // 调用解析器
    println!("   ✓ Parsing successful.");

    // --parse 标志检查：在解析后窥视并退出
    if cli.parse {
        println!("--- Generated AST ---");
        // 使用 {:#?} "pretty-print" 格式化输出 AST
        println!("{:#?}", ast);
        println!("---------------------");
        println!("\nHalting as requested by --parse.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    // 4. STAGE 4: COMPILE TO ASSEMBLY (之前叫 STAGE 3)
    println!("\n4. Compiling (codegen)...");

    // **修正 1: 将 ast 传递给 compile_to_assembly**
    let assembly_code = compile_to_assembly(ast)?;

    // **修正 2: 重新加入写入文件的关键步骤**
    let assembly_path = parent_dir.join(file_stem).with_extension("s");
    fs::write(&assembly_path, &assembly_code)?; // <-- 将 assembly_code 写入文件
    println!("   ✓ Compilation complete: {}", assembly_path.display());

    // 5. STAGE 5: ASSEMBLE & LINK
    println!("\n5. Assembling and linking...");
    let output_path = parent_dir.join(file_stem);
    // 现在 assemble 调用是有效的，因为它操作的是一个刚刚被创建的 .s 文件
    assemble(&assembly_path, &output_path)?;
    println!(
        "   ✓ Assembling and linking complete: {}",
        output_path.display()
    );

    // --- 成功时的清理 ---
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
    cmd.arg("-E").arg(input).arg("-o").arg(output);

    run_command(&mut cmd)
}

/// Stage 3: Call `gcc` to assemble and link the assembly file into an executable.
fn assemble(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("gcc");
    cmd.arg(input).arg("-o").arg(output);

    run_command(&mut cmd)
}

// *** 修正 3: 恢复 compile_to_assembly 的正确签名！ ***
/// The main compilation function. Takes an AST and will eventually generate code.
fn compile_to_assembly(ast: parser::Program) -> Result<String, Box<dyn std::error::Error>> {
    println!("   -> (Stub) Generating assembly from AST...");
    // 我们可以从 AST 中提取信息来进行下一步
    println!("   -> Compiling function '{}'", ast.function.name);

    // 将来，我们会遍历 AST 来生成真正的代码
    // let assembly_code = codegen(ast)?;

    // 目前，我们仍然返回硬编码的汇编代码
    let assembly_code = r#"
.globl main
main:
  movl $2, %eax
  ret
"#;
    Ok(assembly_code.to_string())
}
