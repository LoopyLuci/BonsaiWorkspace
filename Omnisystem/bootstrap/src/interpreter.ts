// Titan tree-walking interpreter — the bootstrap Omni runtime.
//
// Registers items (structs/enums/impls/fns/consts), then evaluates. Control
// flow (return/break/continue and the `?` operator) uses thrown signals.
// Method calls dispatch to user `impl` methods first, then to builtins.

import type * as A from './ast.ts';
import type { Span } from './diagnostics.ts';
import { OmniError } from './diagnostics.ts';
import type { Value } from './values.ts';
import {
  Env, UNIT, mkInt, mkFloat, mkBool, mkStr, mkVec,
  some, NONE, ok, err, isTruthy, valueKey, valueEq, display, debug,
} from './values.ts';
import { callBuiltinMethod, staticBuiltin, isEnumCtorPath } from './builtins.ts';

// ── control-flow signals ─────────────────────────────────────────────────────
class ReturnSignal { value: Value; constructor(value: Value) { this.value = value; } }
// `label` is null for unlabeled break/continue (always targets the innermost
// loop) or the target label's name (e.g. "outer" for `break 'outer`) — a
// loop only swallows a labeled signal that names it, and rethrows otherwise
// so it keeps unwinding through intermediate loop frames.
class BreakSignal { value: Value; label: string | null; constructor(value: Value, label: string | null) { this.value = value; this.label = label; } }
class ContinueSignal { label: string | null; constructor(label: string | null) { this.label = label; } }

export interface RunResult {
  exitCode: number;
  stdout: string;
}

export class Interpreter {
  file: string;
  source: string;
  globals: Env;
  structs = new Map<string, A.StructItem>();
  enums = new Map<string, A.EnumItem>();
  // methods[typeName][methodName] = FnItem
  methods = new Map<string, Map<string, A.FnItem>>();
  fns = new Map<string, A.FnItem>();
  enumVariants = new Map<string, string>(); // variantName -> enumName (for bare ctors)
  assocConsts = new Map<string, Value>();   // "Type::NAME" -> value
  out: string[] = [];
  // The declared error type name (`E` in `Result<_, E>`) of the innermost
  // function call currently executing, if its return type declares one.
  // Consulted by `?` to auto-convert a mismatched error type via `From`.
  errTypeStack: (string | null)[] = [];

  constructor(file: string, source: string) {
    this.file = file;
    this.source = source;
    this.globals = new Env();
  }

  private rt(msg: string, span: Span, help: string | null = null): never {
    throw new OmniError('runtime', msg, span, this.file, this.source, help);
  }

  print(s: string): void { this.out.push(s); }

  // ── registration ───────────────────────────────────────────────────────────
  register(prog: A.Program): void {
    for (const item of prog.items) this.registerItem(item);
  }

  private registerItem(item: A.Item): void {
    switch (item.kind) {
      case 'struct': this.structs.set(item.name, item); break;
      case 'enum':
        this.enums.set(item.name, item);
        for (const v of item.variants) this.enumVariants.set(v.name, item.name);
        break;
      case 'fn': this.fns.set(item.name, item); break;
      case 'impl': {
        let table = this.methods.get(item.target);
        if (!table) { table = new Map(); this.methods.set(item.target, table); }
        for (const m of item.methods) table.set(m.name, m);
        for (const c of item.consts) {
          this.assocConsts.set(`${item.target}::${c.name}`, this.eval(c.value, this.globals));
        }
        break;
      }
      case 'const': {
        const v = this.eval(item.value, this.globals);
        this.globals.set(item.name, v);
        break;
      }
      case 'mod':
        for (const it of item.items) this.registerItem(it);
        break;
      case 'use': case 'trait': break;
    }
  }

  // ── entry point ──────────────────────────────────────────────────────────────
  runMain(): RunResult {
    const main = this.fns.get('main');
    if (!main) this.rt("no `main` function found", { start: { offset: 0, line: 1, col: 1 }, end: { offset: 0, line: 1, col: 1 } }, "add `pub fn main() { ... }`");
    let exit = 0;
    try {
      const r = this.callFn(main, [], null);
      if (r.t === 'int') exit = r.v;
      else if (r.t === 'enum' && r.enumName === 'Result') exit = r.variant === 'Ok' ? 0 : 1;
    } catch (e) {
      if (e instanceof ReturnSignal) {
        if (e.value.t === 'int') exit = e.value.v;
      } else { throw e; }
    }
    return { exitCode: exit, stdout: this.out.join('') };
  }

