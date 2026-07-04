/// FFI Type System - Cross-language type definitions

use std::ffi::{c_char, c_void};

/// Base FFI type representation
#[repr(C)]
pub enum FFIType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Pointer,
    CString,
    Struct(FFIStructType),
    Array(FFIArrayType),
    Function(FFIFunctionType),
}

#[repr(C)]
pub struct FFIStructType {
    pub name: *const c_char,
    pub fields: *const FFIField,
    pub field_count: usize,
    pub size: usize,
    pub alignment: usize,
}

#[repr(C)]
pub struct FFIField {
    pub name: *const c_char,
    pub field_type: *const FFIType,
    pub offset: usize,
    pub size: usize,
}

#[repr(C)]
pub struct FFIArrayType {
    pub element_type: *const FFIType,
    pub length: usize,
    pub element_size: usize,
}

#[repr(C)]
pub struct FFIFunctionType {
    pub return_type: *const FFIType,
    pub param_types: *const *const FFIType,
    pub param_count: usize,
    pub is_variadic: bool,
}

/// Type information holder
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub alignment: usize,
    pub is_primitive: bool,
}

impl TypeInfo {
    pub fn new(name: &str, size: usize, alignment: usize) -> Self {
        TypeInfo {
            name: name.to_string(),
            size,
            alignment,
            is_primitive: true,
        }
    }

    pub fn new_struct(name: &str, size: usize, alignment: usize) -> Self {
        TypeInfo {
            name: name.to_string(),
            size,
            alignment,
            is_primitive: false,
        }
    }
}

/// Primitive type helpers
pub fn type_size(ffi_type: &FFIType) -> usize {
    match ffi_type {
        FFIType::Void => 0,
        FFIType::Bool => 1,
        FFIType::Int8 | FFIType::UInt8 => 1,
        FFIType::Int16 | FFIType::UInt16 => 2,
        FFIType::Int32 | FFIType::UInt32 => 4,
        FFIType::Int64 | FFIType::UInt64 => 8,
        FFIType::Float32 => 4,
        FFIType::Float64 => 8,
        FFIType::Pointer | FFIType::CString => 8, // 64-bit pointers
        _ => 0,
    }
}

pub fn type_alignment(ffi_type: &FFIType) -> usize {
    type_size(ffi_type)
}

pub fn type_name(ffi_type: &FFIType) -> &'static str {
    match ffi_type {
        FFIType::Void => "void",
        FFIType::Bool => "bool",
        FFIType::Int8 => "i8",
        FFIType::Int16 => "i16",
        FFIType::Int32 => "i32",
        FFIType::Int64 => "i64",
        FFIType::UInt8 => "u8",
        FFIType::UInt16 => "u16",
        FFIType::UInt32 => "u32",
        FFIType::UInt64 => "u64",
        FFIType::Float32 => "f32",
        FFIType::Float64 => "f64",
        FFIType::Pointer => "*void",
        FFIType::CString => "*const c_char",
        FFIType::Struct(_) => "struct",
        FFIType::Array(_) => "array",
        FFIType::Function(_) => "function",
    }
}

/// Type validation
pub fn is_compatible_type(source: &FFIType, target: &FFIType) -> bool {
    match (source, target) {
        // Exact matches
        (FFIType::Void, FFIType::Void) => true,
        (FFIType::Bool, FFIType::Bool) => true,
        (FFIType::Int8, FFIType::Int8) => true,
        (FFIType::Int16, FFIType::Int16) => true,
        (FFIType::Int32, FFIType::Int32) => true,
        (FFIType::Int64, FFIType::Int64) => true,
        (FFIType::UInt8, FFIType::UInt8) => true,
        (FFIType::UInt16, FFIType::UInt16) => true,
        (FFIType::UInt32, FFIType::UInt32) => true,
        (FFIType::UInt64, FFIType::UInt64) => true,
        (FFIType::Float32, FFIType::Float32) => true,
        (FFIType::Float64, FFIType::Float64) => true,
        (FFIType::Pointer, FFIType::Pointer) => true,
        (FFIType::CString, FFIType::CString) => true,

        // Compatible conversions
        (FFIType::Int8, FFIType::Int16) => true,
        (FFIType::Int8, FFIType::Int32) => true,
        (FFIType::Int16, FFIType::Int32) => true,

        // Default: incompatible
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_size() {
        assert_eq!(type_size(&FFIType::Void), 0);
        assert_eq!(type_size(&FFIType::Bool), 1);
        assert_eq!(type_size(&FFIType::Int32), 4);
        assert_eq!(type_size(&FFIType::Int64), 8);
        assert_eq!(type_size(&FFIType::Pointer), 8);
    }

    #[test]
    fn test_type_alignment() {
        assert_eq!(type_alignment(&FFIType::Int32), 4);
        assert_eq!(type_alignment(&FFIType::Int64), 8);
    }

    #[test]
    fn test_type_name() {
        assert_eq!(type_name(&FFIType::Void), "void");
        assert_eq!(type_name(&FFIType::Int32), "i32");
        assert_eq!(type_name(&FFIType::Pointer), "*void");
    }

    #[test]
    fn test_type_compatibility() {
        assert!(is_compatible_type(&FFIType::Int32, &FFIType::Int32));
        assert!(is_compatible_type(&FFIType::Int8, &FFIType::Int32));
        assert!(!is_compatible_type(&FFIType::Int32, &FFIType::Float32));
    }

    #[test]
    fn test_type_info() {
        let info = TypeInfo::new("int", 4, 4);
        assert_eq!(info.name, "int");
        assert_eq!(info.size, 4);
        assert!(info.is_primitive);
    }
}
