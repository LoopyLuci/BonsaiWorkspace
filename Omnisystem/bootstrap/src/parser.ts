// Titan parser — recursive descent for items/statements/patterns/types and a
// Pratt (precedence-climbing) expression parser. Produces the AST in ast.ts.
//
// Newlines are insignificant except that a statement may end at a newline
// without a `;` (Titan style). We treat `;` as optional and use expression
// boundaries to separate statements.

import type { Pos, Span } from './diagnostics.ts';
import { OmniError } from './diagnostics.ts';
import type { Token } from './token.ts';
import { Lexer } from './lexer.ts';
import type * as A from './ast.ts';

// Binary operator precedence (higher binds tighter).
const BIN_PREC: Record<string, number> = {
  '||': 1,
  '&&': 2,
  '==': 3, '!=': 3, '<': 3, '>': 3, '<=': 3, '>=': 3,
  '|': 4, '^': 5, '&': 6,
  '<<': 7, '>>': 7,
  '+': 8, '-': 8,
  '*': 9, '/': 9, '%': 9,
};
const ASSIGN_OPS = new Set(['=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=']);

export class Parser {
  private toks: Token[];
  private p = 0;
  private file: string;
  private src: string;

  constructor(src: string, file: string) {
    this.src = src;
    this.file = file;
    this.toks = new Lexer(src, file).tokenize();
  }

  // ── token helpers ─────────────────────────────────────────────────────────
  private cur(): Token { return this.toks[this.p]; }
  private next(): Token { return this.toks[this.p + 1] ?? this.toks[this.toks.length - 1]; }
  private atEof(): boolean { return this.cur().kind === 'eof'; }
  private advance(): Token { return this.toks[this.p++]; }

  private is(v: string): boolean {
    const t = this.cur();
    return (t.kind === 'op' || t.kind === 'keyword') && t.value === v;
  }
  private isKw(v: string): boolean {
    const t = this.cur();
    return t.kind === 'keyword' && t.value === v;
  }
  private eat(v: string): boolean {
    if (this.is(v)) { this.advance(); return true; }
    return false;
  }
  private expect(v: string): Token {
    if (this.is(v)) return this.advance();
    this.err(`expected '${v}' but found '${this.cur().raw || this.cur().kind}'`, this.cur().span);
  }
  private expectIdent(): string {
    const t = this.cur();
    if (t.kind === 'ident' || (t.kind === 'keyword' && (t.value === 'self' || t.value === 'Self'))) {
      this.advance();
      return t.value;
    }
    this.err(`expected identifier but found '${t.raw || t.kind}'`, t.span);
  }
  private err(msg: string, sp: Span, help: string | null = null): never {
    throw new OmniError('parse', msg, sp, this.file, this.src, help);
  }

  // Angle-bracket close handling: nested generics end with `>>` (or `>>=`),
  // which the lexer produces as a single token. Treat any leading '>' as a
  // closing angle bracket, shortening the token in place if needed.
  private isGt(): boolean {
    const t = this.cur();
    return t.kind === 'op' && (t.value === '>' || t.value === '>>' || t.value === '>=' || t.value === '>>=');
  }
  private expectGt(): void {
    const t = this.cur();
    if (t.kind === 'op' && t.value === '>') { this.advance(); return; }
    if (t.kind === 'op' && (t.value === '>>' || t.value === '>=' || t.value === '>>=')) {
      t.value = t.value.slice(1);
      t.raw = t.raw.slice(1);
      return;
    }
    this.err(`expected '>' but found '${t.raw || t.kind}'`, t.span);
  }
  private sp(start: Pos): Span {
    const prev = this.toks[this.p - 1] ?? this.cur();
    return { start, end: prev.span.end };
  }
  private startPos(): Pos { return this.cur().span.start; }

  // skip optional statement separators
  private skipSemis(): void { while (this.eat(';')) { /* */ } }

  // ── program ───────────────────────────────────────────────────────────────
  parseProgram(): A.Program {
    const items: A.Item[] = [];
    this.skipSemis();
    while (!this.atEof()) {
      const it = this.parseItem();
      if (process.env.OMNI_DEBUG) {
        const nm = ('name' in it) ? (it as { name: string }).name : '';
        process.stderr.write(`item @${it.span.start.line}: ${it.kind} ${nm}\n`);
      }
      items.push(it);
      this.skipSemis();
    }
    return { items, file: this.file, source: this.src };
  }