  // ── function / method invocation ──────────────────────────────────────────────
  callFn(fn: A.FnItem, args: Value[], selfVal: Value | null): Value {
    const env = this.globals.child();
    let ai = 0;
    for (const p of fn.params) {
      if (p.isSelf) { if (selfVal) env.set('self', selfVal); continue; }
      env.set(p.name, args[ai++] ?? UNIT);
    }
    if (!fn.body) this.rt(`function \`${fn.name}\` has no body`, fn.span);
    // `Result<_, E>` return type declares the error type `?` should convert
    // propagated errors into (via `From`) for the duration of this call.
    const errTy = (fn.ret && fn.ret.name === 'Result' && fn.ret.args[1]) ? fn.ret.args[1].name : null;
    this.errTypeStack.push(errTy);
    try {
      return this.evalBlock(fn.body, env);
    } catch (e) {
      if (e instanceof ReturnSignal) return e.value;
      throw e;
    } finally {
      this.errTypeStack.pop();
    }
  }

  callClosure(cl: Extract<Value, { t: 'closure' }>, args: Value[]): Value {
    const env = cl.env.child();
    cl.decl.params.forEach((p, i) => env.set(p.name, args[i] ?? UNIT));
    try {
      return this.eval(cl.decl.body, env);
    } catch (e) {
      if (e instanceof ReturnSignal) return e.value;
      throw e;
    }
  }

  /** Called by builtins that receive a function-like Value and want to apply it. */
  apply(f: Value, args: Value[], span: Span): Value {
    if (f.t === 'closure') return this.callClosure(f, args);
    if (f.t === 'fn') return this.callFn(f.decl, args, f.selfVal);
    if (f.t === 'builtin') return f.call(args, this);
    this.rt(`value is not callable: ${display(f)}`, span);
  }

  // ── statements / blocks ────────────────────────────────────────────────────────
  private evalBlock(block: A.Block, parent: Env): Value {
    const env = parent.child();
    // hoist nested items
    for (const s of block.stmts) if (s.kind === 'itemStmt') this.registerItem(s.item);
    let last: Value = UNIT;
    for (let i = 0; i < block.stmts.length; i++) {
      const s = block.stmts[i];
      if (s.kind === 'let') {
        const v = s.init ? this.eval(s.init, env) : UNIT;
        this.bindPattern(s.pat, v, env, true);
        last = UNIT;
      } else if (s.kind === 'exprStmt') {
        last = this.eval(s.expr, env);
        if (s.semi) last = UNIT;
      } else {
        last = UNIT; // item
      }
    }
    return last;
  }

  // ── expression evaluation ──────────────────────────────────────────────────────
  eval(e: A.Expr, env: Env): Value {
    switch (e.kind) {
      case 'int': return mkInt(e.value);
      case 'float': return mkFloat(e.value);
      case 'str': return mkStr(e.value);
      case 'char': return { t: 'char', v: e.value };
      case 'bool': return mkBool(e.value);
      case 'path': return this.evalPath(e, env);
      case 'field': return this.evalField(e, env);
      case 'index': return this.evalIndex(e, env);
      case 'call': return this.evalCall(e, env);
      case 'method': return this.evalMethod(e, env);
      case 'unary': return this.evalUnary(e, env);
      case 'binary': return this.evalBinary(e, env);
      case 'assign': return this.evalAssign(e, env);
      case 'range': return this.evalRange(e, env);
      case 'if': return this.evalIf(e, env);
      case 'match': return this.evalMatch(e, env);
      case 'while': return this.evalWhile(e, env);
      case 'for': return this.evalFor(e, env);
      case 'loop': return this.evalLoop(e, env);
      case 'blockExpr': return this.evalBlock(e.block, env);
      case 'return': throw new ReturnSignal(e.value ? this.eval(e.value, env) : UNIT);
      case 'break': throw new BreakSignal(e.value ? this.eval(e.value, env) : UNIT, e.label);
      case 'continue': throw new ContinueSignal(e.label);
      case 'structLit': return this.evalStructLit(e, env);
      case 'array': return this.evalArray(e, env);
      case 'tuple': return { t: 'tuple', items: e.elems.map((x) => this.eval(x, env)) };
      case 'closure': return { t: 'closure', decl: e, env };
      case 'ref': return this.eval(e.expr, env);
      case 'deref': return this.eval(e.expr, env);
      case 'try': return this.evalTry(e, env);
      case 'cast': return this.evalCast(e, env);
      case 'macro': return this.evalMacro(e, env);
    }
  }

