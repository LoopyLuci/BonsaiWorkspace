//! omnicc — real OmniCC v1 CLI.
//!
//! Genuinely parses source in one of two real front-end languages (a
//! Sylva-subset surface syntax via `ir::parser`, or real Titan source via
//! `ir::titan_lower`, which itself calls the real `titan` crate parser) into
//! one shared `IrModule`, and emits real, compilable Rust source from it via
//! `ir::RustCodegen`. This does not generate machine code (no ELF/PE/Mach-O
//! output) — it emits Rust source text that a real Rust toolchain (`rustc`
//! or `cargo`) then compiles.
//!
//! Usage:
//!   omnicc compile --from sylva --to rust <file> [-o out.rs]
//!   omnicc compile --from titan --to rust <file> [-o out.rs]

use ir::IrCompiler;
use std::{env, fs, path::Path, process};

fn usage() -> ! {
    eprintln!("omnicc — real UniIR-based cross-language compiler (v1: sylva, titan -> rust)");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  omnicc compile --from <sylva|titan> --to rust <file> [-o <out.rs>]");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] != "compile" {
        usage();
    }

    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
    let mut out: Option<String> = None;
    let mut file: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => {
                i += 1;
                from = args.get(i).cloned();
            }
            "--to" => {
                i += 1;
                to = args.get(i).cloned();
            }
            "-o" | "--out" => {
                i += 1;
                out = args.get(i).cloned();
            }
            other if !other.starts_with('-') => {
                file = Some(other.to_string());
            }
            other => {
                eprintln!("error: unrecognized flag `{other}`");
                usage();
            }
        }
        i += 1;
    }

    let from = from.unwrap_or_else(|| {
        eprintln!("error: --from <sylva|titan> is required");
        usage();
    });
    let to = to.unwrap_or_else(|| {
        eprintln!("error: --to <rust> is required");
        usage();
    });
    let file = file.unwrap_or_else(|| {
        eprintln!("error: no input file given");
        usage();
    });

    if to != "rust" {
        eprintln!("error: only --to rust is implemented in v1 (no machine-code backend exists)");
        process::exit(2);
    }

    let src = fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("error reading {file}: {e}");
        process::exit(1);
    });

    let module_name = Path::new(&file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_string();

    let compiler = IrCompiler::new();
    let rust_src = match from.as_str() {
        "sylva" => compiler.compile_to_rust(&src, &module_name),
        "titan" => compiler.compile_titan_to_rust(&src, &file, &module_name),
        other => {
            eprintln!("error: unknown --from language `{other}` (expected sylva or titan)");
            process::exit(2);
        }
    };

    let rust_src = rust_src.unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    match out {
        Some(path) => {
            fs::write(&path, &rust_src).unwrap_or_else(|e| {
                eprintln!("error writing {path}: {e}");
                process::exit(1);
            });
            eprintln!("wrote {path}");
        }
        None => println!("{rust_src}"),
    }
}