  // ── items ─────────────────────────────────────────────────────────────────
  private parseItem(): A.Item {
    const start = this.startPos();
    const pub = this.eat('pub');
    // pub(crate) / pub(super) etc.
    if (pub && this.is('(')) { this.skipBalanced('(', ')'); }
    this.skipAttributes();

    if (this.isKw('use')) return this.parseUse(start);
    if (this.isKw('struct')) return this.parseStruct(start, pub);
    if (this.isKw('enum')) return this.parseEnum(start, pub);
    if (this.isKw('impl')) return this.parseImpl(start);
    if (this.isKw('trait')) return this.parseTrait(start, pub);
    if (this.isKw('fn') || this.isKw('async')) return this.parseFn(start, pub);
    if (this.isKw('const') || this.isKw('static')) return this.parseConst(start, pub);
    if (this.isKw('mod')) return this.parseMod(start, pub);
    if (this.isKw('type')) { // type alias — parse and ignore body
      this.advance(); this.expectIdent();
      if (this.is('<')) this.skipBalanced('<', '>');
      this.expect('='); this.parseType(); this.eat(';');
      return { kind: 'use', path: [], names: [], span: this.sp(start) };
    }
    if (this.isKw('extern')) {
      // `extern "C" { fn foo(...) -> T; ... }` FFI blocks — the tree-walking
      // interpreter can't call real native functions, so these are parsed
      // (ABI string + balanced brace skip) and discarded rather than
      // rejected, matching the `type` alias handling above.
      this.advance();
      if (this.cur().kind === 'string') this.advance();
      if (this.is('{')) this.skipBalanced('{', '}');
      else this.eat(';'); // `extern crate foo;` form, no block
      return { kind: 'use', path: [], names: [], span: this.sp(start) };
    }
    this.err(`expected an item (fn/struct/enum/impl/use/...) but found '${this.cur().raw || this.cur().kind}'`, this.cur().span);
  }

  private skipAttributes(): void {
    while (this.is('#')) {
      this.advance();
      this.eat('!');
      if (this.is('[')) this.skipBalanced('[', ']');
    }
  }

  private skipBalanced(open: string, close: string): void {
    this.expect(open);
    let depth = 1;
    while (depth > 0 && !this.atEof()) {
      if (this.is(open)) depth++;
      else if (this.is(close)) depth--;
      this.advance();
    }
  }

  private parseUse(start: Pos): A.UseItem {
    this.expect('use');
    const path: string[] = [];
    const names: string[] = [];
    path.push(this.expectIdent());
    while (this.eat('::')) {
      if (this.is('{')) {
        this.advance();
        while (!this.is('}') && !this.atEof()) {
          names.push(this.expectIdent());
          // handle `as alias`
          if (this.eat('as')) this.expectIdent();
          if (!this.eat(',')) break;
        }
        this.expect('}');
        break;
      } else if (this.is('*')) {
        this.advance();
        break;
      } else {
        path.push(this.expectIdent());
      }
    }
    // trailing single name is the last path segment
    if (names.length === 0 && path.length > 1) {
      names.push(path[path.length - 1]);
    }
    this.eat(';');
    return { kind: 'use', path, names, span: this.sp(start) };
  }

  private parseGenerics(): string[] {
    const gs: string[] = [];
    if (!this.is('<')) return gs;
    this.advance();
    while (!this.isGt() && !this.atEof()) {
      if (this.cur().kind === 'lifetime') { this.advance(); }
      else { this.eat('const'); gs.push(this.expectIdent()); }
      // bounds: T: Trait + Trait2
      if (this.eat(':')) { this.parseBounds(); }
      if (this.eat('=')) { this.parseType(); }
      if (!this.eat(',')) break;
    }
    this.expectGt();
    return gs;
  }

  private parseBounds(): void {
    for (;;) {
      if (this.cur().kind === 'lifetime') this.advance();
      else this.parseType();
      if (!this.eat('+')) break;
    }
  }

  private parseWhere(): void {
    if (!this.isKw('where')) return;
    this.advance();
    while (!this.is('{') && !this.is(';') && !this.atEof()) this.advance();
  }

  private parseStruct(start: Pos, pub: boolean): A.StructItem {
    this.expect('struct');
    const name = this.expectIdent();
    const generics = this.parseGenerics();
    this.parseWhere();
    const fields: A.Field[] = [];
    let tuple: A.TypeRef[] | null = null;
    if (this.eat('(')) {
      tuple = [];
      while (!this.is(')') && !this.atEof()) {
        this.eat('pub');
        tuple.push(this.parseType());
        if (!this.eat(',')) break;
      }
      this.expect(')');
      this.eat(';');
    } else if (this.eat('{')) {
      while (!this.is('}') && !this.atEof()) {
        this.skipAttributes();
        const fpub = this.eat('pub');
        if (fpub && this.is('(')) this.skipBalanced('(', ')');
        const fname = this.expectIdent();
        this.expect(':');
        const ty = this.parseType();
        fields.push({ name: fname, ty, pub: fpub });
        if (!this.eat(',')) break;
      }
      this.expect('}');
    } else {
      this.eat(';'); // unit struct
    }
    return { kind: 'struct', name, pub, generics, fields, tuple, span: this.sp(start) };
  }

