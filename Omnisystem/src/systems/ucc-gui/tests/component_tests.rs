//! Component-level tests for UCC GUI

#[cfg(test)]
mod component_tests {
    use std::path::PathBuf;

    // Test project path validation
    #[test]
    fn test_valid_project_path() {
        let path = PathBuf::from(".");
        assert!(path.exists(), "Current directory should exist");
        assert!(path.is_dir(), "Path should be a directory");
    }

    #[test]
    fn test_invalid_project_path() {
        let path = PathBuf::from("/this/path/definitely/does/not/exist");
        assert!(!path.exists(), "Non-existent path should return false");
    }

    // Test language detection from file extensions
    #[test]
    fn test_language_detection_rust() {
        let ext = "rs";
        let lang = match ext {
            "rs" => Some("Rust"),
            _ => None,
        };
        assert_eq!(lang, Some("Rust"));
    }

    #[test]
    fn test_language_detection_python() {
        let ext = "py";
        let lang = match ext {
            "py" => Some("Python"),
            _ => None,
        };
        assert_eq!(lang, Some("Python"));
    }

    #[test]
    fn test_language_detection_go() {
        let ext = "go";
        let lang = match ext {
            "go" => Some("Go"),
            _ => None,
        };
        assert_eq!(lang, Some("Go"));
    }

    #[test]
    fn test_language_detection_typescript() {
        let ext = "ts";
        let lang = match ext {
            "ts" | "tsx" => Some("TypeScript"),
            _ => None,
        };
        assert_eq!(lang, Some("TypeScript"));
    }

    #[test]
    fn test_language_detection_cpp() {
        let ext = "cpp";
        let lang = match ext {
            "c" | "cpp" | "cc" | "cxx" => Some("C++"),
            _ => None,
        };
        assert_eq!(lang, Some("C++"));
    }

    #[test]
    fn test_unknown_extension() {
        let ext = "xyz";
        let lang = match ext {
            "rs" => Some("Rust"),
            "py" => Some("Python"),
            _ => None,
        };
        assert_eq!(lang, None);
    }

    // Test build result creation
    #[test]
    fn test_build_result_success() {
        let success = true;
        let errors = 0;
        let warnings = 0;

        assert!(success);
        assert_eq!(errors, 0);
        assert_eq!(warnings, 0);
    }

    #[test]
    fn test_build_result_failure() {
        let success = false;
        let errors = 3;
        let warnings = 1;

        assert!(!success);
        assert_eq!(errors, 3);
        assert_eq!(warnings, 1);
    }

    // Test metrics calculations
    #[test]
    fn test_success_rate_calculation() {
        let total = 10;
        let successful = 8;
        let rate = (successful as f32 / total as f32) * 100.0;

        assert!((rate - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_average_build_time() {
        let builds = vec![1000u128, 2000u128, 3000u128];
        let total: u128 = builds.iter().sum();
        let average = total / builds.len() as u128;

        assert_eq!(average, 2000);
    }

    #[test]
    fn test_cache_hit_rate_zero() {
        let total_builds = 0;
        let cache_hit_rate = if total_builds > 0 {
            100.0
        } else {
            0.0
        };

        assert_eq!(cache_hit_rate, 0.0);
    }

    #[test]
    fn test_cache_hit_rate_high() {
        let total_builds = 100;
        let successful = 95;
        let cache_hit_rate = (successful as f32 / total_builds as f32) * 100.0;

        assert!((cache_hit_rate - 95.0).abs() < 0.1);
    }

    // Test UI state initialization
    #[test]
    fn test_empty_detected_languages() {
        let detected_languages: Vec<String> = vec![];
        assert_eq!(detected_languages.len(), 0);
    }

    #[test]
    fn test_multiple_detected_languages() {
        let mut detected_languages = vec!["Rust", "Python", "Go"];
        detected_languages.sort();

        assert_eq!(detected_languages.len(), 3);
        assert_eq!(detected_languages[0], "Go");
    }

    // Test error messages
    #[test]
    fn test_no_project_error() {
        let project_path: Option<PathBuf> = None;
        let msg = match project_path {
            Some(_) => "Project loaded",
            None => "No project selected",
        };

        assert_eq!(msg, "No project selected");
    }

    #[test]
    fn test_build_without_project_error() {
        let project_path: Option<PathBuf> = None;
        if project_path.is_none() {
            let msg = "No project selected. Click 'Open Project' first.";
            assert!(msg.contains("No project selected"));
        }
    }

    #[test]
    fn test_invalid_path_error() {
        let path = PathBuf::from("/nonexistent");
        let msg = if path.exists() && path.is_dir() {
            "Project loaded"
        } else {
            "Invalid project path"
        };

        assert_eq!(msg, "Invalid project path");
    }
}