  private evalPath(e: A.PathExpr, env: Env): Value {
    const segs = e.segments;
    if (segs.length === 1) {
      const name = segs[0];
      const local = env.get(name);
      if (local !== undefined) return local;
      if (name === 'None') return NONE;
      if (this.fns.has(name)) return { t: 'fn', decl: this.fns.get(name)!, selfVal: null };
      if (this.enumVariants.has(name)) {
        const en = this.enumVariants.get(name)!;
        return { t: 'enum', enumName: en, variant: name, payload: [] };
      }
      this.rt(`unknown name \`${name}\``, e.span);
    }
    // Type::member
    const [type, member] = [segs[0], segs[segs.length - 1]];
    // enum variant constructor with no payload, e.g. Color::Red / Option::None
    const en = this.enums.get(type);
    if (en) {
      const variant = en.variants.find((v) => v.name === member);
      if (variant) return { t: 'enum', enumName: type, variant: member, payload: [] };
    }
    if (type === 'Option' && member === 'None') return NONE;
    // associated constant Type::NAME
    const ac = this.assocConsts.get(`${type}::${member}`);
    if (ac !== undefined) return ac;
    // static impl method or associated fn -> return a bound fn value
    const table = this.methods.get(type);
    if (table && table.has(member)) return { t: 'fn', decl: table.get(member)!, selfVal: null };
    // builtin associated fn (Vec::new, ...) becomes callable placeholder
    const b = staticBuiltin(type, member);
    if (b) return { t: 'builtin', name: `${type}::${member}`, call: b as never };
    // constant path fallback
    const local = env.get(member);
    if (local !== undefined) return local;
    this.rt(`unknown path \`${segs.join('::')}\``, e.span);
  }

  private evalField(e: A.FieldExpr, env: Env): Value {
    const obj = this.eval(e.obj, env);
    if (obj.t === 'struct') {
      if (obj.fields.has(e.name)) return obj.fields.get(e.name)!;
      this.rt(`struct \`${obj.name}\` has no field \`${e.name}\``, e.span);
    }
    if (obj.t === 'tuple') {
      const idx = Number(e.name);
      if (idx < obj.items.length) return obj.items[idx];
      this.rt(`tuple index ${idx} out of range`, e.span);
    }
    this.rt(`cannot access field \`${e.name}\` on ${obj.t}`, e.span);
  }

  private evalIndex(e: A.IndexExpr, env: Env): Value {
    const obj = this.eval(e.obj, env);
    const idx = this.eval(e.index, env);
    if (obj.t === 'vec') {
      if (idx.t !== 'int') this.rt('index must be an integer', e.span);
      if (idx.v < 0 || idx.v >= obj.items.length) this.rt(`index out of bounds: len is ${obj.items.length} but index is ${idx.v}`, e.span);
      return obj.items[idx.v];
    }
    if (obj.t === 'map') {
      const hit = obj.entries.get(valueKey(idx));
      if (hit) return hit[1];
      this.rt('key not found', e.span);
    }
    if (obj.t === 'str') {
      if (idx.t === 'int') return { t: 'char', v: obj.v[idx.v] ?? '' };
    }
    this.rt(`cannot index ${obj.t}`, e.span);
  }

  private evalCall(e: A.CallExpr, env: Env): Value {
    // enum tuple-variant constructor: Some(x), Ok(v), Color::Rgb(...)
    if (e.callee.kind === 'path') {
      const segs = e.callee.segments;
      const args = e.args.map((a) => this.eval(a, env));
      const ctor = this.tryEnumCtor(segs, args);
      if (ctor) return ctor;
      // Type::method static call or free fn
      const callee = this.evalPathCallable(segs, e.callee.span, env);
      if (callee) return this.apply(callee, args, e.span);
      this.rt(`unknown function \`${segs.join('::')}\``, e.span);
    }
    const callee = this.eval(e.callee, env);
    const args = e.args.map((a) => this.eval(a, env));
    return this.apply(callee, args, e.span);
  }

  private tryEnumCtor(segs: string[], args: Value[]): Value | null {
    const last = segs[segs.length - 1];
    if (segs.length === 1) {
      if (last === 'Some') return some(args[0]);
      if (last === 'Ok') return ok(args[0]);
      if (last === 'Err') return err(args[0]);
      if (this.enumVariants.has(last)) {
        return { t: 'enum', enumName: this.enumVariants.get(last)!, variant: last, payload: args };
      }
      return null;
    }
    const type = segs[0];
    const en = this.enums.get(type);
    if (en && en.variants.some((v) => v.name === last)) {
      return { t: 'enum', enumName: type, variant: last, payload: args };
    }
    if (type === 'Option' && last === 'Some') return some(args[0]);
    if (type === 'Result' && last === 'Ok') return ok(args[0]);
    if (type === 'Result' && last === 'Err') return err(args[0]);
    return null;
  }

