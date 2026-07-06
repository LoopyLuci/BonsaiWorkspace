// Titan lexer — turns source text into a token stream with precise spans.
//
// Handles: line (//) and block (/* */, nested) comments, decimal/hex/bin/oct
// integer and float literals with `_` separators and type suffixes, string
// literals with escapes, char literals, lifetimes ('a), identifiers/keywords,
// and the full operator set (greedy longest-match).

import type { Pos, Span } from './diagnostics.ts';
import { OmniError } from './diagnostics.ts';
import type { Token, TokKind } from './token.ts';
import { KEYWORDS, OPERATORS } from './token.ts';

export class Lexer {
  private src: string;
  private file: string;
  private i = 0;
  private line = 1;
  private col = 1;

  constructor(src: string, file: string) {
    this.src = src;
    this.file = file;
  }

  private pos(): Pos {
    return { offset: this.i, line: this.line, col: this.col };
  }

  private error(msg: string, start: Pos, help: string | null = null): never {
    const sp: Span = { start, end: this.pos() };
    throw new OmniError('lex', msg, sp, this.file, this.src, help);
  }

  private peek(k = 0): string {
    return this.src[this.i + k] ?? '';
  }

  private advance(): string {
    const ch = this.src[this.i++];
    if (ch === '\n') {
      this.line++;
      this.col = 1;
    } else {
      this.col++;
    }
    return ch ?? '';
  }

  private starts(s: string): boolean {
    return this.src.startsWith(s, this.i);
  }

  tokenize(): Token[] {
    const toks: Token[] = [];
    for (;;) {
      this.skipTrivia();
      const start = this.pos();
      if (this.i >= this.src.length) {
        toks.push({ kind: 'eof', value: '', raw: '', span: { start, end: start } });
        return toks;
      }
      const ch = this.peek();
      if (ch === 'r' && this.isRawStringStart()) {
        toks.push(this.lexRawString(start));
      } else if (isIdentStart(ch)) {
        toks.push(this.lexIdent(start));
      } else if (isDigit(ch)) {
        toks.push(this.lexNumber(start));
      } else if (ch === '"') {
        toks.push(this.lexString(start));
      } else if (ch === "'") {
        toks.push(this.lexCharOrLifetime(start));
      } else {
        toks.push(this.lexOperator(start));
      }
    }
  }

  private skipTrivia(): void {
    for (;;) {
      const ch = this.peek();
      if (ch === ' ' || ch === '\t' || ch === '\r' || ch === '\n') {
        this.advance();
      } else if (ch === '/' && this.peek(1) === '/') {
        while (this.i < this.src.length && this.peek() !== '\n') this.advance();
      } else if (ch === '/' && this.peek(1) === '*') {
        this.skipBlockComment();
      } else {
        return;
      }
    }
  }

  private skipBlockComment(): void {
    const start = this.pos();
    this.advance();
    this.advance(); // consume /*
    let depth = 1;
    while (depth > 0) {
      if (this.i >= this.src.length) this.error('unterminated block comment', start);
      if (this.starts('/*')) {
        this.advance();
        this.advance();
        depth++;
      } else if (this.starts('*/')) {
        this.advance();
        this.advance();
        depth--;
      } else {
        this.advance();
      }
    }
  }

  private mk(kind: TokKind, value: string, raw: string, start: Pos): Token {
    return { kind, value, raw, span: { start, end: this.pos() } };
  }

  private lexIdent(start: Pos): Token {
    let s = '';
    while (isIdentContinue(this.peek())) s += this.advance();
    if (s === 'true' || s === 'false') return this.mk('bool', s, s, start);
    if (KEYWORDS.has(s)) return this.mk('keyword', s, s, start);
    return this.mk('ident', s, s, start);
  }

  private lexNumber(start: Pos): Token {
    let raw = '';
    let isFloat = false;
    if (this.peek() === '0' && (this.peek(1) === 'x' || this.peek(1) === 'b' || this.peek(1) === 'o')) {
      raw += this.advance();
      const base = this.advance(); // x/b/o
      raw += base;
      while (isHex(this.peek()) || this.peek() === '_') raw += this.advance();
      const digits = raw.slice(2).replace(/_/g, '');
      const radix = base === 'x' ? 16 : base === 'o' ? 8 : 2;
      const val = parseInt(digits, radix);
      let hsuffix = '';
      while (isIdentContinue(this.peek())) hsuffix += this.advance();
      return this.mk('int', String(val), raw + hsuffix, start);
    }
    while (isDigit(this.peek()) || this.peek() === '_') raw += this.advance();
    if (this.peek() === '.' && isDigit(this.peek(1))) {
      isFloat = true;
      raw += this.advance();
      while (isDigit(this.peek()) || this.peek() === '_') raw += this.advance();
    }
    if (this.peek() === 'e' || this.peek() === 'E') {
      isFloat = true;
      raw += this.advance();
      if (this.peek() === '+' || this.peek() === '-') raw += this.advance();
      while (isDigit(this.peek())) raw += this.advance();
    }
    // optional numeric type suffix (i32, u64, f64, usize, ...)
    let suffix = '';
    while (isIdentContinue(this.peek())) suffix += this.advance();
    if (suffix.startsWith('f')) isFloat = true;
    const clean = raw.replace(/_/g, '');
    return this.mk(isFloat ? 'float' : 'int', clean, raw + suffix, start);
  }

