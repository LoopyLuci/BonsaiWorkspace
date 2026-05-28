//! Sylva lexer — tokenizes Sylva source text.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,

    // Identifiers
    Ident(String),

    // Keywords
    Let, Mut, Fn, If, Else, While, For, In,
    Return, Break, Continue,
    And, Or, Not,
    Import, Export, Struct, Enum, Type, Match, With,
    Await, Async, Spawn, Send, Receive,
    Df,    // dataframe literal keyword
    Array, // @array device hint

    // Operators
    Plus, Minus, Star, Slash, Percent,
    Eqq, Ne, Lt, Le, Gt, Ge,
    Assign,
    PlusAssign, MinusAssign,
    Arrow, FatArrow,
    Pipe,         // |>  pipeline operator
    Concat,       // ++  string/array concat

    // Punctuation
    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Comma, Colon, Semicolon, Dot, DotDot, Question, Bang,
    Hash, At,

    Eof,
}

#[derive(Debug, Clone)]
pub struct LexError(pub String, pub usize); // (message, line)

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error at line {}: {}", self.1, self.0)
    }
}

pub type LexResult<T> = Result<T, LexError>;

#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

pub fn lex(src: &str) -> LexResult<Vec<Spanned>> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1usize;
    let mut col = 1usize;

    macro_rules! spanned {
        ($tok:expr) => { Spanned { token: $tok, line, col } };
    }

    while i < chars.len() {
        let c = chars[i];

        // skip whitespace
        if c == '\n' { line += 1; col = 1; i += 1; continue; }
        if c == ' ' || c == '\t' || c == '\r' { col += 1; i += 1; continue; }

        // line comments
        if c == '/' && chars.get(i+1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' { i += 1; }
            continue;
        }
        // block comments
        if c == '/' && chars.get(i+1) == Some(&'*') {
            i += 2; col += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i+1] == '/') {
                if chars[i] == '\n' { line += 1; col = 1; } else { col += 1; }
                i += 1;
            }
            i += 2; col += 2;
            continue;
        }

        // strings
        if c == '"' {
            let start_line = line;
            let start_col = col;
            i += 1; col += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1; col += 1;
                    match chars.get(i) {
                        Some('n')  => { s.push('\n'); i += 1; col += 1; }
                        Some('t')  => { s.push('\t'); i += 1; col += 1; }
                        Some('"')  => { s.push('"');  i += 1; col += 1; }
                        Some('\\') => { s.push('\\'); i += 1; col += 1; }
                        Some('0')  => { s.push('\0'); i += 1; col += 1; }
                        other => return Err(LexError(format!("unknown escape \\{:?}", other), line)),
                    }
                } else {
                    if chars[i] == '\n' { line += 1; col = 1; } else { col += 1; }
                    s.push(chars[i]);
                    i += 1;
                }
            }
            if i >= chars.len() {
                return Err(LexError("unterminated string".into(), start_line));
            }
            i += 1; col += 1;
            out.push(Spanned { token: Token::Str(s), line: start_line, col: start_col });
            continue;
        }

        // numbers
        if c.is_ascii_digit() || (c == '-' && chars.get(i+1).map_or(false, |x| x.is_ascii_digit())) {
            let start = i;
            let start_col = col;
            if c == '-' { i += 1; col += 1; }
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; col += 1; }
            if i < chars.len() && chars[i] == '.' && chars.get(i+1).map_or(false, |x| x.is_ascii_digit()) {
                i += 1; col += 1;
                while i < chars.len() && chars[i].is_ascii_digit() { i += 1; col += 1; }
                let s: String = chars[start..i].iter().collect();
                let f: f64 = s.parse().map_err(|_| LexError(format!("bad float: {s}"), line))?;
                out.push(Spanned { token: Token::Float(f), line, col: start_col });
            } else {
                let s: String = chars[start..i].iter().collect();
                let n: i64 = s.parse().map_err(|_| LexError(format!("bad int: {s}"), line))?;
                out.push(Spanned { token: Token::Int(n), line, col: start_col });
            }
            continue;
        }

        // identifiers and keywords
        if c.is_alphabetic() || c == '_' {
            let start = i;
            let start_col = col;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; col += 1; }
            let word: String = chars[start..i].iter().collect();
            let tok = keyword_or_ident(word);
            out.push(Spanned { token: tok, line, col: start_col });
            continue;
        }

        // two-char operators
        macro_rules! try2 {
            ($next:expr, $tok2:expr, $tok1:expr) => {
                if chars.get(i+1) == Some(&$next) {
                    out.push(spanned!($tok2)); i += 2; col += 2; continue;
                } else {
                    out.push(spanned!($tok1)); i += 1; col += 1; continue;
                }
            };
        }

        match c {
            '+' => { try2!('=', Token::PlusAssign,  Token::Plus) }
            '-' => {
                if chars.get(i+1) == Some(&'>') { out.push(spanned!(Token::Arrow)); i+=2; col+=2; continue; }
                try2!('=', Token::MinusAssign, Token::Minus)
            }
            '=' => {
                if chars.get(i+1) == Some(&'>') { out.push(spanned!(Token::FatArrow)); i+=2; col+=2; continue; }
                try2!('=', Token::Eqq, Token::Assign)
            }
            '!' => { try2!('=', Token::Ne, Token::Bang) }
            '<' => { try2!('=', Token::Le, Token::Lt) }
            '>' => { try2!('=', Token::Ge, Token::Gt) }
            '|' => {
                if chars.get(i+1) == Some(&'>') { out.push(spanned!(Token::Pipe)); i+=2; col+=2; continue; }
                if chars.get(i+1) == Some(&'|') { out.push(spanned!(Token::Or)); i+=2; col+=2; continue; }
                out.push(spanned!(Token::Pipe)); i+=1; col+=1; continue;
            }
            '&' => {
                if chars.get(i+1) == Some(&'&') { out.push(spanned!(Token::And)); i+=2; col+=2; continue; }
                out.push(spanned!(Token::Bang)); i+=1; col+=1; continue; // treat lone & as bang (not used)
            }
            '+' if chars.get(i+1) == Some(&'+') => { out.push(spanned!(Token::Concat)); i+=2; col+=2; continue; }
            '.' => {
                if chars.get(i+1) == Some(&'.') { out.push(spanned!(Token::DotDot)); i+=2; col+=2; continue; }
                out.push(spanned!(Token::Dot)); i+=1; col+=1; continue;
            }
            '*' => { out.push(spanned!(Token::Star));     i+=1; col+=1; continue; }
            '/' => { out.push(spanned!(Token::Slash));    i+=1; col+=1; continue; }
            '%' => { out.push(spanned!(Token::Percent));  i+=1; col+=1; continue; }
            '(' => { out.push(spanned!(Token::LParen));   i+=1; col+=1; continue; }
            ')' => { out.push(spanned!(Token::RParen));   i+=1; col+=1; continue; }
            '{' => { out.push(spanned!(Token::LBrace));   i+=1; col+=1; continue; }
            '}' => { out.push(spanned!(Token::RBrace));   i+=1; col+=1; continue; }
            '[' => { out.push(spanned!(Token::LBracket)); i+=1; col+=1; continue; }
            ']' => { out.push(spanned!(Token::RBracket)); i+=1; col+=1; continue; }
            ',' => { out.push(spanned!(Token::Comma));    i+=1; col+=1; continue; }
            ':' => { out.push(spanned!(Token::Colon));    i+=1; col+=1; continue; }
            ';' => { out.push(spanned!(Token::Semicolon)); i+=1; col+=1; continue; }
            '#' => { out.push(spanned!(Token::Hash));     i+=1; col+=1; continue; }
            '@' => { out.push(spanned!(Token::At));       i+=1; col+=1; continue; }
            '?' => { out.push(spanned!(Token::Question)); i+=1; col+=1; continue; }
            other => return Err(LexError(format!("unexpected character: {other:?}"), line)),
        }
    }

    out.push(Spanned { token: Token::Eof, line, col });
    Ok(out)
}

