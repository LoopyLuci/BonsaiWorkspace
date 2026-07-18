//! Omnisystem FFI CLI - exercises ABI detection, type info, marshaling, and versioning

use omnisystem_ffi::{
    detect_calling_convention, marshaling::IntMarshaler, type_name, type_size, FFIType, Marshaler,
    Version,
};

fn main() {
    let convention = detect_calling_convention();
    println!("detected calling convention: {:?}", convention);

    println!(
        "i32 size={} name={}",
        type_size(&FFIType::Int32),
        type_name(&FFIType::Int32)
    );

    let m = IntMarshaler(42);
    let bytes = m.to_ffi().unwrap();
    println!("marshaled 42 -> {} bytes: {:?}", bytes.len(), bytes);

    let v1 = Version::new(1, 2, 0);
    let v2 = Version::new(1, 0, 0);
    println!("{} compatible with {}: {}", v1.to_string(), v2.to_string(), v1.is_compatible_with(v2));
}