  private parseEnum(start: Pos, pub: boolean): A.EnumItem {
    this.expect('enum');
    const name = this.expectIdent();
    const generics = this.parseGenerics();
    this.parseWhere();
    this.expect('{');
    const variants: A.EnumVariant[] = [];
    while (!this.is('}') && !this.atEof()) {
      this.skipAttributes();
      const vname = this.expectIdent();
      const vfields: A.TypeRef[] = [];
      let structFields: A.Field[] | null = null;
      if (this.eat('(')) {
        while (!this.is(')') && !this.atEof()) {
          vfields.push(this.parseType());
          if (!this.eat(',')) break;
        }
        this.expect(')');
      } else if (this.eat('{')) {
        structFields = [];
        while (!this.is('}') && !this.atEof()) {
          const fn = this.expectIdent();
          this.expect(':');
          const ty = this.parseType();
          structFields.push({ name: fn, ty, pub: true });
          if (!this.eat(',')) break;
        }
        this.expect('}');
      } else if (this.eat('=')) {
        this.parseExpr(); // discriminant
      }
      variants.push({ name: vname, fields: vfields, structFields });
      if (!this.eat(',')) break;
    }
    this.expect('}');
    return { kind: 'enum', name, pub, generics, variants, span: this.sp(start) };
  }

  private parseImpl(start: Pos): A.ImplItem {
    this.expect('impl');
    const generics = this.parseGenerics();
    let first = this.parseType();
    let trait: string | null = null;
    let targetTy = first;
    if (this.eat('for')) {
      trait = first.name;
      targetTy = this.parseType();
    } else if (this.isKw('for')) {
      // handled above
    }
    this.parseWhere();
    this.expect('{');
    const methods: A.FnItem[] = [];
    const consts: A.ConstItem[] = [];
    this.skipSemis();
    while (!this.is('}') && !this.atEof()) {
      this.skipAttributes();
      const mpub = this.eat('pub');
      if (mpub && this.is('(')) this.skipBalanced('(', ')');
      // associated const / type alias (Titan omits trailing ';', so parse them)
      if (this.isKw('const') || this.isKw('static')) {
        consts.push(this.parseConst(this.startPos(), mpub));
        this.skipSemis();
        continue;
      }
      if (this.isKw('type')) {
        this.advance(); this.expectIdent();
        if (this.is('<')) this.skipBalanced('<', '>');
        if (this.eat('=')) this.parseType();
        this.eat(';');
        continue;
      }
      methods.push(this.parseFn(this.startPos(), mpub));
      this.skipSemis();
    }
    this.expect('}');
    return { kind: 'impl', trait, target: targetTy.name, generics, methods, consts, span: this.sp(start) };
  }

  private parseTrait(start: Pos, pub: boolean): A.TraitItem {
    this.expect('trait');
    const name = this.expectIdent();
    this.parseGenerics();
    if (this.eat(':')) this.parseBounds();
    this.parseWhere();
    this.expect('{');
    const methods: A.FnItem[] = [];
    while (!this.is('}') && !this.atEof()) {
      this.skipAttributes();
      this.eat('pub');
      if (this.isKw('type') || this.isKw('const')) {
        while (!this.is(';') && !this.atEof()) this.advance();
        this.eat(';');
        continue;
      }
      methods.push(this.parseFn(this.startPos(), true));
      this.skipSemis();
    }
    this.expect('}');
    return { kind: 'trait', name, pub, methods, span: this.sp(start) };
  }

  private parseFn(start: Pos, pub: boolean): A.FnItem {
    this.eat('async');
    this.expect('fn');
    const name = this.expectIdent();
    const generics = this.parseGenerics();
    this.expect('(');
    const params: A.Param[] = [];
    while (!this.is(')') && !this.atEof()) {
      params.push(this.parseParam());
      if (!this.eat(',')) break;
    }
    this.expect(')');
    let ret: A.TypeRef | null = null;
    if (this.eat('->')) ret = this.parseType();
    this.parseWhere();
    let body: A.Block | null = null;
    if (this.is('{')) body = this.parseBlock();
    else this.eat(';'); // signature only (trait)
    return { kind: 'fn', name, pub, generics, params, ret, body, span: this.sp(start) };
  }

  private parseParam(): A.Param {
    this.skipAttributes();
    let byRef = false;
    let mut = false;
    // &self / &mut self / self / mut name: T
    if (this.is('&')) {
      byRef = true;
      this.advance();
      if (this.cur().kind === 'lifetime') this.advance();
      if (this.eat('mut')) mut = true;
    }
    if (this.isKw('self')) {
      this.advance();
      return { name: 'self', ty: null, isSelf: true, byRef, mut };
    }
    if (this.eat('mut')) mut = true;
    const name = this.expectIdent();
    let ty: A.TypeRef | null = null;
    if (this.eat(':')) ty = this.parseType();
    return { name, ty, isSelf: false, byRef, mut };
  }

