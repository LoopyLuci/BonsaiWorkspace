// TITAN LANGUAGE COMPILER & RUNTIME
// A next-generation systems programming language
//
// This is the main compiler entry point that orchestrates:
// 1. Lexical analysis (tokenization)
// 2. Parsing (AST construction)
// 3. Type checking & semantic analysis
// 4. Code generation / interpretation

mod lexer;
mod parser;
mod ast;
mod type_checker;
mod interpreter;
mod stdlib;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: titan run <file.titan>");
                return;
            }
            run_file(&args[2]);
        },
        "repl" => {
            run_repl();
        },
        "build" => {
            if args.len() < 3 {
                eprintln!("Usage: titan build <file.titan>");
                return;
            }
            build_file(&args[2]);
        },
        "compile" => {
            if args.len() < 3 {
                eprintln!("Usage: titan compile <file.titan>");
                return;
            }
            compile_file(&args[2]);
        },
        "--version" => {
            println!("TITAN v2.5.0");
        },
        "--help" => {
            print_help();
        },
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }
}

fn run_file(path: &str) {
    match fs::read_to_string(path) {
        Ok(source) => {
            match execute_program(&source, path) {
                Ok(_) => {},
                Err(e) => eprintln!("Error: {}", e),
            }
        },
        Err(e) => eprintln!("Failed to read file: {}", e),
    }
}

fn build_file(path: &str) {
    match fs::read_to_string(path) {
        Ok(source) => {
            match compile_program(&source, path) {
                Ok(binary_path) => println!("Built: {}", binary_path),
                Err(e) => eprintln!("Compilation error: {}", e),
            }
        },
        Err(e) => eprintln!("Failed to read file: {}", e),
    }
}

fn compile_file(path: &str) {
    match fs::read_to_string(path) {
        Ok(source) => {
            match compile_to_llvm(&source, path) {
                Ok(llvm_ir) => println!("LLVM IR generated: {} bytes", llvm_ir.len()),
                Err(e) => eprintln!("Compilation error: {}", e),
            }
        },
        Err(e) => eprintln!("Failed to read file: {}", e),
    }
}

fn run_repl() {
    println!("TITAN v2.5.0 REPL");
    println!("Type 'exit' to quit, 'help' for help");
    println!();

    loop {
        print!("titan> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();

                if input == "exit" {
                    break;
                }

                if input == "help" {
                    print_help();
                    continue;
                }

                if input.is_empty() {
                    continue;
                }

                match execute_program(input, "repl") {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error: {}", e),
                }
            },
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }

    println!("Goodbye!");
}

fn execute_program(source: &str, filename: &str) -> Result<(), String> {
    // Step 1: Lexical analysis (tokenization)
    let tokens = lexer::tokenize(source)?;

    // Step 2: Parsing (build AST)
    let ast = parser::parse(tokens)?;

    // Step 3: Type checking
    type_checker::check(&ast)?;

    // Step 4: Interpretation
    interpreter::interpret(&ast)?;

    Ok(())
}

fn compile_program(source: &str, filename: &str) -> Result<String, String> {
    // Step 1: Lexical analysis
    let tokens = lexer::tokenize(source)?;

    // Step 2: Parsing
    let ast = parser::parse(tokens)?;

    // Step 3: Type checking
    type_checker::check(&ast)?;

    // Step 4: Generate binary
    let binary = interpreter::compile_to_binary(&ast)?;

    let output_path = format!("{}.exe", filename.trim_end_matches(".titan"));
    fs::write(&output_path, binary)
        .map_err(|e| e.to_string())?;

    Ok(output_path)
}

fn compile_to_llvm(source: &str, filename: &str) -> Result<String, String> {
    // Step 1: Lexical analysis
    let tokens = lexer::tokenize(source)?;

    // Step 2: Parsing
    let ast = parser::parse(tokens)?;

    // Step 3: Type checking
    type_checker::check(&ast)?;

    // Step 4: Generate LLVM IR
    interpreter::generate_llvm_ir(&ast)
}

fn print_usage() {
    println!("TITAN v2.5.0 - Next-Generation Systems Language");
    println!();
    println!("Usage: titan <command> [options]");
    println!();
    println!("Commands:");
    println!("  run <file>     - Run a TITAN program");
    println!("  build <file>   - Build executable");
    println!("  compile <file> - Generate LLVM IR");
    println!("  repl           - Interactive REPL");
    println!("  --version      - Show version");
    println!("  --help         - Show this help");
}

fn print_help() {
    println!();
    println!("TITAN Language Help");
    println!();
    println!("Basic syntax:");
    println!("  let x = 42;              // Immutable binding");
    println!("  mut y = 10;              // Mutable binding");
    println!("  fn add(a, b) -> i32 {{   // Function definition");
    println!("    a + b");
    println!("  }}");
    println!();
    println!("Types:");
    println!("  i32, i64, f32, f64, bool, String, Array<T>, Vector<T>");
    println!();
    println!("Control flow:");
    println!("  if x > 0 {{ ... }} else {{ ... }}");
    println!("  for i in 0..10 {{ ... }}");
    println!("  while condition {{ ... }}");
    println!("  match value {{ ... }}");
    println!();
    println!("See documentation: https://docs.omnisystem.dev");
    println!();
}
