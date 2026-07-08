// AXIOM LEXER

#[derive(Debug, Clone)]
pub enum Token {
    Theorem, Proof, Assume, Show, Qed,
    Forall, Exists, Implies, And, Or, Not,
    Identifier(String), Number(i64), Eof,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => { chars.next(); },
            'a'..='z' | 'A'..='Z' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else { break; }
                }
                tokens.push(match ident.as_str() {
                    "theorem" => Token::Theorem,
                    "proof" => Token::Proof,
                    "assume" => Token::Assume,
                    "show" => Token::Show,
                    "qed" => Token::Qed,
                    "forall" => Token::Forall,
                    "exists" => Token::Exists,
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
