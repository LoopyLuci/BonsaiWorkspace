// Build script for bonsai-mobile-ffi
//
// This script configures the crate for Android NDK compilation using cargo-ndk.
// It ensures the crate compiles to a native .so library suitable for JNI use.

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let host = env::var("HOST").unwrap_or_default();

    // Android-specific configuration
    if target.contains("android") {
        println!("cargo:warning=Building for Android target: {}", target);

        // Set up proper flags for Android NDK
        if let Ok(ndk_path) = env::var("NDK_PATH") {
            println!("cargo:warning=Using NDK at: {}", ndk_path);
        }

        // Android targets require specific linker flags
        match target.as_str() {
            "aarch64-linux-android" => {
                println!("cargo:rustc-link-arg=-lc");
            }
            "armv7-linux-android" => {
                println!("cargo:rustc-link-arg=-lc");
            }
            "x86_64-linux-android" => {
                println!("cargo:rustc-link-arg=-lc");
            }
            _ => {}
        }
    } else if host.contains("android") {
        // Building on Android itself
        println!("cargo:warning=Building on Android");
    }

    // Generate proper cdylib metadata for JNI
    println!("cargo:rustc-cfg=jni");

    // Link to Android libraries when cross-compiling for Android
    if target.contains("android") && !host.contains("android") {
        println!("cargo:rustc-link-lib=dylib=c");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=m");
    }
}