fn keyword_or_ident(s: String) -> Token {
    match s.as_str() {
        "let"      => Token::Let,
        "mut"      => Token::Mut,
        "fn"       => Token::Fn,
        "if"       => Token::If,
        "else"     => Token::Else,
        "while"    => Token::While,
        "for"      => Token::For,
        "in"       => Token::In,
        "return"   => Token::Return,
        "break"    => Token::Break,
        "continue" => Token::Continue,
        "and"      => Token::And,
        "or"       => Token::Or,
        "not"      => Token::Not,
        "import"   => Token::Import,
        "export"   => Token::Export,
        "struct"   => Token::Struct,
        "enum"     => Token::Enum,
        "type"     => Token::Type,
        "match"    => Token::Match,
        "with"     => Token::With,
        "await"    => Token::Await,
        "async"    => Token::Async,
        "spawn"    => Token::Spawn,
        "send"     => Token::Send,
        "receive"  => Token::Receive,
        "nil"      => Token::Nil,
        "true"     => Token::Bool(true),
        "false"    => Token::Bool(false),
        _          => Token::Ident(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_numbers() {
        let toks = lex("42 3.14 -7").unwrap();
        assert!(matches!(toks[0].token, Token::Int(42)));
        assert!(matches!(toks[1].token, Token::Float(_)));
        assert!(matches!(toks[2].token, Token::Int(-7)));
    }

    #[test]
    fn lex_string() {
        let toks = lex(r#""hello\nworld""#).unwrap();
        assert_eq!(toks[0].token, Token::Str("hello\nworld".into()));
    }

    #[test]
    fn lex_keywords() {
        let toks = lex("let fn if else return").unwrap();
        assert!(matches!(toks[0].token, Token::Let));
        assert!(matches!(toks[1].token, Token::Fn));
        assert!(matches!(toks[2].token, Token::If));
    }

    #[test]
    fn lex_operators() {
        let toks = lex("== != <= >= -> =>").unwrap();
        assert!(matches!(toks[0].token, Token::Eqq));
        assert!(matches!(toks[1].token, Token::Ne));
        assert!(matches!(toks[2].token, Token::Le));
        assert!(matches!(toks[3].token, Token::Ge));
        assert!(matches!(toks[4].token, Token::Arrow));
        assert!(matches!(toks[5].token, Token::FatArrow));
    }
}
