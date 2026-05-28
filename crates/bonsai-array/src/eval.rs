//! Simple tokenizer + evaluator for a J/APL-inspired expression syntax.
//!
//! Grammar (simplified):
//!   expr     ::= dyad | monad | atom
//!   monad    ::= VERB expr
//!   dyad     ::= atom VERB expr
//!   atom     ::= NUMBER | '(' expr ')' | '[' expr (',' expr)* ']'
//!   VERB     ::= '+' | '-' | '×' | '÷' | '|' | '⌈' | '⌊'
//!              | '<' | '≤' | '=' | '≥' | '>' | '≠'
//!              | '∧' | '∨' | '~' | '#' | '⍴' | '⍋' | '⍒'
//!              | '⌽' | '⍉' | ',' | '+/' | '×/' | '⌈/' | '⌊/'
//!              | '+\' | '×\' | '+.×'

use crate::array::NdArray;
use crate::error::ArrayError;
use crate::ops;

pub type EvalResult = Result<NdArray, ArrayError>;

// ── Tokens ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Verb(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<Tok>, ArrayError> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' || c == '\t' { i += 1; continue; }
        if c == '(' { out.push(Tok::LParen);   i += 1; continue; }
        if c == ')' { out.push(Tok::RParen);   i += 1; continue; }
        if c == '[' { out.push(Tok::LBracket); i += 1; continue; }
        if c == ']' { out.push(Tok::RBracket); i += 1; continue; }
        if c == ',' { out.push(Tok::Comma);    i += 1; continue; }

        // Numbers (including negative via leading ¯ or -)
        let neg_prefix = (c == '¯' || (c == '-' && i + 1 < chars.len() && chars[i+1].is_ascii_digit()));
        if c.is_ascii_digit() || neg_prefix {
            let start = i;
            if c == '¯' || c == '-' { i += 1; }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
            let s: String = chars[start..i].iter().map(|&c| if c == '¯' { '-' } else { c }).collect();
            let v: f64 = s.parse().map_err(|_| ArrayError::SyntaxError(format!("bad number: {s}")))?;
            out.push(Tok::Num(v));
            continue;
        }

        // Multi-char verbs
        let remaining: String = chars[i..].iter().collect();
        if remaining.starts_with("+.×") { out.push(Tok::Verb("+.×".into())); i += 3; continue; }
        if remaining.starts_with("∘.") && i + 2 < chars.len() {
            let verb: String = chars[i..i+3].iter().collect();
            out.push(Tok::Verb(verb)); i += 3; continue;
        }

        // Reductions / scans (verb followed by / or \)
        let verb_chars = ['+','-','×','÷','|','⌈','⌊','⌽','⍉','⍋','⍒',',','∧','∨','~','#','⍴'];
        if verb_chars.contains(&c) {
            let v = c.to_string();
            i += 1;
            if i < chars.len() && (chars[i] == '/' || chars[i] == '\\') {
                let sym = chars[i];
                out.push(Tok::Verb(format!("{v}{sym}")));
                i += 1;
            } else {
                out.push(Tok::Verb(v));
            }
            continue;
        }

        // Unicode comparison operators
        let uc_verbs = ['<','≤','=','≥','>','≠'];
        if uc_verbs.contains(&c) { out.push(Tok::Verb(c.to_string())); i += 1; continue; }

        return Err(ArrayError::SyntaxError(format!("unexpected char: {c:?}")));
    }
    Ok(out)
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser { tokens: Vec<Tok>, pos: usize }

impl Parser {
    fn new(tokens: Vec<Tok>) -> Self { Self { tokens, pos: 0 } }
    fn peek(&self) -> Option<&Tok> { self.tokens.get(self.pos) }
    fn advance(&mut self) -> Option<Tok> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else { None }
    }

    fn parse_expr(&mut self) -> EvalResult {
        // APL is right-to-left; we implement as: parse atom, then check for dyadic verb
        let left = self.parse_atom()?;
        if let Some(Tok::Verb(v)) = self.peek().cloned() {
            self.advance();
            let right = self.parse_expr()?;
            return apply_dyad(&v, &left, &right);
        }
        Ok(left)
    }

    fn parse_atom(&mut self) -> EvalResult {
        match self.peek().cloned() {
            Some(Tok::Verb(v)) => {
                self.advance();
                let arg = self.parse_atom()?;
                apply_monad(&v, &arg)
            }
            Some(Tok::LParen) => {
                self.advance();
                let e = self.parse_expr()?;
                match self.advance() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err(ArrayError::SyntaxError("expected )".into())),
                }
            }
            Some(Tok::LBracket) => {
                self.advance();
                let mut vals = Vec::new();
                loop {
                    match self.peek() {
                        Some(Tok::RBracket) => { self.advance(); break; }
                        None => return Err(ArrayError::SyntaxError("unterminated [".into())),
                        _ => {
                            let e = self.parse_atom()?;
                            // Flatten scalars into vector
                            if e.is_scalar() {
                                vals.push(e.data[0]);
                            } else {
                                vals.extend_from_slice(&e.data);
                            }
                            if matches!(self.peek(), Some(Tok::Comma)) { self.advance(); }
                        }
                    }
                }
                Ok(NdArray::vector(vals))
            }
            Some(Tok::Num(v)) => {
                self.advance();
                Ok(NdArray::scalar(v))
            }
            other => Err(ArrayError::SyntaxError(format!("unexpected token: {other:?}"))),
        }
    }
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

