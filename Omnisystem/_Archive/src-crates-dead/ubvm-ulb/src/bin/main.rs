/// TestL Compiler CLI
use clap::Parser;
use ubvm_ulb::{TestModule, Function, Statement, ULBEngine};

#[derive(Parser)]
#[command(name = "testl-compile", about = "Compile TestL specs to target languages")]
struct Args {
    /// TestL source file
    #[arg(long, short)]
    source: String,

    /// Target language (or 'all')
    #[arg(long, short, default_value = "rust")]
    language: String,

    /// Output directory
    #[arg(long, short)]
    output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Create a simple test module
    let module = TestModule {
        name: "generated_test".into(),
        functions: vec![Function {
            name: "test_main".into(),
            params: vec![],
            body: vec![Statement::Print("Hello from TestL".into())],
        }],
    };

    let engine = ULBEngine::new();

    if args.language == "all" {
        println!("Compiling to all languages...");
        let results = engine.compile_all(&module);
        for (lang, result) in results {
            match result {
                Ok(code) => {
                    println!("\n=== {} ===", lang);
                    println!("{}", code);
                }
                Err(e) => eprintln!("Error compiling {}: {}", lang, e),
            }
        }
    } else {
        match engine.compile(&module, &args.language) {
            Ok(code) => {
                println!("Generated {} code:", args.language);
                println!("{}", code);
                if let Some(out) = args.output {
                    std::fs::write(&out, code)?;
                    println!("Wrote to {}", out);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