  private parseConst(start: Pos, pub: boolean): A.ConstItem {
    const isStatic = this.isKw('static');
    this.advance(); // const/static
    this.eat('mut');
    const name = this.expectIdent();
    let ty: A.TypeRef | null = null;
    if (this.eat(':')) ty = this.parseType();
    this.expect('=');
    const value = this.parseExpr();
    this.eat(';');
    return { kind: 'const', name, pub, ty, value, isStatic, span: this.sp(start) };
  }

  private parseMod(start: Pos, pub: boolean): A.ModItem {
    this.expect('mod');
    const name = this.expectIdent();
    const items: A.Item[] = [];
    if (this.eat('{')) {
      this.skipSemis();
      while (!this.is('}') && !this.atEof()) {
        items.push(this.parseItem());
        this.skipSemis();
      }
      this.expect('}');
    } else {
      this.eat(';');
    }
    return { kind: 'mod', name, pub, items, span: this.sp(start) };
  }

  // ── types ─────────────────────────────────────────────────────────────────
  private parseType(): A.TypeRef {
    const start = this.startPos();
    let ref = false;
    let mut = false;
    if (this.is('&')) {
      ref = true;
      this.advance();
      if (this.cur().kind === 'lifetime') this.advance();
      if (this.eat('mut')) mut = true;
    }
    this.eat('dyn');
    // tuple / unit type
    if (this.is('(')) {
      this.advance();
      const args: A.TypeRef[] = [];
      while (!this.is(')') && !this.atEof()) {
        args.push(this.parseType());
        if (!this.eat(',')) break;
      }
      this.expect(')');
      return { kind: 'type', name: 'tuple', args, ref, mut, span: this.sp(start) };
    }
    // slice/array type
    if (this.is('[')) {
      this.advance();
      const el = this.parseType();
      if (this.eat(';')) this.parseExpr();
      this.expect(']');
      return { kind: 'type', name: 'array', args: [el], ref, mut, span: this.sp(start) };
    }
    // fn pointer
    if (this.isKw('fn')) {
      this.advance();
      if (this.is('(')) this.skipBalanced('(', ')');
      if (this.eat('->')) this.parseType();
      return { kind: 'type', name: 'fn', args: [], ref, mut, span: this.sp(start) };
    }
    // impl Trait / closures Fn(...)
    let name = this.expectIdent();
    while (this.eat('::')) name = this.expectIdent();
    const args: A.TypeRef[] = [];
    if (this.is('<')) {
      this.advance();
      while (!this.isGt() && !this.atEof()) {
        if (this.cur().kind === 'lifetime') { this.advance(); }
        else {
          // associated type binding: Item = T
          const save = this.p;
          if (this.cur().kind === 'ident' && this.next().kind === 'op' && this.next().value === '=') {
            this.advance(); this.advance(); this.parseType();
          } else {
            this.p = save;
            args.push(this.parseType());
          }
        }
        if (!this.eat(',')) break;
      }
      this.expectGt();
    } else if (this.is('(')) {
      // Fn(A, B) -> C
      this.skipBalanced('(', ')');
      if (this.eat('->')) this.parseType();
    }
    return { kind: 'type', name, args, ref, mut, span: this.sp(start) };
  }

  // ── blocks & statements ────────────────────────────────────────────────────
  private parseBlock(): A.Block {
    const start = this.startPos();
    this.expect('{');
    const stmts: A.Stmt[] = [];
    this.skipSemis();
    while (!this.is('}') && !this.atEof()) {
      stmts.push(this.parseStmt());
      this.skipSemis();
    }
    this.expect('}');
    return { kind: 'block', stmts, span: this.sp(start) };
  }

  private parseStmt(): A.Stmt {
    const start = this.startPos();
    if (this.isKw('let')) {
      this.advance();
      const pat = this.parsePattern();
      let ty: A.TypeRef | null = null;
      if (this.eat(':')) ty = this.parseType();
      let init: A.Expr | null = null;
      if (this.eat('=')) init = this.parseExpr();
      // let-else
      if (this.isKw('else')) { this.parseBlock(); }
      this.eat(';');
      return { kind: 'let', pat, ty, init, span: this.sp(start) };
    }
    // nested items
    if (this.isItemStart()) {
      const item = this.parseItem();
      return { kind: 'itemStmt', item, span: this.sp(start) };
    }
    const expr = this.parseExpr();
    const semi = this.eat(';');
    return { kind: 'exprStmt', expr, semi, span: this.sp(start) };
  }

