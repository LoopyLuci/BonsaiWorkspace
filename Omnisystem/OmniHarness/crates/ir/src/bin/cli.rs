//! CLI — parse a Sylva-subset source file into UniIR and emit generated Rust.

use ir::IrCompiler;
use std::{env, fs, path::Path, process};

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: ir_cli <source.sylva>");
            process::exit(1);
        }
    };

    let src = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error reading {path}: {e}");
        process::exit(1);
    });

    let module_name = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");

    let compiler = IrCompiler::new();
    let rust_src = compiler.compile_to_rust(&src, module_name).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    println!("{rust_src}");
    eprintln!("// {}", compiler.stats().status);
}
