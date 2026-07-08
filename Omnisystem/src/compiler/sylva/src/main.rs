// SYLVA LANGUAGE COMPILER v2.5.0
// AI/ML-First Language with Neural Networks and Automatic Differentiation

mod lexer;
mod parser;
mod interpreter;
mod neural;

use std::env;
use std::fs;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: sylva run <file.sylva>");
                return;
            }
            run_file(&args[2]);
        },
        "repl" => run_repl(),
        "--version" => println!("SYLVA v2.5.0"),
        "--help" => print_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn run_file(path: &str) {
    match fs::read_to_string(path) {
        Ok(source) => {
            match execute_program(&source) {
                Ok(_) => {},
                Err(e) => eprintln!("Error: {}", e),
            }
        },
        Err(e) => eprintln!("Failed to read file: {}", e),
    }
}

fn run_repl() {
    println!("SYLVA v2.5.0 REPL - AI/ML Language");
    println!("Type 'exit' to quit");
    println!();

    loop {
        print!("sylva> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                if input == "exit" { break; }
                if input.is_empty() { continue; }

                match execute_program(input) {
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
}

fn execute_program(source: &str) -> Result<(), String> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens)?;
    interpreter::interpret(&ast)?;
    Ok(())
}

fn print_usage() {
    println!("SYLVA v2.5.0 - AI/ML-First Language");
    println!("Usage: sylva <command> [options]");
    println!("Commands: run <file>, repl, --version, --help");
}

fn print_help() {
    println!("\nSYLVA Language Syntax:\n");
    println!("  tensor t = randn([3, 4])    // Create tensor");
    println!("  let model = model {{         // Define model");
    println!("    layer1: dense(784, 128),");
    println!("    output: dense(128, 10),");
    println!("  }}");
    println!("  let loss = train(model, data)  // Train model");
    println!("  let pred = predict(model, x)   // Inference");
    println!();
}