  private lexString(start: Pos): Token {
    this.advance(); // opening quote
    let out = '';
    let raw = '"';
    for (;;) {
      if (this.i >= this.src.length) this.error('unterminated string literal', start);
      const ch = this.advance();
      raw += ch;
      if (ch === '"') break;
      if (ch === '\\') {
        const e = this.advance();
        raw += e;
        if (e === '\n') {
          // Line continuation (Rust string literal semantics): the newline
          // and any leading whitespace on the next line contribute nothing
          // to the string's value, letting multi-line format!()/error
          // strings wrap without embedding a literal newline+indent.
          while (this.peek() === ' ' || this.peek() === '\t') raw += this.advance();
        } else {
          out += this.escape(e, start);
        }
      } else {
        out += ch;
      }
    }
    return this.mk('string', out, raw, start);
  }

  // Rust-style raw string literals: r"..." or r#"..."# / r##"..."## etc. — no
  // escape processing at all, so JSON/regex/paths embedded in Titan specs
  // (e.g. format!(r#"{{"id":"{}"}}"#, ...) in omni-integration/*.titan) don't
  // need every quote and backslash hand-escaped.
  private isRawStringStart(): boolean {
    let k = 1;
    while (this.peek(k) === '#') k++;
    return this.peek(k) === '"';
  }

  private lexRawString(start: Pos): Token {
    let raw = this.advance(); // 'r'
    let hashes = 0;
    while (this.peek() === '#') {
      raw += this.advance();
      hashes++;
    }
    raw += this.advance(); // opening quote
    const closer = '"' + '#'.repeat(hashes);
    let out = '';
    for (;;) {
      if (this.i >= this.src.length) this.error('unterminated raw string literal', start);
      if (this.starts(closer)) {
        for (let k = 0; k < closer.length; k++) raw += this.advance();
        break;
      }
      out += this.advance();
    }
    return this.mk('string', out, raw, start);
  }

  private lexCharOrLifetime(start: Pos): Token {
    // Could be a char literal 'a' / '\n' or a lifetime 'a.
    this.advance(); // opening '
    // lifetime: '<ident> not followed by closing quote
    if (isIdentStart(this.peek())) {
      const save = this.i;
      let name = '';
      while (isIdentContinue(this.peek())) name += this.advance();
      if (this.peek() !== "'") {
        return this.mk('lifetime', name, "'" + name, start);
      }
      // it was a char like 'a' — rewind and fall through
      this.i = save;
      this.col = start.col + 1;
      this.line = start.line;
    }
    let out = '';
    const ch = this.advance();
    if (ch === '\\') {
      out += this.escape(this.advance(), start);
    } else {
      out += ch;
    }
    if (this.peek() !== "'") this.error("unterminated char literal", start, "char literals hold exactly one character, e.g. 'a'");
    this.advance();
    return this.mk('char', out, "'" + out + "'", start);
  }

  private escape(e: string, start: Pos): string {
    switch (e) {
      case 'n': return '\n';
      case 't': return '\t';
      case 'r': return '\r';
      case '0': return '\0';
      case '\\': return '\\';
      case '"': return '"';
      case "'": return "'";
      default: this.error(`unknown escape sequence \\${e}`, start);
    }
  }

  private lexOperator(start: Pos): Token {
    for (const op of OPERATORS) {
      if (this.starts(op)) {
        for (let k = 0; k < op.length; k++) this.advance();
        return this.mk('op', op, op, start);
      }
    }
    const bad = this.advance();
    this.error(`unexpected character '${bad}'`, start);
  }
}

function isDigit(c: string): boolean {
  return c >= '0' && c <= '9';
}
function isHex(c: string): boolean {
  return isDigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
}
function isIdentStart(c: string): boolean {
  return c === '_' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
function isIdentContinue(c: string): boolean {
  return isIdentStart(c) || isDigit(c);
}
