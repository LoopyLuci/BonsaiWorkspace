/// C ABI System V AMD64 / Windows x64 Calling Convention
///
/// This module implements the C ABI calling convention for x86-64 architectures,
/// enabling interoperability between all programming languages.

use std::ffi::{c_void, c_char};

/// Language identifier constants
pub const LANG_C: u32 = 0x00000001;
pub const LANG_RUST: u32 = 0x00000002;
pub const LANG_PYTHON: u32 = 0x00000003;
pub const LANG_JAVASCRIPT: u32 = 0x00000004;
pub const LANG_JAVA: u32 = 0x00000005;
pub const LANG_GO: u32 = 0x00000006;
pub const LANG_TYPESCRIPT: u32 = 0x00000007;
pub const LANG_CSHARP: u32 = 0x00000008;
pub const LANG_CPP: u32 = 0x00000009;
pub const LANG_SWIFT: u32 = 0x0000000A;
pub const LANG_KOTLIN: u32 = 0x0000000B;
pub const LANG_SCALA: u32 = 0x0000000C;
pub const LANG_HASKELL: u32 = 0x0000000D;
pub const LANG_CLOJURE: u32 = 0x0000000E;
pub const LANG_ELIXIR: u32 = 0x0000000F;

/// Type identifiers
pub const TYPE_VOID: u32 = 0x00;
pub const TYPE_BOOL: u32 = 0x01;
pub const TYPE_I8: u32 = 0x02;
pub const TYPE_I16: u32 = 0x03;
pub const TYPE_I32: u32 = 0x04;
pub const TYPE_I64: u32 = 0x05;
pub const TYPE_U8: u32 = 0x06;
pub const TYPE_U16: u32 = 0x07;
pub const TYPE_U32: u32 = 0x08;
pub const TYPE_U64: u32 = 0x09;
pub const TYPE_F32: u32 = 0x0A;
pub const TYPE_F64: u32 = 0x0B;
pub const TYPE_PTR: u32 = 0x0C;
pub const TYPE_STR: u32 = 0x0D;
pub const TYPE_STRUCT: u32 = 0x0E;
pub const TYPE_ARRAY: u32 = 0x0F;

/// System V AMD64 ABI parameter passing
///
/// First 6 integer/pointer arguments: RDI, RSI, RDX, RCX, R8, R9
/// First 8 float arguments: XMM0-XMM7
/// Return value: RAX (+ RDX for 128-bit)
#[repr(C)]
pub enum RegisterClass {
    // Integer registers (System V AMD64 ABI)
    RDI,  // First integer argument
    RSI,  // Second integer argument
    RDX,  // Third integer argument
    RCX,  // Fourth integer argument
    R8,   // Fifth integer argument
    R9,   // Sixth integer argument

    // Vector registers (XMM0-XMM7)
    XMM0, // First float argument
    XMM1, // Second float argument
    XMM2, // Third float argument
    XMM3, // Fourth float argument
    XMM4, // Fifth float argument
    XMM5, // Sixth float argument
    XMM6, // Seventh float argument
    XMM7, // Eighth float argument

    // Return value registers
    RAX,  // Integer return

    // Stack
    Stack,
}

/// ABI Context for function calls
#[repr(C)]
pub struct ABIContext {
    pub language: u32,
    pub architecture: u32,
    pub pointer_size: u32,
    pub call_convention: CallConvention,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CallConvention {
    /// System V AMD64 (Unix/Linux/macOS)
    SystemVAmd64,
    /// Microsoft x64 (Windows)
    MicrosoftX64,
    /// ARM64 AAPCS (Apple/Mobile)
    ARM64AAPCS,
    /// RISC-V ABI
    RISCVABI,
}

impl ABIContext {
    pub fn new_systemv() -> Self {
        ABIContext {
            language: 0,
            architecture: 0x86_64,
            pointer_size: 8,
            call_convention: CallConvention::SystemVAmd64,
        }
    }

    pub fn new_microsoft() -> Self {
        ABIContext {
            language: 0,
            architecture: 0x86_64,
            pointer_size: 8,
            call_convention: CallConvention::MicrosoftX64,
        }
    }

    pub fn new_arm64() -> Self {
        ABIContext {
            language: 0,
            architecture: 0xAA64, // ARM64 architecture ID
            pointer_size: 8,
            call_convention: CallConvention::ARM64AAPCS,
        }
    }
}

/// Parameter passing information
#[repr(C)]
pub struct ParameterInfo {
    pub type_id: u32,
    pub register_class: RegisterClass,
    pub offset: u32, // For stack parameters
    pub size: u32,
}

/// Function signature for C ABI
#[repr(C)]
pub struct CFunctionSignature {
    pub name: *const c_char,
    pub return_type: u32,
    pub parameters: *const ParameterInfo,
    pub param_count: usize,
    pub is_variadic: bool,
}

impl CFunctionSignature {
    pub fn new() -> Self {
        CFunctionSignature {
            name: std::ptr::null(),
            return_type: TYPE_VOID,
            parameters: std::ptr::null(),
            param_count: 0,
            is_variadic: false,
        }
    }
}

/// Calling convention detection at runtime
pub fn detect_calling_convention() -> CallConvention {
    #[cfg(target_os = "windows")]
    {
        CallConvention::MicrosoftX64
    }

    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "aarch64") {
            CallConvention::ARM64AAPCS
        } else {
            CallConvention::SystemVAmd64
        }
    }

    #[cfg(target_os = "linux")]
    {
        CallConvention::SystemVAmd64
    }

    #[cfg(target_arch = "aarch64")]
    {
        CallConvention::ARM64AAPCS
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        CallConvention::SystemVAmd64 // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_context_systemv() {
        let ctx = ABIContext::new_systemv();
        assert_eq!(ctx.architecture, 0x86_64);
        assert_eq!(ctx.pointer_size, 8);
    }

    #[test]
    fn test_abi_context_microsoft() {
        let ctx = ABIContext::new_microsoft();
        assert_eq!(ctx.architecture, 0x86_64);
    }

    #[test]
    fn test_language_constants() {
        assert_ne!(LANG_C, LANG_RUST);
        assert_ne!(LANG_PYTHON, LANG_JAVASCRIPT);
        assert_ne!(LANG_GO, LANG_JAVA);
    }

    #[test]
    fn test_type_constants() {
        assert_ne!(TYPE_I32, TYPE_I64);
        assert_ne!(TYPE_F32, TYPE_F64);
        assert_ne!(TYPE_PTR, TYPE_STR);
    }
}
