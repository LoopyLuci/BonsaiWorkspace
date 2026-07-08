//! OmniCC seed — the Omnisystem bootstrap compiler driver (Rust host).
//!
//!     omnicc-seed run    program.titan
//!     omnicc-seed check  program.titan
//!     omnicc-seed build  program.titan
//!     omnicc-seed tokens program.titan
//!     omnicc-seed test   [dir]

mod ast;
mod builtins;
mod diag;
mod interp;
mod lexer;
mod parser;
mod values;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use diag::OmniError;
use interp::Interp;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn read_source(file: &str) -> Result<String, String> {
    fs::read_to_string(file).map_err(|e| format!("cannot read file '{file}': {e}"))
}

/// Load a root file plus any `mod name;` sibling files into one item list.
fn link(root: &str) -> Result<(Vec<ast::Item>, String), Box<OmniError>> {
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut queue = vec![PathBuf::from(root)];
    let mut root_src = String::new();
    while let Some(file) = queue.pop() {
        let key = file.to_string_lossy().to_string();
        if !seen.insert(key.clone()) {
            continue;
        }
        let src = fs::read_to_string(&file).map_err(|e| {
            Box::new(OmniError::new(diag::Phase::Parse, format!("cannot read '{key}': {e}"), diag::Span::point(1, 1), &key))
        })?;
        if root_src.is_empty() {
            root_src = src.clone();
        }
        let prog = parser::parse(&src, &key).map_err(Box::new)?;
        for item in &prog.items {
            if let ast::Item::Mod(m) = item {
                if m.items.is_empty() {
                    let dir = file.parent().unwrap_or(Path::new("."));
                    for cand in [dir.join(format!("{}.titan", m.name)), dir.join(&m.name).join("mod.titan")] {
                        if cand.exists() {
                            queue.push(cand);
                            break;
                        }
                    }
                }
            }
        }
        items.extend(prog.items);
    }
    Ok((items, root_src))
}

fn report(e: &OmniError, src_cache: &str) {
    eprintln!("\n{}", e.render(&fs::read_to_string(&e.file).unwrap_or_else(|_| src_cache.to_string())));
}

fn cmd_new(name: &str) -> i32 {
    let dir = std::path::Path::new(name);
    if dir.exists() {
        eprintln!("error: '{name}' already exists");
        return 1;
    }
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("error: could not create '{name}': {e}");
        return 1;
    }
    let main_src = "pub fn main() {\n    println!(\"hello from Titan\")\n}\n";
    if let Err(e) = std::fs::write(dir.join("main.titan"), main_src) {
        eprintln!("error: could not write main.titan: {e}");
        return 1;
    }
    println!("Created '{name}/main.titan'. Run it with:\n  titan run {name}/main.titan");
    0
}

fn cmd_run(file: &str) -> i32 {
    let (items, src) = match link(file) {
        Ok(v) => v,
        Err(e) => {
            report(&e, "");
            return 1;
        }
    };
    let mut intr = Interp::new(file);
    if let Err(e) = intr.register(&items) {
        report(&e, &src);
        return 1;
    }
    match intr.run_main() {
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

fn cmd_check(file: &str, quiet: bool) -> i32 {
    let (items, src) = match link(file) {
        Ok(v) => v,
        Err(e) => {
            report(&e, "");
            return 1;
        }
    };
    let mut intr = Interp::new(file);
    if let Err(e) = intr.register(&items) {
        report(&e, &src);
        return 1;
    }
    if !quiet {
        let methods: usize = intr.methods.values().map(|t| t.len()).sum();
        println!(
            "\u{2713} check passed  ({} structs, {} enums, {} fns, {} methods)",
            intr.structs.len(),
            intr.enums.len(),
            intr.fns.len(),
            methods
        );
    }
    0
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

fn cmd_build(file: &str) -> i32 {
    println!("OmniCC seed {VERSION} — Omnisystem bootstrap compiler (Titan, Rust host)");
    let t0 = std::time::Instant::now();
    let rc = cmd_check(file, true);
    if rc == 0 {
        println!("\u{2713} build ready  {} compiled in {:?}", file, t0.elapsed());
    }
    rc
}

/// Test files declare expectations as comments:
///   //@ expect-stdout: <exact text>   (may repeat for multiple lines)
///   //@ expect-exit: <code>
///   //@ expect-error
fn cmd_test(dir: &str) -> i32 {
    let Ok(rd) = fs::read_dir(dir) else {
        eprintln!("error: no test directory '{dir}'");
        return 2;
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "titan"))
        .collect();
    files.sort();
    let (mut pass, mut fail) = (0, 0);
    for file in &files {
        let src = fs::read_to_string(file).unwrap_or_default();
        let want_err = src.contains("//@ expect-error");
        let expect_stdout: Vec<&str> = src
            .lines()
            .filter_map(|l| l.split("//@ expect-stdout:").nth(1))
            .map(|s| s.strip_prefix(' ').unwrap_or(s))
            .collect();
        let expect_exit: Option<i32> =
            src.lines().find_map(|l| l.split("//@ expect-exit:").nth(1)).and_then(|s| s.trim().parse().ok());

        let fname = file.file_name().unwrap_or_default().to_string_lossy().to_string();
        let path_str = file.to_string_lossy().to_string();

        let mut stdout = String::new();
        let mut exit = 0;
        let mut errored = false;
        match link(&path_str) {
            Ok((items, _)) => {
                let mut intr = Interp::new(&path_str);
                if intr.register(&items).is_err() {
                    errored = true;
                } else {
                    match intr.run_main() {
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
    // Deep ASTs (e.g. the Titan-written compiler compiling Titan) recurse well
    // past Windows' 1 MB default stack — run the driver on a 512 MB stack.
    std::thread::Builder::new()
        .name("omnicc-main".into())
        .stack_size(512 * 1024 * 1024)
        .spawn(real_main)
        .expect("spawn omnicc thread")
        .join()
        .expect("omnicc thread panicked")
}

fn real_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("run") => args.get(1).map(|f| cmd_run(f)).unwrap_or(2),
        Some("check") => args.get(1).map(|f| cmd_check(f, false)).unwrap_or(2),
        Some("build") => args.get(1).map(|f| cmd_build(f)).unwrap_or(2),
        Some("tokens") => args.get(1).map(|f| cmd_tokens(f)).unwrap_or(2),
        Some("test") => cmd_test(args.get(1).map(String::as_str).unwrap_or("tests")),
        Some("new") => args.get(1).map(|n| cmd_new(n)).unwrap_or(2),
        Some("version") | Some("--version") | Some("-v") => {
            println!("OmniCC seed {VERSION} — Omnisystem bootstrap compiler (Titan, Rust host)");
            0
        }
        _ => {
            println!("OmniCC seed {VERSION}\n\nUsage: titan <run|check|build|tokens|test|new> [file|dir|name]");
            2
        }
    };
    ExitCode::from(code.clamp(0, 255) as u8)
}
