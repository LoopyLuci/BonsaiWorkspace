// TITAN STANDARD LIBRARY - Built-in functions and utilities

pub fn get_stdlib_functions() -> Vec<(&'static str, &'static str)> {
    vec![
        // I/O Functions
        ("println", "fn println(String) -> void"),
        ("print", "fn print(String) -> void"),
        ("input", "fn input(String) -> String"),

        // Math Functions
        ("abs", "fn abs(i64) -> i64"),
        ("sqrt", "fn sqrt(f64) -> f64"),
        ("pow", "fn pow(f64, f64) -> f64"),
        ("sin", "fn sin(f64) -> f64"),
        ("cos", "fn cos(f64) -> f64"),
        ("tan", "fn tan(f64) -> f64"),
        ("floor", "fn floor(f64) -> f64"),
        ("ceil", "fn ceil(f64) -> f64"),
        ("round", "fn round(f64) -> f64"),

        // String Functions
        ("len", "fn len(String) -> i64"),
        ("substring", "fn substring(String, i64, i64) -> String"),
        ("contains", "fn contains(String, String) -> bool"),
        ("starts_with", "fn starts_with(String, String) -> bool"),
        ("ends_with", "fn ends_with(String, String) -> bool"),
        ("to_upper", "fn to_upper(String) -> String"),
        ("to_lower", "fn to_lower(String) -> String"),

        // Array Functions
        ("array_len", "fn array_len(Array) -> i64"),
        ("array_push", "fn array_push(Array, i64) -> void"),
        ("array_pop", "fn array_pop(Array) -> i64"),
        ("array_reverse", "fn array_reverse(Array) -> Array"),
        ("array_sort", "fn array_sort(Array) -> Array"),

        // Type Conversion
        ("to_string", "fn to_string(i64) -> String"),
        ("to_int", "fn to_int(String) -> i64"),
        ("to_float", "fn to_float(String) -> f64"),

        // System Functions
        ("time", "fn time() -> i64"),
        ("sleep", "fn sleep(i64) -> void"),
        ("exit", "fn exit(i64) -> void"),
    ]
}

// Built-in implementations would go here in a full implementation
// For now, these are just declarations that a real TITAN runtime would provide
