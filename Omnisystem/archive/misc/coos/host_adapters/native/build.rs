use std::env;
use std::path::PathBuf;

fn main() {
    // LLAMA_CPP_PATH must point to a built llama.cpp directory containing
    // libllama.a (or llama.lib on Windows) and ggml.h / llama.h headers.
    let llama_path = match env::var("LLAMA_CPP_PATH") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            // Soft-fail: without the path we can't link, but we can still
            // compile the crate so cargo check / clippy work offline.
            println!("cargo:warning=LLAMA_CPP_PATH not set; bonsai-native will not link llama.cpp");
            return;
        }
    };

    let lib_dir = llama_path.join("build").join("src");
    let ggml_lib_dir = llama_path.join("build").join("ggml").join("src");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-search=native={}", ggml_lib_dir.display());
    println!("cargo:rustc-link-lib=static=llama");
    println!("cargo:rustc-link-lib=static=ggml");

    // Vulkan runtime — must be installed on the system
    println!("cargo:rustc-link-lib=vulkan");

    // On Windows, also link against the C++ runtime
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=stdc++");
    }

    println!("cargo:rerun-if-env-changed=LLAMA_CPP_PATH");
    println!("cargo:rerun-if-changed=build.rs");
}
