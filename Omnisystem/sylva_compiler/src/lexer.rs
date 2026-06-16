// SYLVA LEXER

#[derive(Debug, Clone)]
pub enum Token {
    Tensor, Let, Model, Dense, Conv2d, Train, Predict,
    Randn, Zeros, Ones, Identifier(String), Number(f64),
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
            'a'..='z' | 'A'..='Z' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(match ident.as_str() {
                    \"tensor\" => Token::Tensor,
                    \"let\" => Token::Let,
                    \"model\" => Token::Model,
                    \"dense\" => Token::Dense,
                    \"conv2d\" => Token::Conv2d,
                    \"train\" => Token::Train,
                    \"predict\" => Token::Predict,
                    \"randn\" => Token::Randn,
                    _ => Token::Identifier(ident),
                });
            },
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_numeric() || c == '.' {
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num.parse().unwrap_or(0.0)));
            },
            _ => { chars.next(); },
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}
