// TITAN LEXER - Tokenization Module
// Converts raw source code into a stream of tokens

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),

    // Keywords
    Let,
    Mut,
    Fn,
    Return,
    If,
    Else,
    For,
    While,
    Loop,
    Break,
    Continue,
    Match,
    Async,
    Await,
    Pub,
    Struct,
    Enum,
    Trait,
    Impl,
    Use,
    Mod,
    Type,
    Const,
    Static,
    Unsafe,
    As,
    True,
    False,
    None,
    Some,
    In,

    // Identifiers
    Identifier(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    LeftShift,
    RightShift,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    Arrow,
    FatArrow,
    DotDot,
    DotDotEqual,

    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Colon,
    Comma,
    Dot,
    Question,

    // Special
    Eof,
    Newline,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}

struct Lexer {
    source: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        while self.position < self.source.len() {
            self.skip_whitespace_and_comments();

            if self.position >= self.source.len() {
                break;
            }

            let ch = self.current_char();
            let token = self.next_token()?;

            if let TokenType::Eof = token.token_type {
                break;
            }

            self.tokens.push(token);
        }

        self.tokens.push(Token {
            token_type: TokenType::Eof,
            line: self.line,
            column: self.column,
        });

        Ok(self.tokens.clone())
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.position < self.source.len() {
            let ch = self.current_char();

            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            if ch == '\n' {
                self.line += 1;
                self.column = 1;
                self.advance();
                continue;
            }

            // Skip single-line comments
            if ch == '/' && self.peek() == '/' {
                self.advance();
                self.advance();
                while self.position < self.source.len() && self.current_char() != '\n' {
                    self.advance();
                }
                continue;
            }

            // Skip multi-line comments
            if ch == '/' && self.peek() == '*' {
                self.advance();
                self.advance();
                while self.position < self.source.len() {
                    if self.current_char() == '*' && self.peek() == '/' {
                        self.advance();
                        self.advance();
                        break;
                    }
                    if self.current_char() == '\n' {
                        self.line += 1;
                        self.column = 1;
                    }
                    self.advance();
                }
                continue;
            }

            break;
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        let line = self.line;
        let column = self.column;

        let ch = self.current_char();

        // Numbers
        if ch.is_ascii_digit() {
            return Ok(Token {
                token_type: self.read_number()?,
                line,
                column,
            });
        }

        // Strings
        if ch == '"' {
            return Ok(Token {
                token_type: TokenType::String(self.read_string()?),
                line,
                column,
            });
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            let ident = self.read_identifier();
            let token_type = match ident.as_str() {
                "let" => TokenType::Let,
                "mut" => TokenType::Mut,
                "fn" => TokenType::Fn,
                "return" => TokenType::Return,
                "if" => TokenType::If,
                "else" => TokenType::Else,
                "for" => TokenType::For,
                "while" => TokenType::While,
                "loop" => TokenType::Loop,
                "break" => TokenType::Break,
                "continue" => TokenType::Continue,
                "match" => TokenType::Match,
                "async" => TokenType::Async,
                "await" => TokenType::Await,
                "pub" => TokenType::Pub,
                "struct" => TokenType::Struct,
                "enum" => TokenType::Enum,
                "trait" => TokenType::Trait,
                "impl" => TokenType::Impl,
                "use" => TokenType::Use,
                "mod" => TokenType::Mod,
                "type" => TokenType::Type,
                "const" => TokenType::Const,
                "static" => TokenType::Static,
                "unsafe" => TokenType::Unsafe,
                "as" => TokenType::As,
                "true" => TokenType::Boolean(true),
                "false" => TokenType::Boolean(false),
                "None" => TokenType::None,
                "Some" => TokenType::Some,
                "in" => TokenType::In,
                _ => TokenType::Identifier(ident),
            };
            return Ok(Token {
                token_type,
                line,
                column,
            });
        }

        // Operators and punctuation
        let token_type = match ch {
            '+' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::PlusEqual
                } else {
                    TokenType::Plus
                }
            },
            '-' => {
                if self.peek() == '>' {
                    self.advance();
                    TokenType::Arrow
                } else if self.peek() == '=' {
                    self.advance();
                    TokenType::MinusEqual
                } else {
                    TokenType::Minus
                }
            },
            '*' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::StarEqual
                } else {
                    TokenType::Star
                }
            },
            '/' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::SlashEqual
                } else {
                    TokenType::Slash
                }
            },
            '%' => TokenType::Percent,
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::EqualEqual
                } else if self.peek() == '>' {
                    self.advance();
                    TokenType::FatArrow
                } else {
                    TokenType::Equal
                }
            },
            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::NotEqual
                } else {
                    TokenType::Not
                }
            },
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::LessEqual
                } else if self.peek() == '<' {
                    self.advance();
                    TokenType::LeftShift
                } else {
                    TokenType::Less
                }
            },
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenType::GreaterEqual
                } else if self.peek() == '>' {
                    self.advance();
                    TokenType::RightShift
                } else {
                    TokenType::Greater
                }
            },
            '&' => {
                if self.peek() == '&' {
                    self.advance();
                    TokenType::And
                } else {
                    TokenType::Ampersand
                }
            },
            '|' => {
                if self.peek() == '|' {
                    self.advance();
                    TokenType::Or
                } else {
                    TokenType::Pipe
                }
            },
            '^' => TokenType::Caret,
            '~' => TokenType::Tilde,
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            ';' => TokenType::Semicolon,
            ':' => TokenType::Colon,
            ',' => TokenType::Comma,
            '.' => {
                if self.peek() == '.' {
                    self.advance();
                    if self.peek() == '=' {
                        self.advance();
                        TokenType::DotDotEqual
                    } else {
                        TokenType::DotDot
                    }
                } else {
                    TokenType::Dot
                }
            },
            '?' => TokenType::Question,
            _ => return Err(format!("Unexpected character: {} at line {}", ch, self.line)),
        };

        self.advance();
        Ok(Token {
            token_type,
            line,
            column,
        })
    }

    fn read_number(&mut self) -> Result<TokenType, String> {
        let mut num_str = String::new();
        let mut is_float = false;

        while self.position < self.source.len() {
            let ch = self.current_char();

            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float && self.peek().is_ascii_digit() {
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance(); // Skip underscores in numbers
            } else {
                break;
            }
        }

        if is_float {
            let value = num_str.parse::<f64>()
                .map_err(|_| format!("Invalid float: {}", num_str))?;
            Ok(TokenType::Float(value))
        } else {
            let value = num_str.parse::<i64>()
                .map_err(|_| format!("Invalid integer: {}", num_str))?;
            Ok(TokenType::Integer(value))
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.advance(); // Skip opening quote
        let mut result = String::new();

        while self.position < self.source.len() {
            let ch = self.current_char();

            if ch == '"' {
                self.advance(); // Skip closing quote
                return Ok(result);
            }

            if ch == '\\' && self.position + 1 < self.source.len() {
                self.advance();
                let escaped = self.current_char();
                match escaped {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    _ => {
                        result.push('\\');
                        result.push(escaped);
                    }
                }
                self.advance();
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err("Unterminated string".to_string())
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();

        while self.position < self.source.len() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        ident
    }

    fn current_char(&self) -> char {
        self.source[self.position]
    }

    fn peek(&self) -> char {
        if self.position + 1 < self.source.len() {
            self.source[self.position + 1]
        } else {
            '\0'
        }
    }

    fn advance(&mut self) {
        if self.position < self.source.len() {
            self.column += 1;
            self.position += 1;
        }
    }
}