  private isItemStart(): boolean {
    if (this.isKw('fn') || this.isKw('struct') || this.isKw('enum') ||
        this.isKw('impl') || this.isKw('use') || this.isKw('mod') ||
        this.isKw('trait') || this.isKw('static')) return true;
    if (this.isKw('const')) return this.next().kind === 'ident'; // const FOO, not const{} expr
    if (this.isKw('pub')) return true;
    return false;
  }

  // ── patterns ────────────────────────────────────────────────────────────────
  private parsePattern(): A.Pattern {
    const first = this.parsePatternPrimary();
    if (this.is('|')) {
      const alts = [first];
      const start = first.span.start;
      while (this.eat('|')) alts.push(this.parsePatternPrimary());
      return { kind: 'orPat', alts, span: this.sp(start) };
    }
    return first;
  }

  // Whether the current token can start the upper bound of a range pattern
  // (a literal, or a leading `-` for a negative literal) — used to
  // distinguish `10..100 => ...` (has an upper bound) from `10.. => ...`
  // (open-ended, not exercised by the fixtures but handled for completeness).
  private rangePatHasBound(): boolean {
    const t = this.cur();
    return t.kind === 'int' || t.kind === 'float' || t.kind === 'string' || t.kind === 'char' || t.kind === 'bool' || (t.kind === 'op' && t.value === '-');
  }

  private parsePatternPrimary(): A.Pattern {
    const start = this.startPos();
    if (this.is('&')) { this.advance(); this.eat('mut'); return { kind: 'refPat', inner: this.parsePatternPrimary(), span: this.sp(start) }; }
    if (this.is('_')) { this.advance(); return { kind: 'wildPat', span: this.sp(start) }; }
    if (this.is('(')) {
      this.advance();
      const elems: A.Pattern[] = [];
      while (!this.is(')') && !this.atEof()) {
        elems.push(this.parsePattern());
        if (!this.eat(',')) break;
      }
      this.expect(')');
      return { kind: 'tuplePat', elems, span: this.sp(start) };
    }
    // open-low range pattern: `..=hi` / `..hi` (e.g. `..=69`)
    if (this.is('..') || this.is('..=')) {
      const inclusive = this.cur().value === '..=';
      this.advance();
      const hi = this.rangePatHasBound() ? this.parseUnary() : null;
      return { kind: 'rangePat', lo: null, hi, inclusive, span: this.sp(start) };
    }
    // literals in patterns
    const t = this.cur();
    if (t.kind === 'int' || t.kind === 'float' || t.kind === 'string' || t.kind === 'char' || t.kind === 'bool' || (t.kind === 'op' && t.value === '-')) {
      const value = this.parseUnary();
      // range pattern: `lo..hi` (exclusive) / `lo..=hi` (inclusive)
      if (this.is('..') || this.is('..=')) {
        const inclusive = this.cur().value === '..=';
        this.advance();
        const hi = this.rangePatHasBound() ? this.parseUnary() : null;
        return { kind: 'rangePat', lo: value, hi, inclusive, span: this.sp(start) };
      }
      return { kind: 'litPat', value, span: this.sp(start) };
    }
    if (this.isKw('mut')) { this.advance(); const name = this.expectIdent(); return { kind: 'bindPat', name, mut: true, span: this.sp(start) }; }
    // path / enum / struct pattern
    const path: string[] = [this.expectIdent()];
    while (this.eat('::')) path.push(this.expectIdent());
    if (this.is('(')) {
      this.advance();
      const elems: A.Pattern[] = [];
      while (!this.is(')') && !this.atEof()) {
        elems.push(this.parsePattern());
        if (!this.eat(',')) break;
      }
      this.expect(')');
      return { kind: 'enumPat', path, elems, span: this.sp(start) };
    }
    if (this.is('{')) {
      this.advance();
      const fields: { name: string; pat: A.Pattern }[] = [];
      let rest = false;
      while (!this.is('}') && !this.atEof()) {
        if (this.eat('..')) { rest = true; break; }
        const fname = this.expectIdent();
        let pat: A.Pattern;
        if (this.eat(':')) pat = this.parsePattern();
        else pat = { kind: 'bindPat', name: fname, mut: false, span: this.cur().span };
        fields.push({ name: fname, pat });
        if (!this.eat(',')) break;
      }
      this.expect('}');
      return { kind: 'structPat', path, fields, rest, span: this.sp(start) };
    }
    // single lowercase identifier => binding; Path::Variant or Uppercase => path pattern
    if (path.length === 1 && /^[a-z_]/.test(path[0])) {
      return { kind: 'bindPat', name: path[0], mut: false, span: this.sp(start) };
    }
    return { kind: 'pathPat', path, span: this.sp(start) };
  }

  // ── expressions (Pratt) ──────────────────────────────────────────────────────
  parseExpr(): A.Expr {
    return this.parseAssign();
  }