  private evalPathCallable(segs: string[], span: Span, env: Env): Value | null {
    if (segs.length === 1) {
      const local = env.get(segs[0]);
      if (local && (local.t === 'fn' || local.t === 'closure' || local.t === 'builtin')) return local;
      if (this.fns.has(segs[0])) return { t: 'fn', decl: this.fns.get(segs[0])!, selfVal: null };
      return null;
    }
    const type = segs[0], member = segs[segs.length - 1];
    const table = this.methods.get(type);
    if (table && table.has(member)) return { t: 'fn', decl: table.get(member)!, selfVal: null };
    const b = staticBuiltin(type, member);
    if (b) return { t: 'builtin', name: `${type}::${member}`, call: b as never };
    return null;
  }

  private evalMethod(e: A.MethodCallExpr, env: Env): Value {
    const recv = this.eval(e.recv, env);
    const args = e.args.map((a) => this.eval(a, env));
    // user-defined method on the receiver's type
    const typeName = this.typeNameOf(recv);
    if (typeName) {
      const table = this.methods.get(typeName);
      if (table && table.has(e.name)) {
        const r = this.callFn(table.get(e.name)!, args, recv);
        this.writeBackSelf(e.recv, recv, env);
        return r;
      }
    }
    // builtin method
    const b = callBuiltinMethod(recv, e.name, args, this, e.span);
    if (b !== undefined) {
      this.writeBackSelf(e.recv, recv, env);
      return b;
    }
    this.rt(`no method \`${e.name}\` on ${this.describe(recv)}`, e.span,
      `available: define \`fn ${e.name}(&self, ...)\` in an impl block, or use a supported builtin`);
  }

  // For &mut self semantics on mutable containers/structs we mutate in place;
  // for path receivers we also reassign so value-type updates are visible.
  private writeBackSelf(recvExpr: A.Expr, recv: Value, env: Env): void {
    if (recvExpr.kind === 'path' && recvExpr.segments.length === 1) {
      env.assign(recvExpr.segments[0], recv);
    }
  }

  typeNameOf(v: Value): string | null {
    if (v.t === 'struct') return v.name;
    if (v.t === 'enum') return v.enumName;
    if (v.t === 'vec') return 'Vec';
    if (v.t === 'map') return 'HashMap';
    if (v.t === 'set') return 'HashSet';
    if (v.t === 'str') return 'String';
    return null;
  }

  private describe(v: Value): string {
    const n = this.typeNameOf(v);
    return n ? `\`${n}\`` : `${v.t} value`;
  }

  private evalUnary(e: A.UnaryExpr, env: Env): Value {
    const v = this.eval(e.operand, env);
    if (e.op === '-') {
      if (v.t === 'int') return mkInt(-v.v);
      if (v.t === 'float') return mkFloat(-v.v);
    }
    if (e.op === '!') {
      if (v.t === 'bool') return mkBool(!v.v);
      if (v.t === 'int') return mkInt(~v.v);
    }
    this.rt(`cannot apply unary \`${e.op}\` to ${v.t}`, e.span);
  }

  private evalBinary(e: A.BinaryExpr, env: Env): Value {
    if (e.op === '&&') return mkBool(isTruthy(this.eval(e.left, env)) && isTruthy(this.eval(e.right, env)));
    if (e.op === '||') return mkBool(isTruthy(this.eval(e.left, env)) || isTruthy(this.eval(e.right, env)));
    const l = this.eval(e.left, env);
    const r = this.eval(e.right, env);
    return this.binop(e.op, l, r, e.span);
  }

