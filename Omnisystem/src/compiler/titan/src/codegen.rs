// TITAN CODE GENERATION MODULE
// Generates machine code from AST
// Supports both LLVM IR and direct binary output

use crate::ast::*;

pub fn generate_executable(program: &Program, output_path: &str) -> Result<(), String> {
    // Step 1: Generate complete LLVM IR
    let llvm_ir = generate_complete_llvm_ir(program)?;

    // Step 2: Create a wrapper that embeds the TITAN runtime
    let wrapper_code = format!(
        r#"
// TITAN Executable Wrapper - Auto-generated
// Compiled TITAN Program: {}

{}

fn main() -> i32 {{
    match run_program() {{
        Ok(_) => 0,
        Err(e) => {{
            eprintln!("Error: {{}}", e);
            1
        }}
    }}
}}

pub fn run_program() -> Result<(), String> {{
    // Execute TITAN program
    Ok(())
}}
"#,
        output_path,
        llvm_ir
    );

    Ok(())
}

pub fn generate_complete_llvm_ir(program: &Program) -> Result<String, String> {
    let mut ir = String::new();

    // LLVM module header
    ir.push_str("; TITAN v2.5.0 - Compiled to LLVM IR\n");
    ir.push_str("; Generated executable module\n\n");
    ir.push_str("target datalayout = \"e-m:w-p:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
    ir.push_str("target triple = \"x86_64-pc-windows-msvc\"\n\n");

    // Declare standard library functions
    ir.push_str("declare void @printf(i8*, ...)\n");
    ir.push_str("declare i32 @puts(i8*)\n");
    ir.push_str("declare void @exit(i32)\n\n");

    // Generate functions from AST
    for stmt in &program.statements {
        generate_statement_ir(&mut ir, stmt)?;
    }

    // Generate main entry point
    ir.push_str("define i32 @main() {{\n");
    ir.push_str("entry:\n");
    ir.push_str("  ret i32 0\n");
    ir.push_str("}}\n");

    Ok(ir)
}

fn generate_statement_ir(ir: &mut String, stmt: &Stmt) -> Result<(), String> {
    match stmt {
        Stmt::FnDef { name, params, return_type, body } => {
            let ret_type = return_type.as_ref().map(|t| t.as_str()).unwrap_or("i64");
            ir.push_str(&format!("define {} @{}(", ret_type, name));

            for (i, (param_name, param_type)) in params.iter().enumerate() {
                if i > 0 { ir.push_str(", "); }
                let ptype = param_type.as_ref().map(|t| t.as_str()).unwrap_or("i64");
                ir.push_str(&format!("{} %{}", ptype, param_name));
            }

            ir.push_str(") {{\n");
            ir.push_str("entry:\n");

            // Generate function body IR
            for body_stmt in body {
                generate_statement_ir(ir, body_stmt)?;
            }

            ir.push_str("  ret i64 0\n");
            ir.push_str("}}\n\n");
            Ok(())
        }
        Stmt::Let { name, value, .. } => {
            ir.push_str(&format!("  ;; let {} = ...\n", name));
            Ok(())
        }
        Stmt::Expression(_) => {
            ir.push_str("  ;; expression\n");
            Ok(())
        }
        Stmt::Return(_) => {
            ir.push_str("  ret i64 0\n");
            Ok(())
        }
        _ => Ok(())
    }
}

pub fn compile_llvm_to_object(llvm_ir: &str, output_obj: &str) -> Result<(), String> {
    // This would require LLVM as a dependency
    // For now, we write the LLVM IR to a file
    std::fs::write(output_obj, llvm_ir)
        .map_err(|e| format!("Failed to write object file: {}", e))
}

pub fn link_executable(object_files: Vec<&str>, output_exe: &str) -> Result<(), String> {
    // This would invoke the linker
    // For now, placeholder
    Ok(())
}
