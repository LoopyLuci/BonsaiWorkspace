// AETHER LANGUAGE COMPILER v2.5.0
// Distributed Systems Language with Actor Model

mod lexer;
mod parser;
mod interpreter;
mod actor_system;

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
                eprintln!("Usage: aether run <file.aether>");
                return;
            }
            run_file(&args[2]);
        },
        "repl" => run_repl(),
        "--version" => println!("AETHER v2.5.0"),
        _ => print_usage(),
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
    println!("AETHER v2.5.0 REPL - Distributed Systems");
    loop {
        print!("aether> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if input.trim() == "exit" { break; }
        let _ = execute_program(&input);
    }
}

fn execute_program(source: &str) -> Result<(), String> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens)?;
    interpreter::interpret(&ast)?;
    Ok(())
}

fn print_usage() {
    println!("AETHER v2.5.0 - Distributed Systems Language");
    println!("Usage: aether run <file> | repl");
}