  private binop(op: string, l: Value, r: Value, span: Span): Value {
    if (op === '==') return mkBool(valueEq(l, r));
    if (op === '!=') return mkBool(!valueEq(l, r));
    // string concatenation with +
    if (op === '+' && (l.t === 'str' || r.t === 'str')) return mkStr(display(l) + display(r));
    if ((l.t === 'int' || l.t === 'float') && (r.t === 'int' || r.t === 'float')) {
      const a = l.v as number, b = r.v as number;
      const bothInt = l.t === 'int' && r.t === 'int';
      const num = (x: number) => (bothInt ? mkInt(x) : mkFloat(x));
      switch (op) {
        case '+': return num(a + b);
        case '-': return num(a - b);
        case '*': return num(a * b);
        case '/':
          if (b === 0 && bothInt) this.rt('attempt to divide by zero', span);
          return bothInt ? mkInt(Math.trunc(a / b)) : mkFloat(a / b);
        case '%':
          if (b === 0 && bothInt) this.rt('attempt to calculate remainder with a divisor of zero', span);
          return num(a % b);
        case '<': return mkBool(a < b);
        case '>': return mkBool(a > b);
        case '<=': return mkBool(a <= b);
        case '>=': return mkBool(a >= b);
        case '&': return mkInt(a & b);
        case '|': return mkInt(a | b);
        case '^': return mkInt(a ^ b);
        case '<<': return mkInt(a << b);
        case '>>': return mkInt(a >> b);
      }
    }
    if (l.t === 'bool' && r.t === 'bool') {
      if (op === '&') return mkBool(l.v && r.v);
      if (op === '|') return mkBool(l.v || r.v);
      if (op === '^') return mkBool(l.v !== r.v);
    }
    if ((op === '<' || op === '>' || op === '<=' || op === '>=') && l.t === 'str' && r.t === 'str') {
      const c = l.v < r.v ? -1 : l.v > r.v ? 1 : 0;
      return mkBool(op === '<' ? c < 0 : op === '>' ? c > 0 : op === '<=' ? c <= 0 : c >= 0);
    }
    this.rt(`cannot apply \`${op}\` to ${l.t} and ${r.t}`, span);
  }

  private evalAssign(e: A.AssignExpr, env: Env): Value {
    let value = this.eval(e.value, env);
    if (e.op !== '=') {
      const cur = this.eval(e.target, env);
      value = this.binop(e.op.slice(0, -1), cur, value, e.span);
    }
    this.assignTo(e.target, value, env);
    return UNIT;
  }

  private assignTo(target: A.Expr, value: Value, env: Env): void {
    if (target.kind === 'path' && target.segments.length === 1) {
      if (!env.assign(target.segments[0], value)) env.set(target.segments[0], value);
      return;
    }
    if (target.kind === 'field') {
      const obj = this.eval(target.obj, env);
      if (obj.t === 'struct') { obj.fields.set(target.name, value); return; }
      if (obj.t === 'tuple') { obj.items[Number(target.name)] = value; return; }
      this.rt('invalid assignment target', target.span);
    }
    if (target.kind === 'index') {
      const obj = this.eval(target.obj, env);
      const idx = this.eval(target.index, env);
      if (obj.t === 'vec' && idx.t === 'int') { obj.items[idx.v] = value; return; }
      if (obj.t === 'map') { obj.entries.set(valueKey(idx), [idx, value]); return; }
      this.rt('invalid index assignment', target.span);
    }
    if (target.kind === 'deref') { this.assignTo(target.expr, value, env); return; }
    this.rt('invalid assignment target', target.span);
  }

  private evalRange(e: A.RangeExpr, env: Env): Value {
    const from = e.from ? this.eval(e.from, env) : mkInt(0);
    const to = e.to ? this.eval(e.to, env) : mkInt(0);
    if (from.t !== 'int' || to.t !== 'int') this.rt('range bounds must be integers', e.span);
    return { t: 'range', from: from.v, to: to.v, inclusive: e.inclusive };
  }

  private evalIf(e: A.IfExpr, env: Env): Value {
    if (e.letPat) {
      const v = this.eval(e.cond, env);
      const scope = env.child();
      if (this.matchPattern(e.letPat, v, scope)) return this.evalBlock(e.then, scope);
      if (e.else) return this.eval(e.else as A.Expr, env);
      return UNIT;
    }
    if (isTruthy(this.eval(e.cond, env))) return this.evalBlock(e.then, env);
    if (e.else) {
      if ((e.else as A.Block).kind === 'block') return this.evalBlock(e.else as A.Block, env);
      return this.eval(e.else as A.Expr, env);
    }
    return UNIT;
  }

  private evalMatch(e: A.MatchExpr, env: Env): Value {
    const v = this.eval(e.scrut, env);
    for (const arm of e.arms) {
      const scope = env.child();
      if (this.matchPattern(arm.pat, v, scope)) {
        if (arm.guard && !isTruthy(this.eval(arm.guard, scope))) continue;
        return this.eval(arm.body, scope);
      }
    }
    this.rt(`no match arm covered value ${debug(v)}`, e.span, 'add a catch-all `_ => ...` arm');
  }

