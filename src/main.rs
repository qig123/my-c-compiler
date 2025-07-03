// src/main.rs

use clap::Parser as ClapParser;
use my_c_compiler::backend::asm_gen::AsmGenerator;
use my_c_compiler::backend::tacky_gen::TackyGenerator;
use my_c_compiler::common::UniqueIdGenerator;
use my_c_compiler::lexer::{self, Token};
use my_c_compiler::parser as CParser;
use my_c_compiler::semantics::loop_labeler::LoopLabeler;
use my_c_compiler::semantics::type_checker::TypeChecker;
use my_c_compiler::semantics::validator::Validator;
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

    #[arg(long)]
    validate: bool,
    /// 【新增】Stop after TACKY IR generation and print TACKY
    #[arg(long)]
    tacky: bool,

    /// Stop after assembly generation and print assembly AST
    #[arg(long)]
    codegen: bool,
    /// Do not delete the generated .s assembly file
    #[arg(long)]
    keep_asm: bool,
    /// Only compile and assemble, do not link. Produces a .o object file.
    #[arg(short = 'c')]
    compile_only: bool,
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
    let mut id_generator = UniqueIdGenerator::new();

    // --- STAGE 1 & 2: PREPROCESSING and LEXING ---
    println!("1. Preprocessing {}...", cli.input_file.display());
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
    let tokens: Vec<Token> = lexer::Lexer::new(&source_code).collect::<Result<_, _>>()?;
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

    // --- STAGE 3: PARSING ---
    println!("\n3. Parsing tokens into C Abstract Syntax Tree (AST)...");
    let c_ast = CParser::Parser::new(&tokens).parse()?;
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

    // --- STAGE 4: SEMANTIC ANALYSIS ---
    println!("\n4. Performing semantic analysis...");

    // --- Pass 1: Identifier Resolution ---
    let mut validator = Validator::new(&mut id_generator);
    // validate_program 接受 unchecked AST 并返回一个新的、名字被解析过的 unchecked AST。
    let name_resolved_ast = validator.validate_program(c_ast)?;
    println!("   - Pass 1: Identifier resolution complete.");
    // --- Pass 2: Type Checking ---
    let mut type_checker = TypeChecker::new();
    // check_program 接收一个引用，它不修改 AST，但会返回 Result 来报告错误。
    // 我们必须处理这个 Result！使用 `?` 可以让程序在出错时提前返回。
    type_checker.check_program(&name_resolved_ast)?;
    println!("   - Pass 2: Type checking complete.");
    // 此时，type_checker.symbols 中包含了所有标识符的类型信息，
    // 未来可以传递给代码生成器。
    // --- Pass 3: Loop Labeling ---
    let mut labeler = LoopLabeler::new(&mut id_generator);
    // label_program 接收 name_resolved_ast 并将其转换为最终的 checked_ast。
    let checked_ast = labeler.label_program(name_resolved_ast)?;
    println!("   - Pass 3: Loop labeling complete.");
    // --- Semantic Analysis Succeeded ---
    println!("   ✓ Semantic analysis successful.");

    if cli.validate {
        println!(
            "--- Final Checked AST ---\n{:#?}\n---------------------",
            checked_ast
        );
        println!("\nHalting as requested by --validate.");
        fs::remove_file(&preprocessed_path)?;
        return Ok(());
    }
    // // --- STAGE 5 & 6 & 7: CODE GENERATION ---
    println!("\n5. Generating TACKY Intermediate Representation (IR)...");
    let mut tacky_generator = TackyGenerator::new(&mut id_generator);
    let tacky_ir = tacky_generator.generate_tacky(checked_ast)?;
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

    println!("\n6. Generating Assembly AST from TACKY IR...");
    let mut asm_generator = AsmGenerator::new();
    let asm_ast = asm_generator.generate_assembly(tacky_ir)?;
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

    // println!("\n7. Emitting assembly code from Assembly AST...");
    // let assembly_code = emitter::emit_assembly(asm_ast)?;
    // let assembly_path = parent_dir.join(file_stem).with_extension("s");
    // fs::write(&assembly_path, &assembly_code)?;
    // println!(
    //     "   ✓ Assembly code emission complete: {}",
    //     assembly_path.display()
    // );

    // // --- STAGE 8: ASSEMBLE or LINK ---
    // if cli.compile_only {
    //     println!("\n8. Assembling to object file (-c flag detected)...");
    //     let output_path = parent_dir.join(file_stem).with_extension("o");
    //     assemble_to_object(&assembly_path, &output_path)?;
    //     println!("   ✓ Assembling complete: {}", output_path.display());
    // } else {
    //     println!("\n8. Assembling and linking...");
    //     let output_path = parent_dir.join(file_stem);
    //     link_to_executable(&assembly_path, &output_path)?;
    //     println!(
    //         "   ✓ Assembling and linking complete: {}",
    //         output_path.display()
    //     );
    // }

    // // --- Cleanup ---
    // fs::remove_file(&preprocessed_path)?;
    // if !cli.keep_asm {
    //     if let Err(e) = fs::remove_file(&assembly_path) {
    //         eprintln!(
    //             "Warning: could not remove temporary assembly file '{}': {}",
    //             assembly_path.display(),
    //             e
    //         );
    //     }
    // } else {
    //     println!(
    //         "   ℹ️ Assembly file kept as requested by --keep-asm: {}",
    //         assembly_path.display()
    //     );
    // }

    // if cli.compile_only {
    //     println!(
    //         "\n✅ Success! Object file created at: {}",
    //         parent_dir.join(file_stem).with_extension("o").display()
    //     );
    // } else {
    //     println!(
    //         "\n✅ Success! Executable created at: {}",
    //         parent_dir.join(file_stem).display()
    //     );
    // }

    Ok(())
}

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

fn link_to_executable(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_command(
        Command::new("gcc")
            .arg("-no-pie")
            .arg(input)
            .arg("-o")
            .arg(output),
    )
}

fn assemble_to_object(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_command(
        Command::new("gcc")
            .arg("-c")
            .arg(input)
            .arg("-o")
            .arg(output),
    )
}