  private parseAssign(): A.Expr {
    const start = this.startPos();
    const left = this.parseRange();
    const t = this.cur();
    if (t.kind === 'op' && ASSIGN_OPS.has(t.value)) {
      this.advance();
      const value = this.parseAssign();
      return { kind: 'assign', op: t.value, target: left, value, span: this.sp(start) };
    }
    return left;
  }

  private parseRange(): A.Expr {
    const start = this.startPos();
    if (this.is('..') || this.is('..=')) {
      const inclusive = this.cur().value === '..=';
      this.advance();
      const to = this.isExprStart() ? this.parseBinary(0) : null;
      return { kind: 'range', from: null, to, inclusive, span: this.sp(start) };
    }
    const left = this.parseBinary(0);
    if (this.is('..') || this.is('..=')) {
      const inclusive = this.cur().value === '..=';
      this.advance();
      const to = this.isExprStart() ? this.parseBinary(0) : null;
      return { kind: 'range', from: left, to, inclusive, span: this.sp(start) };
    }
    return left;
  }

  private parseBinary(minPrec: number): A.Expr {
    let left = this.parseCast();
    for (;;) {
      const t = this.cur();
      if (t.kind !== 'op') break;
      const prec = BIN_PREC[t.value];
      if (prec === undefined || prec < minPrec) break;
      this.advance();
      const right = this.parseBinary(prec + 1);
      left = { kind: 'binary', op: t.value, left, right, span: this.sp(left.span.start) };
    }
    return left;
  }

  private parseCast(): A.Expr {
    let e = this.parseUnary();
    while (this.isKw('as')) {
      this.advance();
      const ty = this.parseType();
      e = { kind: 'cast', expr: e, ty, span: this.sp(e.span.start) };
    }
    return e;
  }

  private parseUnary(): A.Expr {
    const start = this.startPos();
    if (this.is('-') || this.is('!')) {
      const op = this.advance().value;
      return { kind: 'unary', op, operand: this.parseUnary(), span: this.sp(start) };
    }
    if (this.is('*')) { this.advance(); return { kind: 'deref', expr: this.parseUnary(), span: this.sp(start) }; }
    if (this.is('&')) {
      this.advance();
      const mut = this.eat('mut');
      return { kind: 'ref', mut, expr: this.parseUnary(), span: this.sp(start) };
    }
    return this.parsePostfix();
  }

  private parsePostfix(): A.Expr {
    let e = this.parsePrimary();
    for (;;) {
      if (this.is('.')) {
        this.advance();
        // tuple index .0
        if (this.cur().kind === 'int') {
          const idx = this.advance();
          e = { kind: 'field', obj: e, name: idx.value, span: this.sp(e.span.start) };
          continue;
        }
        if (this.isKw('await')) { this.advance(); continue; }
        const name = this.expectIdent();
        // turbofish on method
        if (this.is('::')) { this.advance(); this.expect('<'); this.skipTypeArgs(); }
        if (this.is('(')) {
          const args = this.parseArgs();
          e = { kind: 'method', recv: e, name, args, span: this.sp(e.span.start) };
        } else {
          e = { kind: 'field', obj: e, name, span: this.sp(e.span.start) };
        }
      } else if (this.is('(')) {
        const args = this.parseArgs();
        e = { kind: 'call', callee: e, args, span: this.sp(e.span.start) };
      } else if (this.is('[')) {
        this.advance();
        const index = this.parseExpr();
        this.expect(']');
        e = { kind: 'index', obj: e, index, span: this.sp(e.span.start) };
      } else if (this.is('?')) {
        this.advance();
        e = { kind: 'try', expr: e, span: this.sp(e.span.start) };
      } else {
        break;
      }
    }
    return e;
  }

  private skipTypeArgs(): void {
    let depth = 1;
    while (depth > 0 && !this.atEof()) {
      if (this.is('<')) depth++;
      else if (this.is('>')) depth--;
      else if (this.is('>>')) depth -= 2;
      this.advance();
    }
  }

  private parseArgs(): A.Expr[] {
    this.expect('(');
    const args: A.Expr[] = [];
    while (!this.is(')') && !this.atEof()) {
      args.push(this.parseExpr());
      if (!this.eat(',')) break;
    }
    this.expect(')');
    return args;
  }

  private isExprStart(): boolean {
    const t = this.cur();
    if (t.kind === 'int' || t.kind === 'float' || t.kind === 'string' || t.kind === 'char' || t.kind === 'bool' || t.kind === 'ident') return true;
    if (t.kind === 'keyword') return ['self', 'Self', 'if', 'match', 'while', 'for', 'loop', 'return', 'break', 'continue', 'move'].includes(t.value);
    if (t.kind === 'op') return ['(', '[', '{', '-', '!', '*', '&', '|', '..', '..='].includes(t.value);
    return false;
  }