  // A break/continue with no label always targets the innermost loop; one
  // with a label only stops here if it names this loop, otherwise it must
  // keep propagating outward to the enclosing loop that owns that label.
  private labelMatches(sigLabel: string | null, myLabel: string | null): boolean {
    return sigLabel === null || sigLabel === myLabel;
  }

  private evalWhile(e: A.WhileExpr, env: Env): Value {
    for (;;) {
      if (e.letPat) {
        const v = this.eval(e.cond, env);
        const scope = env.child();
        if (!this.matchPattern(e.letPat, v, scope)) break;
        try { this.evalBlock(e.body, scope); }
        catch (sig) {
          if (sig instanceof BreakSignal) { if (this.labelMatches(sig.label, e.label)) break; throw sig; }
          if (sig instanceof ContinueSignal) { if (this.labelMatches(sig.label, e.label)) continue; throw sig; }
          throw sig;
        }
      } else {
        if (!isTruthy(this.eval(e.cond, env))) break;
        try { this.evalBlock(e.body, env); }
        catch (sig) {
          if (sig instanceof BreakSignal) { if (this.labelMatches(sig.label, e.label)) break; throw sig; }
          if (sig instanceof ContinueSignal) { if (this.labelMatches(sig.label, e.label)) continue; throw sig; }
          throw sig;
        }
      }
    }
    return UNIT;
  }

  private evalFor(e: A.ForExpr, env: Env): Value {
    const iter = this.eval(e.iter, env);
    const items = this.iterValues(iter, e.span);
    for (const it of items) {
      const scope = env.child();
      this.bindPattern(e.pat, it, scope, true);
      try { this.evalBlock(e.body, scope); }
      catch (sig) {
        if (sig instanceof BreakSignal) { if (this.labelMatches(sig.label, e.label)) break; throw sig; }
        if (sig instanceof ContinueSignal) { if (this.labelMatches(sig.label, e.label)) continue; throw sig; }
        throw sig;
      }
    }
    return UNIT;
  }

  iterValues(v: Value, span: Span): Value[] {
    if (v.t === 'range') {
      const out: Value[] = [];
      const end = v.inclusive ? v.to : v.to - 1;
      for (let i = v.from; i <= end; i++) out.push(mkInt(i));
      return out;
    }
    if (v.t === 'vec') return v.items.slice();
    if (v.t === 'set') return [...v.items.values()];
    if (v.t === 'map') return [...v.entries.values()].map(([k, val]) => ({ t: 'tuple', items: [k, val] } as Value));
    if (v.t === 'str') return [...v.v].map((c) => ({ t: 'char', v: c } as Value));
    if (v.t === 'enum' && v.enumName === 'Option') return v.variant === 'Some' ? [v.payload[0]] : [];
    this.rt(`\`${v.t}\` is not iterable`, span);
  }

  private evalLoop(e: A.LoopExpr, env: Env): Value {
    for (;;) {
      try { this.evalBlock(e.body, env); }
      catch (sig) {
        if (sig instanceof BreakSignal) { if (this.labelMatches(sig.label, e.label)) return sig.value; throw sig; }
        if (sig instanceof ContinueSignal) { if (this.labelMatches(sig.label, e.label)) continue; throw sig; }
        throw sig;
      }
    }
  }

  private evalStructLit(e: A.StructLitExpr, env: Env): Value {
    const name = e.path[e.path.length - 1];
    // enum struct-variant literal e.g. Shape::Circle { r: 1.0 }
    if (e.path.length > 1 && this.enums.has(e.path[0])) {
      const payload = e.fields.map((f) => this.eval(f.value, env));
      return { t: 'enum', enumName: e.path[0], variant: name, payload };
    }
    const fields = new Map<string, Value>();
    if (e.spread) {
      const base = this.eval(e.spread, env);
      if (base.t === 'struct') for (const [k, val] of base.fields) fields.set(k, val);
    }
    for (const f of e.fields) fields.set(f.name, this.eval(f.value, env));
    return { t: 'struct', name, fields };
  }

  private evalArray(e: A.ArrayLit, env: Env): Value {
    if (e.repeat) {
      const val = this.eval(e.elems[0] ?? { kind: 'int', value: 0, span: e.span } as A.Expr, env);
      const n = this.eval(e.repeat, env);
      const count = n.t === 'int' ? n.v : 0;
      return mkVec(Array.from({ length: count }, () => val));
    }
    return mkVec(e.elems.map((x) => this.eval(x, env)));
  }

