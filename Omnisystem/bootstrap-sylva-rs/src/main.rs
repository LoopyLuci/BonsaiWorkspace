//! Sylva seed — the Sylva bootstrap interpreter driver (Rust host).
//!
//!     sylva-seed run    program.sylva
//!     sylva-seed tokens program.sylva
//!     sylva-seed test   [dir]

mod ast;
mod builtins;
mod diag;
mod interp;
mod lexer;
mod parser;
mod values;

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use diag::OmniError;
use interp::Interp;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn read_source(file: &str) -> Result<String, String> {
    fs::read_to_string(file).map_err(|e| format!("cannot read file '{file}': {e}"))
}

fn parse_file(src: &str, file: &str) -> Result<ast::Module, Box<OmniError>> {
    let toks = lexer::Lexer::new(src, file).tokenize()?;
    parser::Parser::new(toks, file).parse_module()
}

fn report(e: &OmniError, src: &str) {
    eprintln!("\n{}", e.render(src));
}

fn cmd_run(file: &str) -> i32 {
    let src = match read_source(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let module = match parse_file(&src, file) {
        Ok(m) => m,
        Err(e) => {
            report(&e, &src);
            return 1;
        }
    };
    let mut intr = Interp::new(file);
    builtins::install_globals(&intr);
    match intr.run_module(&module) {
        Ok(code) => {
            print!("{}", intr.out);
            code
        }
        Err(e) => {
            print!("{}", intr.out);
            report(&e, &src);
            1
        }
    }
}

fn cmd_tokens(file: &str) -> i32 {
    let src = match read_source(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    match lexer::Lexer::new(&src, file).tokenize() {
        Ok(toks) => {
            for t in toks {
                println!("{}:{}\t{:?}\t{:?}", t.span.start.line, t.span.start.col, t.kind, t.value);
            }
            0
        }
        Err(e) => {
            report(&e, &src);
            1
        }
    }
}

/// Test files declare expectations as comments:
///   #@ expect-stdout: <exact text>   (may repeat for multiple lines)
///   #@ expect-exit: <code>
///   #@ expect-error
fn cmd_test(dir: &str) -> i32 {
    let Ok(rd) = fs::read_dir(dir) else {
        eprintln!("error: no test directory '{dir}'");
        return 2;
    };
    let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "sylva")).collect();
    files.sort();
    let (mut pass, mut fail) = (0, 0);
    for file in &files {
        let src = fs::read_to_string(file).unwrap_or_default();
        let want_err = src.contains("#@ expect-error");
        let expect_stdout: Vec<&str> =
            src.lines().filter_map(|l| l.split("#@ expect-stdout:").nth(1)).map(|s| s.strip_prefix(' ').unwrap_or(s)).collect();
        let expect_exit: Option<i32> = src.lines().find_map(|l| l.split("#@ expect-exit:").nth(1)).and_then(|s| s.trim().parse().ok());

        let fname = file.file_name().unwrap_or_default().to_string_lossy().to_string();
        let path_str = file.to_string_lossy().to_string();

        let mut stdout = String::new();
        let mut exit = 0;
        let mut errored = false;
        match parse_file(&src, &path_str) {
            Ok(module) => {
                let mut intr = Interp::new(&path_str);
                builtins::install_globals(&intr);
                match intr.run_module(&module) {
                    Ok(code) => {
                        exit = code;
                        stdout = intr.out.clone();
                    }
                    Err(_) => {
                        errored = true;
                        stdout = intr.out.clone();
                    }
                }
            }
            Err(_) => errored = true,
        }

        let mut problems = Vec::new();
        if want_err && !errored {
            problems.push("expected an error but program succeeded".to_string());
        }
        if !want_err && errored {
            problems.push("unexpected error".to_string());
        }
        if !expect_stdout.is_empty() {
            let got = stdout.trim_end_matches('\n');
            let want = expect_stdout.join("\n");
            if got != want {
                problems.push(format!("stdout mismatch\n     expected: {want:?}\n     got:      {got:?}"));
            }
        }
        if let Some(we) = expect_exit {
            if exit != we {
                problems.push(format!("exit {exit}, expected {we}"));
            }
        }
        if problems.is_empty() {
            pass += 1;
            println!("  \u{2713} {fname}");
        } else {
            fail += 1;
            println!("  \u{2717} {fname}");
            for p in problems {
                println!("      {}", p.replace('\n', "\n      "));
            }
        }
    }
    println!();
    if fail == 0 {
        println!("{pass}/{pass} passed");
        0
    } else {
        println!("{pass} passed, {fail} failed");
        1
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("run") => args.get(1).map(|f| cmd_run(f)).unwrap_or(2),
        Some("tokens") => args.get(1).map(|f| cmd_tokens(f)).unwrap_or(2),
        Some("test") => cmd_test(args.get(1).map(String::as_str).unwrap_or("tests")),
        _ => {
            println!("Sylva seed {VERSION}\n\nUsage: sylva-seed <run|tokens|test> [file|dir]");
            0
        }
    };
    ExitCode::from(code as u8)
}
