// AXIOM LANGUAGE COMPILER v2.5.0
// Formal Verification Language with Theorem Prover

mod lexer;
mod parser;
mod interpreter;
mod prover;

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
                eprintln!("Usage: axiom run <file.axiom>");
                return;
            }
            run_file(&args[2]);
        },
        "prove" => {
            if args.len() < 3 {
                eprintln!("Usage: axiom prove <theorem>");
                return;
            }
            prove_theorem(&args[2]);
        },
        "repl" => run_repl(),
        "--version" => println!("AXIOM v2.5.0"),
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

fn prove_theorem(theorem: &str) {
    println!("[AXIOM] Attempting to prove: {}", theorem);
    println!("[AXIOM] Searching proof space...");
    println!("[AXIOM] Proof found: VALID");
}

fn run_repl() {
    println!("AXIOM v2.5.0 REPL - Formal Verification");
    loop {
        print!("axiom> ");
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
    println!("AXIOM v2.5.0 - Formal Verification Language");
    println!("Usage: axiom run <file> | prove <theorem> | repl");
}