  private parsePrimary(): A.Expr {
    const start = this.startPos();
    const t = this.cur();

    if (t.kind === 'int') { this.advance(); return { kind: 'int', value: Number(t.value), span: this.sp(start) }; }
    if (t.kind === 'float') { this.advance(); return { kind: 'float', value: Number(t.value), span: this.sp(start) }; }
    if (t.kind === 'string') { this.advance(); return { kind: 'str', value: t.value, span: this.sp(start) }; }
    if (t.kind === 'char') { this.advance(); return { kind: 'char', value: t.value, span: this.sp(start) }; }
    if (t.kind === 'bool') { this.advance(); return { kind: 'bool', value: t.value === 'true', span: this.sp(start) }; }

    if (this.isKw('if')) return this.parseIf();
    if (this.isKw('match')) return this.parseMatch();
    // labeled loop: `'outer: for ... / while ... / loop ...`
    if (this.cur().kind === 'lifetime' && this.next().kind === 'op' && this.next().value === ':') {
      const label = this.advance().value; // 'outer -> "outer"
      this.expect(':');
      if (this.isKw('while')) return this.parseWhile(label);
      if (this.isKw('for')) return this.parseFor(label);
      if (this.isKw('loop')) { this.advance(); const body = this.parseBlock(); return { kind: 'loop', body, label, span: this.sp(start) }; }
      this.err(`loop label must be followed by 'while', 'for', or 'loop' but found '${this.cur().raw || this.cur().kind}'`, this.cur().span);
    }
    if (this.isKw('while')) return this.parseWhile(null);
    if (this.isKw('for')) return this.parseFor(null);
    if (this.isKw('loop')) { this.advance(); const body = this.parseBlock(); return { kind: 'loop', body, label: null, span: this.sp(start) }; }
    if (this.is('{')) { const block = this.parseBlock(); return { kind: 'blockExpr', block, span: this.sp(start) }; }
    if (this.isKw('return')) { this.advance(); const value = this.isExprStart() ? this.parseExpr() : null; return { kind: 'return', value, span: this.sp(start) }; }
    if (this.isKw('break')) {
      this.advance();
      const label = this.cur().kind === 'lifetime' ? this.advance().value : null;
      const value = this.isExprStart() ? this.parseExpr() : null;
      return { kind: 'break', value, label, span: this.sp(start) };
    }
    if (this.isKw('continue')) {
      this.advance();
      const label = this.cur().kind === 'lifetime' ? this.advance().value : null;
      return { kind: 'continue', label, span: this.sp(start) };
    }
    if (this.isKw('move')) { this.advance(); return this.parseClosure(start); }

    // closure |args| body
    if (this.is('|') || this.is('||')) return this.parseClosure(start);

    // parenthesised / tuple
    if (this.is('(')) {
      this.advance();
      if (this.is(')')) { this.advance(); return { kind: 'tuple', elems: [], span: this.sp(start) }; }
      const first = this.parseExpr();
      if (this.is(',')) {
        const elems = [first];
        while (this.eat(',')) {
          if (this.is(')')) break;
          elems.push(this.parseExpr());
        }
        this.expect(')');
        return { kind: 'tuple', elems, span: this.sp(start) };
      }
      this.expect(')');
      return first;
    }

    // array literal
    if (this.is('[')) {
      this.advance();
      const elems: A.Expr[] = [];
      let repeat: A.Expr | null = null;
      if (!this.is(']')) {
        const firstEl = this.parseExpr();
        if (this.eat(';')) {
          repeat = this.parseExpr();
        } else {
          elems.push(firstEl);
          while (this.eat(',')) {
            if (this.is(']')) break;
            elems.push(this.parseExpr());
          }
        }
      }
      this.expect(']');
      return { kind: 'array', elems, repeat, span: this.sp(start) };
    }

    // identifier / path / macro / struct literal
    if (t.kind === 'ident' || (t.kind === 'keyword' && (t.value === 'self' || t.value === 'Self'))) {
      const segments: string[] = [this.advance().value];
      // macro call: name!
      if (this.is('!')) {
        this.advance();
        return this.parseMacro(segments[0], start);
      }
      while (this.is('::')) {
        this.advance();
        if (this.is('<')) { this.advance(); this.skipTypeArgs(); continue; } // turbofish
        segments.push(this.expectIdent());
      }
      // struct literal: Path { field: val }  (only when a struct literal is legal here)
      if (this.is('{') && this.structLitAllowed) {
        return this.parseStructLit(segments, start);
      }
      return { kind: 'path', segments, span: this.sp(start) };
    }

    this.err(`expected an expression but found '${t.raw || t.kind}'`, t.span);
  }

  // struct literals are disallowed directly in `if`/`while`/`for`/`match` heads
  private structLitAllowed = true;

  private parseNoStruct<T>(fn: () => T): T {
    const save = this.structLitAllowed;
    this.structLitAllowed = false;
    try { return fn(); } finally { this.structLitAllowed = save; }
  }

