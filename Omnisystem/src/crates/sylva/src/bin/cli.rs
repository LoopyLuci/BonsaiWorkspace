//! Sylva CLI — runs a `.sy` script file, or drops into a REPL with no arguments.

use std::env;
use std::fs;
use std::io::{self, Write};

use sylva::parser;
use sylva::vm::SylvaVm;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1) {
        Some(path) => run_file(path),
        None => repl(),
    }
}

fn run_file(path: &str) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {path}: {e}");
            std::process::exit(1);
        }
    };

    let mut vm = SylvaVm::new();
    match vm.eval_str(&src) {
        Ok(val) => println!("{val}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn repl() {
    println!("Sylva REPL — Ctrl+D to exit");
    let mut vm = SylvaVm::new();
    let stdin = io::stdin();

    loop {
        print!("sylva> ");
        if io::stdout().flush().is_err() {
            break;
        }

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Err(e) => {
                eprintln!("error reading stdin: {e}");
                break;
            }
            _ => {}
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try as a bare expression first (`1 + 2`); fall back to module-level
        // items (`let`/`fn` defs), which is what `eval_str` parses.
        match parser::parse_expr(line) {
            Ok(expr) => match vm.eval_expr(&expr) {
                Ok(val) => println!("{val}"),
                Err(e) => println!("error: {e}"),
            },
            Err(_) => match vm.eval_str(line) {
                Ok(val) => println!("{val}"),
                Err(e) => println!("error: {e}"),
            },
        }
    }
}
