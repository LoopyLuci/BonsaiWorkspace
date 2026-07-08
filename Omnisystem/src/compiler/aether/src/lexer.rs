// AETHER LEXER - Tokenization for Distributed Systems Language

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Actor, Message, Spawn, Send, Receive, Replicate,
    Identifier(String), Number(i64), String(String),
    LeftBrace, RightBrace, Comma, Colon, Equals, Eof,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => { chars.next(); },
            '{' => { tokens.push(Token::LeftBrace); chars.next(); },
            '}' => { tokens.push(Token::RightBrace); chars.next(); },
            ',' => { tokens.push(Token::Comma); chars.next(); },
            ':' => { tokens.push(Token::Colon); chars.next(); },
            '=' => { tokens.push(Token::Equals); chars.next(); },
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' { chars.next(); break; }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::String(s));
            },
            'a'..='z' | 'A'..='Z' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else { break; }
                }
                tokens.push(match ident.as_str() {
                    "actor" => Token::Actor,
                    "message" => Token::Message,
                    "spawn" => Token::Spawn,
                    "send" => Token::Send,
                    "receive" => Token::Receive,
                    "replicate" => Token::Replicate,
                    _ => Token::Identifier(ident),
                });
            },
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_numeric() {
                        num.push(c);
                        chars.next();
                    } else { break; }
                }
                tokens.push(Token::Number(num.parse().unwrap_or(0)));
            },
            _ => { chars.next(); },
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}