  private evalTry(e: A.TryExpr, env: Env): Value {
    const v = this.eval(e.expr, env);
    if (v.t === 'enum' && v.enumName === 'Result') {
      if (v.variant === 'Ok') return v.payload[0];
      throw new ReturnSignal(this.convertPropagatedErr(v.payload[0])); // propagate Err, converted via From if needed
    }
    if (v.t === 'enum' && v.enumName === 'Option') {
      if (v.variant === 'Some') return v.payload[0];
      throw new ReturnSignal(NONE);
    }
    this.rt('the `?` operator can only be applied to Result or Option', e.span);
  }

  // Implements `?`'s Rust-idiomatic error conversion: if the enclosing
  // function declares `-> Result<_, E>` and the propagated error's type
  // differs from `E`, look for `impl From<SourceErr> for E { fn from(..) }`
  // and apply it — matching Rust's automatic `?` + `From` conversion so
  // callers don't need `.map_err(...)` at every function boundary.
  //
  // Limitation: the method table is keyed by (target type, method name)
  // only, not by source-parameter type, so if `E` has more than one `impl
  // From<X> for E` the most-recently-registered `from` wins rather than
  // dispatching on the actual source type. Fine for the common one-`From`
  // case; multi-source `From` would need a richer method table.
  private convertPropagatedErr(errVal: Value): Value {
    const targetTy = this.errTypeStack[this.errTypeStack.length - 1];
    if (!targetTy) return err(errVal);
    if (this.typeNameOf(errVal) === targetTy) return err(errVal);
    const fromFn = this.methods.get(targetTy)?.get('from');
    if (!fromFn) return err(errVal); // no conversion registered — propagate unchanged
    try {
      return err(this.callFn(fromFn, [errVal], null));
    } catch {
      return err(errVal);
    }
  }

  private evalCast(e: A.CastExpr, env: Env): Value {
    const v = this.eval(e.expr, env);
    const target = e.ty.name;
    if (v.t === 'int' || v.t === 'float' || v.t === 'char' || v.t === 'bool') {
      const n = v.t === 'char' ? v.v.charCodeAt(0) : v.t === 'bool' ? (v.v ? 1 : 0) : v.v;
      if (target.startsWith('f')) return mkFloat(n);
      if (target === 'char') return { t: 'char', v: String.fromCharCode(n) };
      if (target === 'bool') return mkBool(n !== 0);
      return mkInt(n);
    }
    return v;
  }

  private evalMacro(e: A.MacroExpr, env: Env): Value {
    const args = e.args;
    switch (e.name) {
      case 'println': { this.print(this.formatMacro(args, env) + '\n'); return UNIT; }
      case 'print': { this.print(this.formatMacro(args, env)); return UNIT; }
      case 'eprintln': { process.stderr.write(this.formatMacro(args, env) + '\n'); return UNIT; }
      case 'format': return mkStr(this.formatMacro(args, env));
      case 'vec': {
        if (e.repeat) {
          const el = this.eval(args[0], env);
          const n = this.eval(e.repeat, env);
          const count = n.t === 'int' ? n.v : 0;
          return mkVec(Array.from({ length: count }, () => el));
        }
        if (args.length === 0) return mkVec([]);
        return mkVec(args.map((a) => this.eval(a, env)));
      }
      case 'panic': { const m = this.formatMacro(args, env); this.rt(`panicked: ${m || 'explicit panic'}`, e.span); }
      case 'assert': {
        const cond = this.eval(args[0], env);
        if (!isTruthy(cond)) this.rt(`assertion failed`, e.span);
        return UNIT;
      }
      case 'assert_eq': {
        const a = this.eval(args[0], env), b = this.eval(args[1], env);
        if (!valueEq(a, b)) this.rt(`assertion failed: \`(left == right)\`\n  left: ${debug(a)}\n right: ${debug(b)}`, e.span);
        return UNIT;
      }
      case 'assert_ne': {
        const a = this.eval(args[0], env), b = this.eval(args[1], env);
        if (valueEq(a, b)) this.rt(`assertion failed: \`(left != right)\``, e.span);
        return UNIT;
      }
      case 'write': case 'writeln': {
        // write!(buf, "...") — append to a String receiver if present
        return UNIT;
      }
      case 'todo': case 'unimplemented': this.rt(`not yet implemented`, e.span);
      case 'dbg': { const v = args[0] ? this.eval(args[0], env) : UNIT; this.print(debug(v) + '\n'); return v; }
      default:
        // unknown macro: evaluate args, return unit
        args.forEach((a) => this.eval(a, env));
        return UNIT;
    }
  }