  private parseStructLit(path: string[], start: Pos): A.Expr {
    this.expect('{');
    const fields: { name: string; value: A.Expr }[] = [];
    let spread: A.Expr | null = null;
    while (!this.is('}') && !this.atEof()) {
      if (this.eat('..')) { spread = this.parseExpr(); break; }
      const name = this.expectIdent();
      let value: A.Expr;
      if (this.eat(':')) value = this.parseExpr();
      else value = { kind: 'path', segments: [name], span: this.cur().span }; // shorthand
      fields.push({ name, value });
      if (!this.eat(',')) break;
    }
    this.expect('}');
    return { kind: 'structLit', path, fields, spread, span: this.sp(start) };
  }

  private parseMacro(name: string, start: Pos): A.Expr {
    // supports name!( ... ) / name![ ... ] / name!{ ... }
    let open = '(', close = ')';
    if (this.is('[')) { open = '['; close = ']'; }
    else if (this.is('{')) { open = '{'; close = '}'; }
    this.expect(open);
    const args: A.Expr[] = [];
    let repeat: A.Expr | null = null;
    // format-style macros: first arg often a string, then exprs.
    // Also supports the repeat form vec![elem; count].
    if (!this.is(close)) {
      args.push(this.parseExpr());
      if (this.eat(';')) {
        repeat = this.parseExpr();
      } else {
        while (this.eat(',')) {
          if (this.is(close)) break;
          args.push(this.parseExpr());
        }
      }
    }
    this.expect(close);
    return { kind: 'macro', name, args, raw: '', repeat, span: this.sp(start) };
  }

  private parseClosure(start: Pos): A.Expr {
    const params: A.Param[] = [];
    if (this.eat('||')) {
      // no params
    } else {
      this.expect('|');
      while (!this.is('|') && !this.atEof()) {
        const mut = this.eat('mut');
        const name = this.expectIdent();
        let ty: A.TypeRef | null = null;
        if (this.eat(':')) ty = this.parseType();
        params.push({ name, ty, isSelf: false, byRef: false, mut });
        if (!this.eat(',')) break;
      }
      this.expect('|');
    }
    if (this.eat('->')) this.parseType();
    const body = this.is('{') ? ({ kind: 'blockExpr', block: this.parseBlock(), span: this.sp(start) } as A.Expr) : this.parseExpr();
    return { kind: 'closure', params, body, span: this.sp(start) };
  }

  private parseIf(): A.Expr {
    const start = this.startPos();
    this.expect('if');
    let letPat: A.Pattern | null = null;
    let cond: A.Expr;
    if (this.eat('let')) {
      letPat = this.parsePattern();
      this.expect('=');
      cond = this.parseNoStruct(() => this.parseExpr());
    } else {
      cond = this.parseNoStruct(() => this.parseExpr());
    }
    const then = this.parseBlock();
    let els: A.Expr | A.Block | null = null;
    if (this.eat('else')) {
      if (this.isKw('if')) els = this.parseIf();
      else els = this.parseBlock();
    }
    return { kind: 'if', cond, then, else: els, letPat, span: this.sp(start) };
  }

  private parseMatch(): A.Expr {
    const start = this.startPos();
    this.expect('match');
    const scrut = this.parseNoStruct(() => this.parseExpr());
    this.expect('{');
    const arms: A.MatchArm[] = [];
    while (!this.is('}') && !this.atEof()) {
      const pat = this.parsePattern();
      let guard: A.Expr | null = null;
      if (this.eat('if')) guard = this.parseExpr();
      this.expect('=>');
      const body = this.parseExpr();
      arms.push({ pat, guard, body });
      this.eat(',');
    }
    this.expect('}');
    return { kind: 'match', scrut, arms, span: this.sp(start) };
  }

  private parseWhile(label: string | null): A.Expr {
    const start = this.startPos();
    this.expect('while');
    let letPat: A.Pattern | null = null;
    let cond: A.Expr;
    if (this.eat('let')) {
      letPat = this.parsePattern();
      this.expect('=');
      cond = this.parseNoStruct(() => this.parseExpr());
    } else {
      cond = this.parseNoStruct(() => this.parseExpr());
    }
    const body = this.parseBlock();
    return { kind: 'while', cond, body, letPat, label, span: this.sp(start) };
  }

  private parseFor(label: string | null): A.Expr {
    const start = this.startPos();
    this.expect('for');
    const pat = this.parsePattern();
    this.expect('in');
    const iter = this.parseNoStruct(() => this.parseExpr());
    const body = this.parseBlock();
    return { kind: 'for', pat, iter, body, label, span: this.sp(start) };
  }
}

export function parse(src: string, file: string): A.Program {
  return new Parser(src, file).parseProgram();
}
