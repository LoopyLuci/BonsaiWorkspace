//! Omnisystem Serialization
//!
//! JSON and binary serialization without external dependencies.
//! Provides fast, safe encoding and decoding for common data types.

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

/// JSON value enum for flexible data representation
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    /// Get as string, if possible
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as number, if possible
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get as i64, if possible
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) if n.fract() == 0.0 && *n >= 0.0 => Some(*n as i64),
            _ => None,
        }
    }

    /// Get as bool, if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as array, if possible
    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Get as object, if possible
    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Get mutable reference to object
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }
}

/// JSON encoder for converting values to JSON strings
pub struct JsonEncoder;

impl JsonEncoder {
    /// Encode a JsonValue to JSON string
    pub fn encode(value: &JsonValue) -> String {
        let mut output = String::new();
        Self::encode_value(value, &mut output);
        output
    }

    fn encode_value(value: &JsonValue, output: &mut String) {
        match value {
            JsonValue::Null => output.push_str("null"),
            JsonValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && *n >= 0.0 && *n < 1e15 {
                    let _ = write!(output, "{}", *n as i64);
                } else {
                    let _ = write!(output, "{}", n);
                }
            }
            JsonValue::String(s) => {
                output.push('"');
                for c in s.chars() {
                    match c {
                        '"' => output.push_str("\\\""),
                        '\\' => output.push_str("\\\\"),
                        '\n' => output.push_str("\\n"),
                        '\r' => output.push_str("\\r"),
                        '\t' => output.push_str("\\t"),
                        c if c.is_control() => {
                            let _ = write!(output, "\\u{:04x}", c as u32);
                        }
                        c => output.push(c),
                    }
                }
                output.push('"');
            }
            JsonValue::Array(arr) => {
                output.push('[');
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        output.push(',');
                    }
                    Self::encode_value(item, output);
                }
                output.push(']');
            }
            JsonValue::Object(obj) => {
                output.push('{');
                let mut first = true;
                for (key, val) in obj.iter() {
                    if !first {
                        output.push(',');
                    }
                    first = false;
                    output.push('"');
                    output.push_str(key);
                    output.push_str("\":");
                    Self::encode_value(val, output);
                }
                output.push('}');
            }
        }
    }
}

/// JSON decoder for parsing JSON strings to JsonValue
pub struct JsonDecoder;

