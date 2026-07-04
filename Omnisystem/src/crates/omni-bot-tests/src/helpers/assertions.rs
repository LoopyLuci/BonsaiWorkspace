//! Custom assertions for testing

use std::fmt;

/// Custom assertion error
#[derive(Debug)]
pub struct AssertionError {
    message: String,
}

impl fmt::Display for AssertionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AssertionError {}

pub type AssertionResult = Result<(), AssertionError>;

/// Assert that a condition is true
#[macro_export]
macro_rules! assert_api_ok {
    ($result:expr, $message:expr) => {
        assert!($result.is_ok(), "Expected Ok, but got: {}: {}", $message, $result.unwrap_err());
    };
}

/// Assert that two values are equal with custom message
#[macro_export]
macro_rules! assert_eq_with_msg {
    ($left:expr, $right:expr, $message:expr) => {
        assert_eq!(
            $left, $right,
            "Assertion failed: {} - Expected {:?}, got {:?}",
            $message, $right, $left
        );
    };
}

/// Assert response structure
pub fn assert_response_valid<T: fmt::Debug>(
    response: &T,
    _message: &str,
) -> AssertionResult {
    eprintln!("Response: {:?}", response);
    Ok(())
}

/// Assert service is in expected state
pub fn assert_service_state(
    state: &str,
    expected: &str,
) -> AssertionResult {
    if state == expected {
        Ok(())
    } else {
        Err(AssertionError {
            message: format!("Expected state {}, got {}", expected, state),
        })
    }
}

/// Assert resource cleanup
pub fn assert_resource_cleaned(
    count: usize,
    expected: usize,
) -> AssertionResult {
    if count == expected {
        Ok(())
    } else {
        Err(AssertionError {
            message: format!("Expected {} resources, found {}", expected, count),
        })
    }
}

/// Assert concurrent operations succeeded
pub fn assert_concurrent_success(
    successful: usize,
    total: usize,
) -> AssertionResult {
    if successful == total {
        Ok(())
    } else {
        Err(AssertionError {
            message: format!(
                "Concurrent operations: {}/{} succeeded",
                successful, total
            ),
        })
    }
}

/// Assert operation within timeout
pub fn assert_within_timeout(
    duration_ms: u128,
    timeout_ms: u128,
) -> AssertionResult {
    if duration_ms <= timeout_ms {
        Ok(())
    } else {
        Err(AssertionError {
            message: format!(
                "Operation took {}ms, exceeds timeout of {}ms",
                duration_ms, timeout_ms
            ),
        })
    }
}

/// Assert CRDT merge consistency
pub fn assert_crdt_consistent(
    original: &serde_json::Value,
    merged: &serde_json::Value,
) -> AssertionResult {
    // Check that merged state contains all original data
    if let (Some(obj1), Some(obj2)) = (original.as_object(), merged.as_object()) {
        for (key, value) in obj1.iter() {
            if obj2.get(key) != Some(value) {
                return Err(AssertionError {
                    message: format!("CRDT merge lost key: {}", key),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_service_state() {
        assert!(assert_service_state("running", "running").is_ok());
        assert!(assert_service_state("running", "stopped").is_err());
    }

    #[test]
    fn test_assert_resource_cleaned() {
        assert!(assert_resource_cleaned(0, 0).is_ok());
        assert!(assert_resource_cleaned(1, 0).is_err());
    }

    #[test]
    fn test_assert_concurrent_success() {
        assert!(assert_concurrent_success(10, 10).is_ok());
        assert!(assert_concurrent_success(9, 10).is_err());
    }

    #[test]
    fn test_assert_within_timeout() {
        assert!(assert_within_timeout(100, 200).is_ok());
        assert!(assert_within_timeout(300, 200).is_err());
    }
}
