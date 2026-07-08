//! JNI integration helpers and utilities

use jni::objects::{JClass, JString, JValue};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;

/// Get a UTF-8 string from Java
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `java_string` must be a valid Java String object
pub unsafe fn get_java_string<'a>(env: &'a JNIEnv, java_string: JString) -> crate::Result<String> {
    let c_str = env.get_string(&java_string)?;
    Ok(c_str.into())
}

/// Create a Java string from Rust UTF-8
///
/// # Safety
///
/// - `env` must be a valid JNI environment
pub unsafe fn create_java_string<'a>(env: &'a JNIEnv, rust_string: &str) -> crate::Result<JString> {
    Ok(env.new_string(rust_string)?)
}

/// Call a Java void method
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `obj` must be a valid Java object
/// - `method_id` must correspond to a void method
pub unsafe fn call_java_void_method(
    env: &JNIEnv,
    obj: JValue,
    method_id: &str,
    args: &[JValue],
) -> crate::Result<()> {
    let _ = env.call_method(obj, method_id, "()V", args)?;
    Ok(())
}

/// Call a Java method that returns an int
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `obj` must be a valid Java object
/// - `method_id` must correspond to a method returning int
pub unsafe fn call_java_int_method(
    env: &JNIEnv,
    obj: JValue,
    method_id: &str,
    args: &[JValue],
) -> crate::Result<i32> {
    let result = env.call_method(obj, method_id, "()I", args)?;
    Ok(result.i()?)
}

/// Call a Java method that returns a boolean
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `obj` must be a valid Java object
/// - `method_id` must correspond to a method returning boolean
pub unsafe fn call_java_boolean_method(
    env: &JNIEnv,
    obj: JValue,
    method_id: &str,
    args: &[JValue],
) -> crate::Result<bool> {
    let result = env.call_method(obj, method_id, "()Z", args)?;
    Ok(result.z()?)
}

/// Call a Java static method that returns an object
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `class` must be a valid Java class
pub unsafe fn call_java_static_method<'a>(
    env: &'a JNIEnv,
    class: JClass,
    method_id: &str,
    return_type: &str,
    args: &[JValue],
) -> crate::Result<JValue> {
    Ok(env.call_static_method(class, method_id, return_type, args)?)
}

/// Get a field from a Java object
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `obj` must be a valid Java object
pub unsafe fn get_java_field(
    env: &JNIEnv,
    obj: JValue,
    field_name: &str,
    field_type: &str,
) -> crate::Result<JValue> {
    Ok(env.get_field(obj, field_name, field_type)?)
}

/// Set a field on a Java object
///
/// # Safety
///
/// - `env` must be a valid JNI environment
/// - `obj` must be a valid Java object
pub unsafe fn set_java_field(
    env: &JNIEnv,
    obj: JValue,
    field_name: &str,
    field_type: &str,
    value: JValue,
) -> crate::Result<()> {
    env.set_field(obj, field_name, field_type, value)?;
    Ok(())
}

/// Check for and clear any JNI exceptions
///
/// # Safety
///
/// - `env` must be a valid JNI environment
pub unsafe fn check_and_clear_exception(env: &JNIEnv) -> Option<String> {
    if let Ok(throwable) = env.exception_occurred() {
        if !throwable.is_null() {
            let _ = env.exception_clear();
            return Some("JNI exception occurred".to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_helpers_compile() {
        // This test just verifies that the FFI helpers compile correctly
        // Actual JNI testing would require an Android environment
    }
}
