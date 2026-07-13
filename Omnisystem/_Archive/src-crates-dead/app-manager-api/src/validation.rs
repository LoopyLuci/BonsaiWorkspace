//! Request validation utilities

use regex::Regex;

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validation result
pub type ValidationResult<T> = Result<T, Vec<ValidationError>>;

/// Email validator
pub struct EmailValidator;

impl EmailValidator {
    /// Validate email format
    pub fn validate(email: &str) -> ValidationResult<()> {
        if email.is_empty() {
            return Err(vec![ValidationError::new("email", "Email is required")]);
        }

        if email.len() > 254 {
            return Err(vec![ValidationError::new(
                "email",
                "Email is too long (max 254 characters)",
            )]);
        }

        // Basic email validation
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .unwrap();

        if !email_regex.is_match(email) {
            return Err(vec![ValidationError::new("email", "Invalid email format")]);
        }

        Ok(())
    }
}

/// Password validator
pub struct PasswordValidator;

impl PasswordValidator {
    /// Validate password strength
    pub fn validate(password: &str) -> ValidationResult<()> {
        let mut errors = Vec::new();

        if password.len() < 8 {
            errors.push(ValidationError::new(
                "password",
                "Password must be at least 8 characters",
            ));
        }

        if password.len() > 128 {
            errors.push(ValidationError::new(
                "password",
                "Password must be less than 128 characters",
            ));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        if !has_uppercase {
            errors.push(ValidationError::new(
                "password",
                "Password must contain at least one uppercase letter",
            ));
        }

        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        if !has_lowercase {
            errors.push(ValidationError::new(
                "password",
                "Password must contain at least one lowercase letter",
            ));
        }

        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        if !has_digit {
            errors.push(ValidationError::new(
                "password",
                "Password must contain at least one digit",
            ));
        }

        let has_special = password.chars().any(|c| "!@#$%^&*".contains(c));
        if !has_special {
            errors.push(ValidationError::new(
                "password",
                "Password must contain at least one special character (!@#$%^&*)",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// UUID validator
pub struct UuidValidator;

impl UuidValidator {
    /// Validate UUID format
    pub fn validate(uuid_str: &str) -> ValidationResult<()> {
        if uuid_str.is_empty() {
            return Err(vec![ValidationError::new("id", "ID is required")]);
        }

        match uuid::Uuid::parse_str(uuid_str) {
            Ok(_) => Ok(()),
            Err(_) => Err(vec![ValidationError::new(
                "id",
                "Invalid UUID format",
            )]),
        }
    }
}

/// String validator
pub struct StringValidator;

impl StringValidator {
    /// Validate non-empty string
    pub fn non_empty(value: &str, field_name: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(vec![ValidationError::new(
                field_name,
                &format!("{} cannot be empty", field_name),
            )]);
        }
        Ok(())
    }

    /// Validate string length
    pub fn length(value: &str, field_name: &str, min: usize, max: usize) -> ValidationResult<()> {
        let len = value.len();

        if len < min {
            return Err(vec![ValidationError::new(
                field_name,
                &format!("{} must be at least {} characters", field_name, min),
            )]);
        }

        if len > max {
            return Err(vec![ValidationError::new(
                field_name,
                &format!("{} must be less than {} characters", field_name, max),
            )]);
        }

        Ok(())
    }

    /// Validate matches pattern
    pub fn matches_pattern(
        value: &str,
        field_name: &str,
        pattern: &str,
    ) -> ValidationResult<()> {
        let regex = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => {
                return Err(vec![ValidationError::new(
                    field_name,
                    "Invalid validation pattern",
                )])
            }
        };

        if !regex.is_match(value) {
            return Err(vec![ValidationError::new(
                field_name,
                "Value does not match required pattern",
            )]);
        }

        Ok(())
    }
}

/// Version validator
pub struct VersionValidator;

impl VersionValidator {
    /// Validate semantic version format
    pub fn validate(version: &str) -> ValidationResult<()> {
        if version.is_empty() {
            return Err(vec![ValidationError::new(
                "version",
                "Version is required",
            )]);
        }

        // Match semantic version pattern: X.Y.Z
        let semver_regex = Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$")
            .unwrap();

        if !semver_regex.is_match(version) {
            return Err(vec![ValidationError::new(
                "version",
                "Invalid semantic version format (expected X.Y.Z)",
            )]);
        }

        Ok(())
    }
}

/// Number validator
pub struct NumberValidator;

impl NumberValidator {
    /// Validate number in range
    pub fn in_range(
        value: i32,
        field_name: &str,
        min: i32,
        max: i32,
    ) -> ValidationResult<()> {
        if value < min || value > max {
            return Err(vec![ValidationError::new(
                field_name,
                &format!("{} must be between {} and {}", field_name, min, max),
            )]);
        }
        Ok(())
    }

    /// Validate positive number
    pub fn positive(value: i32, field_name: &str) -> ValidationResult<()> {
        if value <= 0 {
            return Err(vec![ValidationError::new(
                field_name,
                &format!("{} must be positive", field_name),
            )]);
        }
        Ok(())
    }

    /// Validate non-negative number
    pub fn non_negative(value: i32, field_name: &str) -> ValidationResult<()> {
        if value < 0 {
            return Err(vec![ValidationError::new(
                field_name,
                &format!("{} must be non-negative", field_name),
            )]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation_valid() {
        assert!(EmailValidator::validate("user@example.com").is_ok());
        assert!(EmailValidator::validate("test.user+tag@domain.co.uk").is_ok());
    }

    #[test]
    fn test_email_validation_invalid() {
        assert!(EmailValidator::validate("invalid-email").is_err());
        assert!(EmailValidator::validate("").is_err());
        assert!(EmailValidator::validate("user@").is_err());
    }

    #[test]
    fn test_password_validation_strong() {
        let strong = "SecurePass123!@#";
        assert!(PasswordValidator::validate(strong).is_ok());
    }

    #[test]
    fn test_password_validation_weak() {
        assert!(PasswordValidator::validate("weak").is_err()); // Too short
        assert!(PasswordValidator::validate("nouppercasebuthasdigit123!").is_err()); // No uppercase
        assert!(PasswordValidator::validate("NoDigits!@#").is_err()); // No digit
    }

    #[test]
    fn test_uuid_validation() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(UuidValidator::validate(valid_uuid).is_ok());

        assert!(UuidValidator::validate("not-a-uuid").is_err());
        assert!(UuidValidator::validate("").is_err());
    }

    #[test]
    fn test_string_non_empty() {
        assert!(StringValidator::non_empty("valid", "field").is_ok());
        assert!(StringValidator::non_empty("", "field").is_err());
        assert!(StringValidator::non_empty("   ", "field").is_err());
    }

    #[test]
    fn test_string_length() {
        assert!(StringValidator::length("hello", "field", 1, 10).is_ok());
        assert!(StringValidator::length("hi", "field", 1, 1).is_err()); // Too long
        assert!(StringValidator::length("", "field", 1, 10).is_err()); // Too short
    }

    #[test]
    fn test_version_validation() {
        assert!(VersionValidator::validate("1.0.0").is_ok());
        assert!(VersionValidator::validate("2.3.4").is_ok());
        assert!(VersionValidator::validate("1.0.0-alpha").is_ok());

        assert!(VersionValidator::validate("1.0").is_err());
        assert!(VersionValidator::validate("invalid").is_err());
    }

    #[test]
    fn test_number_in_range() {
        assert!(NumberValidator::in_range(5, "field", 1, 10).is_ok());
        assert!(NumberValidator::in_range(0, "field", 1, 10).is_err());
        assert!(NumberValidator::in_range(11, "field", 1, 10).is_err());
    }

    #[test]
    fn test_number_positive() {
        assert!(NumberValidator::positive(5, "field").is_ok());
        assert!(NumberValidator::positive(0, "field").is_err());
        assert!(NumberValidator::positive(-1, "field").is_err());
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new("email", "Invalid format");
        assert_eq!(err.to_string(), "email: Invalid format");
    }
}
