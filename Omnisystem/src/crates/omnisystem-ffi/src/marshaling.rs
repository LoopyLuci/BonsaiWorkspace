/// Type Marshaling - Converting values between languages

use crate::types::FFIType;
use std::ffi::{c_char, c_void, CStr};

/// Marshal result
pub type MarshalResult<T> = Result<T, MarshalError>;

#[derive(Debug, Clone)]
pub enum MarshalError {
    InvalidType,
    InvalidPointer,
    NullPointer,
    ConversionFailed(String),
    AllocationFailed,
}

impl std::fmt::Display for MarshalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarshalError::InvalidType => write!(f, "Invalid FFI type"),
            MarshalError::InvalidPointer => write!(f, "Invalid pointer"),
            MarshalError::NullPointer => write!(f, "Null pointer"),
            MarshalError::ConversionFailed(msg) => write!(f, "Conversion failed: {}", msg),
            MarshalError::AllocationFailed => write!(f, "Memory allocation failed"),
        }
    }
}

impl std::error::Error for MarshalError {}

/// Marshaler trait for type conversions
pub trait Marshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>>;
    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()>;
}

/// Primitive type marshaling implementations

pub struct BoolMarshaler(pub bool);
impl Marshaler for BoolMarshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>> {
        Ok(vec![if self.0 { 1 } else { 0 }])
    }

    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()> {
        if data.is_empty() {
            return Err(MarshalError::InvalidType);
        }
        self.0 = data[0] != 0;
        Ok(())
    }
}

pub struct IntMarshaler(pub i32);
impl Marshaler for IntMarshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>> {
        Ok(self.0.to_le_bytes().to_vec())
    }

    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()> {
        if data.len() < 4 {
            return Err(MarshalError::InvalidType);
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&data[0..4]);
        self.0 = i32::from_le_bytes(bytes);
        Ok(())
    }
}

pub struct LongMarshaler(pub i64);
impl Marshaler for LongMarshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>> {
        Ok(self.0.to_le_bytes().to_vec())
    }

    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()> {
        if data.len() < 8 {
            return Err(MarshalError::InvalidType);
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[0..8]);
        self.0 = i64::from_le_bytes(bytes);
        Ok(())
    }
}

pub struct FloatMarshaler(pub f32);
impl Marshaler for FloatMarshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>> {
        Ok(self.0.to_le_bytes().to_vec())
    }

    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()> {
        if data.len() < 4 {
            return Err(MarshalError::InvalidType);
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&data[0..4]);
        self.0 = f32::from_le_bytes(bytes);
        Ok(())
    }
}

pub struct DoubleMarshaler(pub f64);
impl Marshaler for DoubleMarshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>> {
        Ok(self.0.to_le_bytes().to_vec())
    }

    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()> {
        if data.len() < 8 {
            return Err(MarshalError::InvalidType);
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[0..8]);
        self.0 = f64::from_le_bytes(bytes);
        Ok(())
    }
}

pub struct PointerMarshaler(pub *mut c_void);
impl Marshaler for PointerMarshaler {
    fn to_ffi(&self) -> MarshalResult<Vec<u8>> {
        let ptr_val = self.0 as usize as u64;
        Ok(ptr_val.to_le_bytes().to_vec())
    }

    fn from_ffi(&mut self, data: &[u8]) -> MarshalResult<()> {
        if data.len() < 8 {
            return Err(MarshalError::InvalidType);
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[0..8]);
        let ptr_val = u64::from_le_bytes(bytes) as usize;
        self.0 = ptr_val as *mut c_void;
        Ok(())
    }
}

/// String marshaling
pub fn marshal_cstring(s: &CStr) -> MarshalResult<Vec<u8>> {
    Ok(s.to_bytes_with_nul().to_vec())
}

pub fn unmarshal_cstring(data: &[u8]) -> MarshalResult<String> {
    CStr::from_bytes_until_nul(data)
        .map_err(|_| MarshalError::InvalidType)
        .and_then(|s| s.to_str().map(|s| s.to_string()).map_err(|_| MarshalError::ConversionFailed("Invalid UTF-8".to_string())))
}

/// Generic type marshaling dispatcher
pub fn marshal_value(value: &FFIType, data: &[u8]) -> MarshalResult<Vec<u8>> {
    match value {
        FFIType::Bool => {
            let mut m = BoolMarshaler(false);
            m.from_ffi(data)?;
            m.to_ffi()
        }
        FFIType::Int32 => {
            let mut m = IntMarshaler(0);
            m.from_ffi(data)?;
            m.to_ffi()
        }
        FFIType::Int64 => {
            let mut m = LongMarshaler(0);
            m.from_ffi(data)?;
            m.to_ffi()
        }
        FFIType::Float32 => {
            let mut m = FloatMarshaler(0.0);
            m.from_ffi(data)?;
            m.to_ffi()
        }
        FFIType::Float64 => {
            let mut m = DoubleMarshaler(0.0);
            m.from_ffi(data)?;
            m.to_ffi()
        }
        FFIType::Pointer => {
            let mut m = PointerMarshaler(std::ptr::null_mut());
            m.from_ffi(data)?;
            m.to_ffi()
        }
        _ => Err(MarshalError::InvalidType),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_marshaling() {
        let mut m = BoolMarshaler(true);
        let bytes = m.to_ffi().unwrap();
        assert!(!bytes.is_empty());

        let mut m2 = BoolMarshaler(false);
        m2.from_ffi(&bytes).unwrap();
        assert_eq!(m2.0, true);
    }

    #[test]
    fn test_int_marshaling() {
        let mut m = IntMarshaler(42);
        let bytes = m.to_ffi().unwrap();

        let mut m2 = IntMarshaler(0);
        m2.from_ffi(&bytes).unwrap();
        assert_eq!(m2.0, 42);
    }

    #[test]
    fn test_float_marshaling() {
        let mut m = FloatMarshaler(3.14);
        let bytes = m.to_ffi().unwrap();

        let mut m2 = FloatMarshaler(0.0);
        m2.from_ffi(&bytes).unwrap();
        assert!((m2.0 - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_pointer_marshaling() {
        let ptr = 0x12345678 as *mut c_void;
        let mut m = PointerMarshaler(ptr);
        let bytes = m.to_ffi().unwrap();

        let mut m2 = PointerMarshaler(std::ptr::null_mut());
        m2.from_ffi(&bytes).unwrap();
        assert_eq!(m2.0 as usize, 0x12345678);
    }
}