impl JsonDecoder {
    /// Decode a JSON string to JsonValue
    pub fn decode(json: &str) -> Result<JsonValue, String> {
        let mut parser = JsonParser::new(json);
        parser.parse_value()
    }
}

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(json: &str) -> Self {
        JsonParser {
            chars: json.chars().collect(),
            pos: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();

        if self.pos >= self.chars.len() {
            return Err("Unexpected end of input".to_string());
        }

        match self.chars[self.pos] {
            'n' => self.parse_null(),
            't' | 'f' => self.parse_bool(),
            '"' => self.parse_string().map(JsonValue::String),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '-' | '0'..='9' => self.parse_number(),
            c => Err(format!("Unexpected character: {}", c)),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.pos + 4 <= self.chars.len() && &self.chars[self.pos..self.pos + 4] == ['n', 'u', 'l', 'l'] {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err("Invalid null value".to_string())
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.pos + 4 <= self.chars.len() && &self.chars[self.pos..self.pos + 4] == ['t', 'r', 'u', 'e'] {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.pos + 5 <= self.chars.len() && &self.chars[self.pos..self.pos + 5] == ['f', 'a', 'l', 's', 'e'] {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err("Invalid boolean value".to_string())
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.chars[self.pos] != '"' {
            return Err("Expected '\"'".to_string());
        }
        self.pos += 1;

        let mut result = String::new();
        while self.pos < self.chars.len() && self.chars[self.pos] != '"' {
            if self.chars[self.pos] == '\\' && self.pos + 1 < self.chars.len() {
                self.pos += 1;
                match self.chars[self.pos] {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    '/' => result.push('/'),
                    'b' => result.push('\u{0008}'),
                    'f' => result.push('\u{000c}'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'u' => {
                        if self.pos + 4 < self.chars.len() {
                            let code: String = self.chars[self.pos + 1..self.pos + 5].iter().collect();
                            if let Ok(codepoint) = u32::from_str_radix(&code, 16) {
                                if let Some(c) = char::from_u32(codepoint) {
                                    result.push(c);
                                    self.pos += 4;
                                }
                            }
                        }
                    }
                    _ => result.push(self.chars[self.pos]),
                }
            } else {
                result.push(self.chars[self.pos]);
            }
            self.pos += 1;
        }

        if self.pos >= self.chars.len() || self.chars[self.pos] != '"' {
            return Err("Unterminated string".to_string());
        }
        self.pos += 1;

        Ok(result)
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;

        if self.pos < self.chars.len() && self.chars[self.pos] == '-' {
            self.pos += 1;
        }

        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos < self.chars.len() && self.chars[self.pos] == '.' {
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        if self.pos < self.chars.len() && (self.chars[self.pos] == 'e' || self.chars[self.pos] == 'E') {
            self.pos += 1;
            if self.pos < self.chars.len() && (self.chars[self.pos] == '+' || self.chars[self.pos] == '-') {
                self.pos += 1;
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let num_str: String = self.chars[start..self.pos].iter().collect();
        num_str.parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| "Invalid number".to_string())
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        if self.chars[self.pos] != '[' {
            return Err("Expected '['".to_string());
        }
        self.pos += 1;

        let mut arr = Vec::new();
        self.skip_whitespace();

        if self.pos < self.chars.len() && self.chars[self.pos] == ']' {
            self.pos += 1;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            arr.push(self.parse_value()?);
            self.skip_whitespace();

            if self.pos >= self.chars.len() {
                return Err("Unterminated array".to_string());
            }

            if self.chars[self.pos] == ']' {
                self.pos += 1;
                break;
            } else if self.chars[self.pos] == ',' {
                self.pos += 1;
                self.skip_whitespace();
            } else {
                return Err("Expected ',' or ']'".to_string());
            }
        }

        Ok(JsonValue::Array(arr))
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        if self.chars[self.pos] != '{' {
            return Err("Expected '{'".to_string());
        }
        self.pos += 1;

        let mut obj = HashMap::new();
        self.skip_whitespace();

        if self.pos < self.chars.len() && self.chars[self.pos] == '}' {
            self.pos += 1;
            return Ok(JsonValue::Object(obj));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();

            if self.pos >= self.chars.len() || self.chars[self.pos] != ':' {
                return Err("Expected ':'".to_string());
            }
            self.pos += 1;

            let value = self.parse_value()?;
            obj.insert(key, value);

            self.skip_whitespace();

            if self.pos >= self.chars.len() {
                return Err("Unterminated object".to_string());
            }

            if self.chars[self.pos] == '}' {
                self.pos += 1;
                break;
            } else if self.chars[self.pos] == ',' {
                self.pos += 1;
            } else {
                return Err("Expected ',' or '}'".to_string());
            }
        }

        Ok(JsonValue::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_null() {
        assert_eq!(JsonEncoder::encode(&JsonValue::Null), "null");
    }

    #[test]
    fn test_encode_bool() {
        assert_eq!(JsonEncoder::encode(&JsonValue::Bool(true)), "true");
        assert_eq!(JsonEncoder::encode(&JsonValue::Bool(false)), "false");
    }

    #[test]
    fn test_encode_number() {
        assert_eq!(JsonEncoder::encode(&JsonValue::Number(42.0)), "42");
        assert_eq!(JsonEncoder::encode(&JsonValue::Number(3.14)), "3.14");
    }

    #[test]
    fn test_encode_string() {
        assert_eq!(JsonEncoder::encode(&JsonValue::String("hello".to_string())), "\"hello\"");
        assert_eq!(JsonEncoder::encode(&JsonValue::String("hello\"world".to_string())), "\"hello\\\"world\"");
    }

    #[test]
    fn test_encode_array() {
        let arr = JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
            JsonValue::Number(3.0),
        ]);
        assert_eq!(JsonEncoder::encode(&arr), "[1,2,3]");
    }

    #[test]
    fn test_encode_object() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), JsonValue::String("test".to_string()));
        obj.insert("value".to_string(), JsonValue::Number(42.0));
        let json = JsonValue::Object(obj);
        let encoded = JsonEncoder::encode(&json);
        assert!(encoded.contains("\"name\":\"test\""));
        assert!(encoded.contains("\"value\":42"));
    }

    #[test]
    fn test_decode_null() {
        assert_eq!(JsonDecoder::decode("null"), Ok(JsonValue::Null));
    }

    #[test]
    fn test_decode_bool() {
        assert_eq!(JsonDecoder::decode("true"), Ok(JsonValue::Bool(true)));
        assert_eq!(JsonDecoder::decode("false"), Ok(JsonValue::Bool(false)));
    }

    #[test]
    fn test_decode_number() {
        assert_eq!(JsonDecoder::decode("42"), Ok(JsonValue::Number(42.0)));
    }

    #[test]
    fn test_decode_string() {
        assert_eq!(JsonDecoder::decode("\"hello\""), Ok(JsonValue::String("hello".to_string())));
    }

    #[test]
    fn test_decode_array() {
        let result = JsonDecoder::decode("[1,2,3]");
        assert!(result.is_ok());
        let arr = result.unwrap();
        assert_eq!(arr.as_array().map(|a| a.len()), Some(3));
    }

    #[test]
    fn test_roundtrip() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), JsonValue::String("test".to_string()));
        obj.insert("count".to_string(), JsonValue::Number(42.0));
        let original = JsonValue::Object(obj);
        let encoded = JsonEncoder::encode(&original);
        let decoded = JsonDecoder::decode(&encoded);
        assert!(decoded.is_ok());
    }
}