fn apply_monad(v: &str, a: &NdArray) -> EvalResult {
    match v {
        "-"  => Ok(ops::neg(a)),
        "|"  => Ok(ops::abs(a)),
        "⌈"  => Ok(ops::ceil(a)),
        "⌊"  => Ok(ops::floor(a)),
        "~"  => Ok(ops::not(a)),
        "⌽"  => Ok(a.reverse()),
        "⍉"  => Ok(a.transpose()),
        ","  => Ok(a.ravel()),
        "⍋"  => Ok(a.grade_up()),
        "⍒"  => Ok(a.grade_down()),
        "#"  => Ok(NdArray::scalar(a.len() as f64)),
        "≡"  => Ok(NdArray::scalar(a.rank() as f64)),
        "⍴"  => Ok(NdArray::vector(a.shape.iter().map(|&s| s as f64).collect())),
        "+/" => Ok(ops::sum(a)),
        "×/" => Ok(ops::product(a)),
        "⌈/" => Ok(ops::max_reduce(a)),
        "⌊/" => Ok(ops::min_reduce(a)),
        "+\\" => Ok(ops::scan_sum(a)),
        "×\\" => Ok(ops::scan_product(a)),
        _ => Err(ArrayError::DomainError(format!("unknown monadic verb: {v}"))),
    }
}

fn apply_dyad(v: &str, a: &NdArray, b: &NdArray) -> EvalResult {
    match v {
        "+"  => ops::add(a, b),
        "-"  => ops::sub(a, b),
        "×"  => ops::mul(a, b),
        "÷"  => ops::div(a, b),
        "|"  => ops::rem(a, b),
        "⌈"  => ops::map2(a, b, f64::max),
        "⌊"  => ops::map2(a, b, f64::min),
        "*"  => ops::pow(a, b),
        "<"  => ops::lt(a, b),
        "≤"  => ops::le(a, b),
        "="  => ops::eq(a, b),
        "≥"  => ops::ge(a, b),
        ">"  => ops::gt(a, b),
        "≠"  => ops::ne(a, b),
        "∧"  => ops::and(a, b),
        "∨"  => ops::or(a, b),
        ","  => a.catenate(b),
        "+.×"=> ops::matmul(a, b),
        "⍴"  => {
            // Reshape: left is shape vector, right is data
            let shape: Vec<usize> = a.data.iter().map(|&x| x as usize).collect();
            Ok(b.reshape(shape))
        }
        "↑"  => a.take(b.scalar_val().unwrap_or(0.0) as i64),
        "↓"  => a.drop(b.scalar_val().unwrap_or(0.0) as i64),
        _ => Err(ArrayError::DomainError(format!("unknown dyadic verb: {v}"))),
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct AplEval;

impl AplEval {
    /// Evaluate an APL/J expression string and return an NdArray.
    pub fn eval(src: &str) -> EvalResult {
        let tokens = tokenize(src)?;
        let mut parser = Parser::new(tokens);
        let result = parser.parse_expr()?;
        Ok(result)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_scalar_add() {
        let r = AplEval::eval("3 + 4").unwrap();
        assert_eq!(r.scalar_val(), Some(7.0));
    }

    #[test]
    fn eval_vector_sum() {
        let r = AplEval::eval("+/ [1, 2, 3, 4]").unwrap();
        assert_eq!(r.scalar_val(), Some(10.0));
    }

    #[test]
    fn eval_scan_sum() {
        let r = AplEval::eval("+\\ [1, 2, 3, 4]").unwrap();
        assert_eq!(r.data, vec![1.0, 3.0, 6.0, 10.0]);
    }

    #[test]
    fn eval_grade() {
        let r = AplEval::eval("⍋ [3, 1, 2]").unwrap();
        assert_eq!(r.data, vec![1.0, 2.0, 0.0]);
    }

    #[test]
    fn eval_reshape() {
        // 2 3 ⍴ [1, 2, 3]  →  2×3 matrix
        let r = AplEval::eval("[2, 3] ⍴ [1, 2, 3]").unwrap();
        assert_eq!(r.shape, vec![2, 3]);
        assert_eq!(r.data.len(), 6);
    }
}
