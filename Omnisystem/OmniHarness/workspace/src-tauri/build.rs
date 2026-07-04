fn main() {
    tauri_build::build();

    // Re-run if the features source changes so bindings stay current
    println!("cargo:rerun-if-changed=src/features.rs");
}
