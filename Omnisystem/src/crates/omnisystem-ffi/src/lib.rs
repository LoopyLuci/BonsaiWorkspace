//! Omnisystem FFI - cross-language foreign function interface primitives
//!
//! - [`abi`]: C ABI calling-convention constants and runtime detection
//!   (System V AMD64, Microsoft x64, ARM64 AAPCS)
//! - [`types`]: a cross-language FFI type system (size/alignment/name/
//!   compatibility for primitive and composite types)
//! - [`marshaling`]: byte-level encode/decode between Rust values and the
//!   FFI wire format
//! - [`callbacks`]: registering and invoking C function pointers and Rust
//!   closures across the FFI boundary
//! - [`versioning`]: semantic versioning and dependency compatibility
//!   checks for FFI modules

pub mod abi;
pub mod callbacks;
pub mod marshaling;
pub mod types;
pub mod versioning;

pub use abi::{
    detect_calling_convention, ABIContext, CFunctionSignature, CallConvention, ParameterInfo,
    RegisterClass,
};
pub use callbacks::{
    AsyncCallback, AsyncCallbackManager, Callback, CallbackHandle, CallbackManager,
};
pub use marshaling::{MarshalError, MarshalResult, Marshaler};
pub use types::{type_alignment, type_name, type_size, is_compatible_type, FFIType, TypeInfo};
pub use versioning::{APIVersion, ModuleVersion, Version, VersionRequirement};
