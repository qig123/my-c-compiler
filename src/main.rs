// src/main.rs

use clap::Parser as ClapParser;
use my_c_compiler::backend::{asm_gen::AsmGenerator, emitter, tacky_gen::TackyGenerator};
use my_c_compiler::lexer::{self, Token};
use my_c_compiler::parser as CParser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A C compiler, written in Rust.
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Stop after lexing and print tokens
    #[arg(long)]
    lex: bool,

    /// Stop after parsing and print C AST
    #[arg(long)]
    parse: bool,

    /// 【新增】Stop after TACKY IR generation and print TACKY
    #[arg(long)]
    tacky: bool,

    /// Stop after assembly generation and print assembly AST
    #[arg(long)]
    codegen: bool,
    /// Do not delete the generated .s assembly file
    #[arg(long)]
    keep_asm: bool,
    /// The C source file to compile
    input_file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if let Err(e) = run_pipeline(&cli) {
        eprintln!("\nCompilation failed: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

fn run_pipeline(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    // --- STAGE 1 & 2: PREPROCESSING and LEXING (不变) ---
    println!("1. Preprocessing {}...", cli.input_file.display());
    // ... (preprocessing code is correct) ...
    let input_path = &cli.input_file;
    if !input_path.exists() {
        return Err(format!("Input file not found: {}", input_path.display()).into());
    }
    let file_stem = input_path.file_stem().ok_or("Invalid input file name")?;
    let parent_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
    let preprocessed_path = parent_dir.join(file_stem).with_extension("i");
    preprocess(input_path, &preprocessed_path)?;
    let source_code = fs::read_to_string(&preprocessed_path)?;

    println!("\n2. Lexing source code...");
    let lexer = lexer::Lexer::new(&source_code);
    let tokens: Vec<Token> = lexer.collect::<Result<_, _>>()?;
    println!("   ✓ Lexing successful, found {} tokens.", tokens.len());
    if cli.lex {
        println!(
            "--- Generated Tokens ---\n{:#?}\n------------------------",
            tokens
        );
        println!("\nHalting as requested by --lex.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    // --- STAGE 3: PARSING (C -> C AST) (不变) ---
    println!("\n3. Parsing tokens into C Abstract Syntax Tree (AST)...");
    let mut parser = CParser::Parser::new(&tokens);
    let c_ast = parser.parse()?;
    println!("   ✓ Parsing successful.");
    if cli.parse {
        println!(
            "--- Generated C AST ---\n{:#?}\n---------------------",
            c_ast
        );
        println!("\nHalting as requested by --parse.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    // --- 【STAGE 4: TACKY IR GENERATION】 (C AST -> TACKY IR) ---
    println!("\n4. Generating TACKY Intermediate Representation (IR)...");
    let mut tacky_generator = TackyGenerator::new();
    let tacky_ir = tacky_generator.generate_tacky(c_ast)?; // c_ast 被消耗
    println!("   ✓ TACKY IR generation successful.");
    if cli.tacky {
        println!(
            "--- Generated TACKY IR ---\n{:#?}\n------------------------",
            tacky_ir
        );
        println!("\nHalting as requested by --tacky.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    // --- 【STAGE 5: ASSEMBLY GENERATION】 (TACKY IR -> Assembly AST) ---
    println!("\n5. Generating Assembly AST from TACKY IR...");
    let mut asm_generator = AsmGenerator::new();
    let asm_ast = asm_generator.generate_assembly(tacky_ir)?; // tacky_ir 被消耗
    println!("   ✓ Assembly AST generation successful.");
    if cli.codegen {
        println!(
            "--- Generated Assembly AST ---\n{:#?}\n--------------------------",
            asm_ast
        );
        println!("\nHalting as requested by --codegen.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }

    // --- 【STAGE 6: CODE EMISSION】 (Assembly AST -> Assembly Code String) ---
    println!("\n6. Emitting assembly code from Assembly AST...");
    let assembly_code = emitter::emit_assembly(asm_ast)?; // asm_ast 被消耗
    let assembly_path = parent_dir.join(file_stem).with_extension("s");
    fs::write(&assembly_path, &assembly_code)?;
    println!(
        "   ✓ Assembly code emission complete: {}",
        assembly_path.display()
    );

    // --- 【STAGE 7: ASSEMBLE & LINK】 ---
    println!("\n7. Assembling and linking...");
    let output_path = parent_dir.join(file_stem);
    assemble(&assembly_path, &output_path)?;
    println!(
        "   ✓ Assembling and linking complete: {}",
        output_path.display()
    );

    // --- Cleanup ---
    // 总是删除预处理文件
    fs::remove_file(&preprocessed_path)?;

    // 【核心修改】只有在没有 --keep-asm 标志时才删除 .s 文件
    if !cli.keep_asm {
        if let Err(e) = fs::remove_file(&assembly_path) {
            // 如果文件因为某些原因不存在，打印一个警告而不是让整个程序失败
            eprintln!(
                "Warning: could not remove temporary assembly file '{}': {}",
                assembly_path.display(),
                e
            );
        }
    } else {
        println!(
            "   ℹ️ Assembly file kept as requested by --keep-asm: {}",
            assembly_path.display()
        );
    }

    println!(
        "\n✅ Success! Executable created at: {}",
        output_path.display()
    );

    Ok(())
}

// --- Helper Functions for external commands (不变) ---

fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("Command `{:?}` failed with status: {}", command, status).into());
    }
    Ok(())
}

fn preprocess(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_command(
        Command::new("gcc")
            .arg("-E")
            .arg(input)
            .arg("-o")
            .arg(output),
    )
}

fn assemble(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_command(
        Command::new("gcc")
            .arg("-no-pie")
            .arg(input)
            .arg("-o")
            .arg(output),
    )
}