  // format!/println! — supports {}, {:?}, {name}, and positional {0}
  formatMacro(args: A.Expr[], env: Env): string {
    if (args.length === 0) return '';
    const first = args[0];
    if (first.kind !== 'str') {
      // println!(x) with no format string
      return args.map((a) => display(this.eval(a, env))).join(' ');
    }
    const fmt = first.value;
    const rest = args.slice(1).map((a) => this.eval(a, env));
    let ai = 0;
    return fmt.replace(/\{\{|\}\}|\{[^}]*\}/g, (m) => {
      if (m === '{{') return '{';
      if (m === '}}') return '}';
      const spec = m.slice(1, -1);
      const debugFmt = spec.endsWith(':?') || spec === ':?' || /:.*\?/.test(spec);
      const nameOrIdx = spec.split(':')[0];
      let val: Value;
      if (nameOrIdx === '') {
        val = rest[ai++] ?? UNIT;
      } else if (/^\d+$/.test(nameOrIdx)) {
        val = rest[Number(nameOrIdx)] ?? UNIT;
      } else {
        const nv = env.get(nameOrIdx);
        val = nv ?? UNIT;
      }
      return debugFmt ? debug(val) : display(val);
    });
  }

  // ── pattern matching ───────────────────────────────────────────────────────
  bindPattern(pat: A.Pattern, v: Value, env: Env, declare: boolean): void {
    if (!this.matchPattern(pat, v, env)) {
      this.rt(`pattern does not match value ${debug(v)}`, pat.span);
    }
  }

  matchPattern(pat: A.Pattern, v: Value, env: Env): boolean {
    switch (pat.kind) {
      case 'wildPat': return true;
      case 'bindPat': env.set(pat.name, v); return true;
      case 'refPat': return this.matchPattern(pat.inner, v, env);
      case 'litPat': {
        const lit = this.eval(pat.value, env);
        return valueEq(lit, v);
      }
      case 'orPat': return pat.alts.some((a) => this.matchPattern(a, v, env));
      case 'rangePat': {
        // Ordered scalar (int/float/char) range membership test — inclusive
        // (`..=`) includes the upper bound, exclusive (`..`) excludes it; an
        // absent bound (`..=69` or `10..`) means unbounded on that side.
        const x = rangeKey(v);
        if (x === null) return false;
        if (pat.lo) {
          const lo = rangeKey(this.eval(pat.lo, env));
          if (lo === null || x < lo) return false;
        }
        if (pat.hi) {
          const hi = rangeKey(this.eval(pat.hi, env));
          if (hi === null) return false;
          if (pat.inclusive ? x > hi : x >= hi) return false;
        }
        return true;
      }
      case 'tuplePat': {
        if (v.t !== 'tuple' || v.items.length !== pat.elems.length) return false;
        return pat.elems.every((p, i) => this.matchPattern(p, v.items[i], env));
      }
      case 'pathPat': {
        const name = pat.path[pat.path.length - 1];
        if (name === 'None') return v.t === 'enum' && v.variant === 'None';
        if (v.t === 'enum') return v.variant === name;
        // unit struct / constant compare
        const c = env.get(name);
        if (c) return valueEq(c, v);
        return false;
      }
      case 'enumPat': {
        const name = pat.path[pat.path.length - 1];
        if (v.t !== 'enum') return false;
        if (v.variant !== name) return false;
        if (pat.elems.length !== v.payload.length) {
          // allow single binding capturing whole payload tuple
          if (pat.elems.length === 1) return this.matchPattern(pat.elems[0], v.payload[0], env);
          return false;
        }
        return pat.elems.every((p, i) => this.matchPattern(p, v.payload[i], env));
      }
      case 'structPat': {
        if (v.t !== 'struct') return false;
        for (const f of pat.fields) {
          const fv = v.fields.get(f.name);
          if (fv === undefined) return false;
          if (!this.matchPattern(f.pat, fv, env)) return false;
        }
        return true;
      }
    }
  }
}

// Maps int/float/char values to a common numeric ordering key for
// range-pattern membership tests (`1..=9`, `..=69`, char ranges, ...).
// Returns null for non-scalar values, which never match a range pattern.
function rangeKey(v: Value): number | null {
  if (v.t === 'int' || v.t === 'float') return v.v;
  if (v.t === 'char') return v.v.charCodeAt(0);
  return null;
}
