fn main() {
    tauri_build::build();

    // Re-run if the features source changes so bindings stay current
    println!("cargo:rerun-if-changed=src/features.rs");

    // Client-only stubs for the OmniHarness Rust kernel's gRPC substrate
    // (event store / model registry / harness status). Kernel itself
    // (../../kernel) generates the server side from the same file.
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["../../proto/omniharness.proto"], &["../../proto"])
        .expect("failed to compile omniharness.proto for kernel_bridge client stubs");
}
