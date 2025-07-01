//  src/main.rs

use clap::Parser as ClapParser;
use my_c_compiler::codegen::CodeGenerator;
use my_c_compiler::emitter;
use my_c_compiler::lexer::{self, Token};
use my_c_compiler::parser as CParser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A C compiler, written in Rust.
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // ... Cli 结构体定义不变 ...
    #[arg(long)]
    lex: bool,
    #[arg(long)]
    parse: bool,
    #[arg(long)]
    codegen: bool,
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
    // --- STAGE 1: PREPROCESSING ---
    println!("1. Preprocessing {}...", cli.input_file.display());
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

    // --- STAGE 2: LEXING ---
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

    // --- STAGE 3: PARSING (C -> C AST) ---
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

    // --- STAGE 4: CODE GENERATION (C AST -> Assembly AST) ---
    println!("\n4. Generating Assembly AST from C AST...");
    let codegen = CodeGenerator::new(c_ast);
    let asm_ast = codegen.generate()?;
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

    // --- STAGE 5: CODE EMISSION (Assembly AST -> Assembly Code String) ---
    println!("\n5. Emitting assembly code from Assembly AST...");
    let assembly_code = emitter::emit_assembly(asm_ast)?;
    let assembly_path = parent_dir.join(file_stem).with_extension("s");
    fs::write(&assembly_path, &assembly_code)?;
    println!(
        "   ✓ Assembly code emission complete: {}",
        assembly_path.display()
    );

    // --- STAGE 6: ASSEMBLE & LINK ---
    println!("\n6. Assembling and linking...");
    let output_path = parent_dir.join(file_stem);
    assemble(&assembly_path, &output_path)?;
    println!(
        "   ✓ Assembling and linking complete: {}",
        output_path.display()
    );

    // --- Cleanup ---
    fs::remove_file(&preprocessed_path)?;
    fs::remove_file(&assembly_path)?;

    println!(
        "\n✅ Success! Executable created at: {}",
        output_path.display()
    );

    Ok(())
}

// --- Helper Functions for external commands ---

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
