// Builtin standard library — the runtime surface every Titan program can rely
// on without importing: Vec, HashMap, HashSet, String, Option, Result, plus
// numeric/char helpers. Method calls fall through here when no user `impl`
// method matches; associated fns (Vec::new, ...) come through staticBuiltin.

import type { Span } from './diagnostics.ts';
import type { Value } from './values.ts';
import {
  UNIT, mkInt, mkFloat, mkBool, mkStr, mkVec,
  some, NONE, ok, err, valueKey, valueEq, display, debug,
} from './values.ts';
import type { Interpreter } from './interpreter.ts';

type Intr = Interpreter;

function rt(intr: Intr, msg: string, span: Span): never {
  // reuse the interpreter's error path
  (intr as unknown as { rt: (m: string, s: Span) => never }).rt(msg, span);
  throw new Error(msg);
}

// Associated (static) functions: Type::fn
export function staticBuiltin(type: string, member: string): ((args: Value[], intr: Intr) => Value) | null {
  const key = `${type}::${member}`;
  switch (key) {
    case 'Vec::new': case 'Vec::with_capacity': return () => mkVec([]);
    case 'Vec::from': return (a) => (a[0]?.t === 'vec' ? { t: 'vec', items: a[0].items.slice() } : mkVec(a));
    case 'HashMap::new': case 'HashMap::with_capacity': case 'BTreeMap::new': return () => ({ t: 'map', entries: new Map() });
    case 'HashSet::new': case 'BTreeSet::new': return () => ({ t: 'set', items: new Map() });
    case 'String::new': return () => mkStr('');
    case 'String::from': return (a) => mkStr(a[0] ? display(a[0]) : '');
    case 'String::with_capacity': return () => mkStr('');
    case 'Box::new': case 'Rc::new': case 'Arc::new': case 'RefCell::new': case 'Cell::new': case 'Mutex::new':
      return (a) => a[0] ?? UNIT;
    case 'Some': return (a) => some(a[0]);
    case 'Option::Some': return (a) => some(a[0]);
    case 'Ok': case 'Result::Ok': return (a) => ok(a[0]);
    case 'Err': case 'Result::Err': return (a) => err(a[0]);
  }
  // numeric parse/const helpers: i32::MAX etc. handled as calls only
  if (member === 'from' || member === 'new') {
    // generic fallback constructors for unknown wrapper types: identity/first arg
    return (a) => a[0] ?? UNIT;
  }
  return null;
}

export function isEnumCtorPath(segs: string[]): boolean {
  const last = segs[segs.length - 1];
  return last === 'Some' || last === 'None' || last === 'Ok' || last === 'Err';
}

// Instance methods. Returns undefined when no builtin applies (so the
// interpreter can raise a good "no method" error).
export function callBuiltinMethod(recv: Value, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  // Universal methods
  switch (name) {
    case 'clone': return deepClone(recv);
    case 'to_string': return mkStr(display(recv));
    case 'eq': return mkBool(valueEq(recv, args[0]));
    case 'ne': return mkBool(!valueEq(recv, args[0]));
  }

  switch (recv.t) {
    case 'vec': return vecMethod(recv, name, args, intr, span);
    case 'map': return mapMethod(recv, name, args, intr, span);
    case 'set': return setMethod(recv, name, args, intr, span);
    case 'str': return strMethod(recv, name, args, intr, span);
    case 'int': case 'float': return numMethod(recv, name, args, intr, span);
    case 'char': return charMethod(recv, name, args, intr, span);
    case 'bool': return boolMethod(recv, name, args);
    case 'enum': return enumMethod(recv, name, args, intr, span);
    case 'range': return rangeMethod(recv, name, args, intr, span);
    case 'tuple': return undefined;
    default: return undefined;
  }
}

function deepClone(v: Value): Value {
  switch (v.t) {
    case 'vec': return { t: 'vec', items: v.items.map(deepClone) };
    case 'tuple': return { t: 'tuple', items: v.items.map(deepClone) };
    case 'map': { const e = new Map<string, [Value, Value]>(); for (const [k, [kk, vv]] of v.entries) e.set(k, [deepClone(kk), deepClone(vv)]); return { t: 'map', entries: e }; }
    case 'set': { const m = new Map<string, Value>(); for (const [k, vv] of v.items) m.set(k, deepClone(vv)); return { t: 'set', items: m }; }
    case 'struct': { const f = new Map<string, Value>(); for (const [k, vv] of v.fields) f.set(k, deepClone(vv)); return { t: 'struct', name: v.name, fields: f }; }
    case 'enum': return { t: 'enum', enumName: v.enumName, variant: v.variant, payload: v.payload.map(deepClone) };
    default: return v;
  }
}

function vecMethod(recv: Extract<Value, { t: 'vec' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const items = recv.items;
  switch (name) {
    case 'push': items.push(args[0]); return UNIT;
    case 'pop': return items.length ? some(items.pop()!) : NONE;
    case 'len': return mkInt(items.length);
    case 'is_empty': return mkBool(items.length === 0);
    case 'get': { const i = intInt(args[0]); return i >= 0 && i < items.length ? some(items[i]) : NONE; }
    case 'first': return items.length ? some(items[0]) : NONE;
    case 'last': return items.length ? some(items[items.length - 1]) : NONE;
    case 'contains': return mkBool(items.some((x) => valueEq(x, args[0])));
    case 'clear': items.length = 0; return UNIT;
    case 'insert': items.splice(intInt(args[0]), 0, args[1]); return UNIT;
    case 'remove': { const i = intInt(args[0]); return items.splice(i, 1)[0] ?? UNIT; }
    case 'reverse': items.reverse(); return UNIT;
    case 'sort': items.sort((a, b) => cmp(a, b)); return UNIT;
    case 'sort_by': items.sort((a, b) => orderingToNum(intr.apply(args[0], [a, b], span))); return UNIT;
    case 'dedup': { for (let i = items.length - 1; i > 0; i--) if (valueEq(items[i], items[i - 1])) items.splice(i, 1); return UNIT; }
    case 'extend': case 'append': { const other = args[0]; if (other.t === 'vec') { for (const x of other.items) items.push(x); if (name === 'append') other.items.length = 0; } return UNIT; }
    case 'truncate': items.length = Math.min(items.length, intInt(args[0])); return UNIT;
    case 'swap': { const i = intInt(args[0]), j = intInt(args[1]); const t = items[i]; items[i] = items[j]; items[j] = t; return UNIT; }
    case 'iter': case 'into_iter': case 'iter_mut': case 'to_vec': case 'collect': case 'cloned': case 'as_slice':
      return { t: 'vec', items: items.slice() };
    case 'map': return { t: 'vec', items: items.map((x) => intr.apply(args[0], [x], span)) };
    case 'filter': return { t: 'vec', items: items.filter((x) => truthy(intr.apply(args[0], [x], span))) };
    case 'filter_map': { const out: Value[] = []; for (const x of items) { const r = intr.apply(args[0], [x], span); if (r.t === 'enum' && r.variant === 'Some') out.push(r.payload[0]); } return mkVec(out); }
    case 'for_each': { for (const x of items) intr.apply(args[0], [x], span); return UNIT; }
    case 'any': return mkBool(items.some((x) => truthy(intr.apply(args[0], [x], span))));
    case 'all': return mkBool(items.every((x) => truthy(intr.apply(args[0], [x], span))));
    case 'find': { for (const x of items) if (truthy(intr.apply(args[0], [x], span))) return some(x); return NONE; }
    case 'position': { for (let i = 0; i < items.length; i++) if (truthy(intr.apply(args[0], [items[i]], span))) return some(mkInt(i)); return NONE; }
    case 'count': return mkInt(items.length);
    case 'sum': { let s = 0; let f = false; for (const x of items) { if (x.t === 'float') f = true; if (x.t === 'int' || x.t === 'float') s += x.v; } return f ? mkFloat(s) : mkInt(s); }
    case 'product': { let s = 1; for (const x of items) if (x.t === 'int' || x.t === 'float') s *= x.v; return mkInt(s); }
    case 'min': { if (!items.length) return NONE; let m = items[0]; for (const x of items) if (cmp(x, m) < 0) m = x; return some(m); }
    case 'max': { if (!items.length) return NONE; let m = items[0]; for (const x of items) if (cmp(x, m) > 0) m = x; return some(m); }
    case 'fold': case 'reduce': { let acc = args[0]; for (const x of items) acc = intr.apply(args[1], [acc, x], span); return acc; }
    case 'enumerate': return { t: 'vec', items: items.map((x, i) => ({ t: 'tuple', items: [mkInt(i), x] } as Value)) };
    case 'rev': return { t: 'vec', items: items.slice().reverse() };
    case 'take': return { t: 'vec', items: items.slice(0, intInt(args[0])) };
    case 'skip': return { t: 'vec', items: items.slice(intInt(args[0])) };
    case 'zip': { const other = args[0]; const o = other.t === 'vec' ? other.items : []; const n = Math.min(items.length, o.length); const out: Value[] = []; for (let i = 0; i < n; i++) out.push({ t: 'tuple', items: [items[i], o[i]] }); return mkVec(out); }
    case 'join': { const sep = args[0]?.t === 'str' ? args[0].v : ''; return mkStr(items.map(display).join(sep)); }
    case 'chain': { const other = args[0]; const o = other.t === 'vec' ? other.items : []; return mkVec(items.concat(o)); }
    case 'sort_unstable': items.sort((a, b) => cmp(a, b)); return UNIT;
    case 'retain': { const keep = items.filter((x) => truthy(intr.apply(args[0], [x], span))); items.length = 0; items.push(...keep); return UNIT; }
    default: return undefined;
  }
}

function mapMethod(recv: Extract<Value, { t: 'map' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const m = recv.entries;
  switch (name) {
    case 'insert': { const k = valueKey(args[0]); const prev = m.get(k); m.set(k, [args[0], args[1]]); return prev ? some(prev[1]) : NONE; }
    case 'get': { const hit = m.get(valueKey(args[0])); return hit ? some(hit[1]) : NONE; }
    case 'get_mut': { const hit = m.get(valueKey(args[0])); return hit ? some(hit[1]) : NONE; }
    case 'contains_key': return mkBool(m.has(valueKey(args[0])));
    case 'remove': { const k = valueKey(args[0]); const hit = m.get(k); m.delete(k); return hit ? some(hit[1]) : NONE; }
    case 'len': return mkInt(m.size);
    case 'is_empty': return mkBool(m.size === 0);
    case 'clear': m.clear(); return UNIT;
    case 'keys': return mkVec([...m.values()].map(([k]) => k));
    case 'values': return mkVec([...m.values()].map(([, v]) => v));
    case 'iter': case 'into_iter': return mkVec([...m.values()].map(([k, v]) => ({ t: 'tuple', items: [k, v] } as Value)));
    case 'entry': { const hit = m.get(valueKey(args[0])); return hit ? some(hit[1]) : NONE; }
    case 'get_or': { const hit = m.get(valueKey(args[0])); return hit ? hit[1] : args[1]; }
    default: return undefined;
  }
}

function setMethod(recv: Extract<Value, { t: 'set' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const s = recv.items;
  switch (name) {
    case 'insert': { const k = valueKey(args[0]); const had = s.has(k); s.set(k, args[0]); return mkBool(!had); }
    case 'contains': return mkBool(s.has(valueKey(args[0])));
    case 'remove': { const k = valueKey(args[0]); const had = s.has(k); s.delete(k); return mkBool(had); }
    case 'len': return mkInt(s.size);
    case 'is_empty': return mkBool(s.size === 0);
    case 'clear': s.clear(); return UNIT;
    case 'iter': case 'into_iter': return mkVec([...s.values()]);
    default: return undefined;
  }
}

function strMethod(recv: Extract<Value, { t: 'str' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const s = recv.v;
  switch (name) {
    case 'len': return mkInt(s.length);
    case 'is_empty': return mkBool(s.length === 0);
    case 'push': recv.v += args[0]?.t === 'char' ? args[0].v : display(args[0]); return UNIT;
    case 'push_str': recv.v += display(args[0]); return UNIT;
    case 'to_uppercase': case 'to_ascii_uppercase': return mkStr(s.toUpperCase());
    case 'to_lowercase': case 'to_ascii_lowercase': return mkStr(s.toLowerCase());
    case 'trim': return mkStr(s.trim());
    case 'trim_start': return mkStr(s.replace(/^\s+/, ''));
    case 'trim_end': return mkStr(s.replace(/\s+$/, ''));
    case 'contains': return mkBool(s.includes(display(args[0])));
    case 'starts_with': return mkBool(s.startsWith(display(args[0])));
    case 'ends_with': return mkBool(s.endsWith(display(args[0])));
    case 'replace': return mkStr(s.split(display(args[0])).join(display(args[1])));
    case 'split': { const sep = display(args[0]); return mkVec(s.split(sep).map(mkStr)); }
    case 'split_whitespace': return mkVec(s.split(/\s+/).filter((x) => x.length).map(mkStr));
    case 'lines': return mkVec(s.split(/\r?\n/).map(mkStr));
    case 'chars': return mkVec([...s].map((c) => ({ t: 'char', v: c } as Value)));
    case 'bytes': return mkVec([...s].map((c) => mkInt(c.charCodeAt(0))));
    case 'char_at': case 'nth': { const i = intInt(args[0]); return s[i] !== undefined ? some({ t: 'char', v: s[i] }) : NONE; }
    case 'find': { const i = s.indexOf(display(args[0])); return i >= 0 ? some(mkInt(i)) : NONE; }
    case 'repeat': return mkStr(s.repeat(intInt(args[0])));
    case 'as_str': case 'to_string': case 'clone': case 'trim_matches': return mkStr(s);
    case 'parse': { const n = Number(s); return Number.isNaN(n) ? err(mkStr('invalid number')) : ok(s.includes('.') ? mkFloat(n) : mkInt(n)); }
    case 'to_int': { const n = parseInt(s, 10); return Number.isNaN(n) ? NONE : some(mkInt(n)); }
    case 'substring': case 'slice': return mkStr(s.slice(intInt(args[0]), args[1] ? intInt(args[1]) : undefined));
    case 'reverse': return mkStr([...s].reverse().join(''));
    case 'count': return mkInt(s.length);
    default: return undefined;
  }
}

function numMethod(recv: Extract<Value, { t: 'int' | 'float' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const n = recv.v;
  const wrap = recv.t === 'int' ? mkInt : mkFloat;
  switch (name) {
    case 'abs': return wrap(Math.abs(n));
    case 'pow': case 'powi': return wrap(Math.pow(n, num(args[0])));
    case 'powf': return mkFloat(Math.pow(n, num(args[0])));
    case 'sqrt': return mkFloat(Math.sqrt(n));
    case 'min': return wrap(Math.min(n, num(args[0])));
    case 'max': return wrap(Math.max(n, num(args[0])));
    case 'floor': return mkFloat(Math.floor(n));
    case 'ceil': return mkFloat(Math.ceil(n));
    case 'round': return mkFloat(Math.round(n));
    case 'to_string': return mkStr(display(recv));
    case 'is_positive': return mkBool(n > 0);
    case 'is_negative': return mkBool(n < 0);
    case 'is_even': return mkBool(n % 2 === 0);
    case 'is_odd': return mkBool(n % 2 !== 0);
    case 'signum': return wrap(Math.sign(n));
    case 'to_f64': case 'as_f64': return mkFloat(n);
    case 'to_i64': case 'as_i64': case 'trunc': return mkInt(Math.trunc(n));
    case 'checked_add': return some(wrap(n + num(args[0])));
    case 'saturating_sub': return wrap(Math.max(0, n - num(args[0])));
    case 'count_ones': return mkInt(n.toString(2).split('').filter((c) => c === '1').length);
    case 'clone': return recv;
    default: return undefined;
  }
}

function charMethod(recv: Extract<Value, { t: 'char' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const c = recv.v;
  switch (name) {
    case 'is_alphabetic': return mkBool(/[a-zA-Z]/.test(c));
    case 'is_numeric': case 'is_ascii_digit': case 'is_digit': return mkBool(/[0-9]/.test(c));
    case 'is_alphanumeric': return mkBool(/[a-zA-Z0-9]/.test(c));
    case 'is_whitespace': return mkBool(/\s/.test(c));
    case 'is_uppercase': return mkBool(c !== c.toLowerCase());
    case 'is_lowercase': return mkBool(c !== c.toUpperCase());
    case 'to_uppercase': case 'to_ascii_uppercase': return { t: 'char', v: c.toUpperCase() };
    case 'to_lowercase': case 'to_ascii_lowercase': return { t: 'char', v: c.toLowerCase() };
    case 'to_digit': { const d = parseInt(c, args[0] ? intInt(args[0]) : 10); return Number.isNaN(d) ? NONE : some(mkInt(d)); }
    case 'as_u32': case 'to_int': return mkInt(c.charCodeAt(0));
    default: return undefined;
  }
}

function boolMethod(recv: Extract<Value, { t: 'bool' }>, name: string, args: Value[]): Value | undefined {
  switch (name) {
    case 'then': return recv.v ? some(args[0]) : NONE;
    case 'to_string': return mkStr(String(recv.v));
    default: return undefined;
  }
}

function enumMethod(recv: Extract<Value, { t: 'enum' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const isOpt = recv.enumName === 'Option';
  const isRes = recv.enumName === 'Result';
  if (isOpt || isRes) {
    const okVariant = isOpt ? 'Some' : 'Ok';
    const isOk = recv.variant === okVariant;
    const inner = recv.payload[0] ?? UNIT;
    switch (name) {
      case 'is_some': return mkBool(recv.variant === 'Some');
      case 'is_none': return mkBool(recv.variant === 'None');
      case 'is_ok': return mkBool(recv.variant === 'Ok');
      case 'is_err': return mkBool(recv.variant === 'Err');
      case 'unwrap':
        if (isOk) return inner;
        rt(intr, `called \`${recv.enumName}::unwrap()\` on a \`${recv.variant}\` value${recv.variant === 'Err' ? ': ' + debug(inner) : ''}`, span);
        return UNIT;
      case 'expect':
        if (isOk) return inner;
        rt(intr, `${display(args[0] ?? mkStr(''))}`, span);
        return UNIT;
      case 'unwrap_or': return isOk ? inner : args[0];
      case 'unwrap_or_else': return isOk ? inner : intr.apply(args[0], isRes ? [inner] : [], span);
      case 'unwrap_or_default': return isOk ? inner : mkInt(0);
      case 'unwrap_err': if (recv.variant === 'Err' || recv.variant === 'None') return inner; rt(intr, `called unwrap_err on ${recv.variant}`, span); return UNIT;
      case 'map': return isOk ? (isOpt ? some(intr.apply(args[0], [inner], span)) : ok(intr.apply(args[0], [inner], span))) : recv;
      case 'map_or': return isOk ? intr.apply(args[1], [inner], span) : args[0];
      case 'and_then': return isOk ? intr.apply(args[0], [inner], span) : recv;
      case 'or_else': return isOk ? recv : intr.apply(args[0], [], span);
      case 'or': return isOk ? recv : args[0];
      case 'ok': return recv.variant === 'Ok' ? some(inner) : recv.variant === 'Some' ? recv : NONE;
      case 'ok_or': return recv.variant === 'Some' ? ok(inner) : err(args[0]);
      case 'filter': return recv.variant === 'Some' && truthy(intr.apply(args[0], [inner], span)) ? recv : NONE;
      case 'take': return recv;
      case 'as_ref': case 'as_mut': case 'cloned': case 'copied': return recv;
      case 'contains': return mkBool(isOk && valueEq(inner, args[0]));
      case 'unwrap_none': return UNIT;
      default: return undefined;
    }
  }
  // generic enum helpers
  switch (name) {
    case 'variant_name': return mkStr(recv.variant);
    default: return undefined;
  }
}

function rangeMethod(recv: Extract<Value, { t: 'range' }>, name: string, args: Value[], intr: Intr, span: Span): Value | undefined {
  const items = intr.iterValues(recv, span);
  switch (name) {
    case 'collect': case 'iter': case 'into_iter': case 'rev': { const v = mkVec(items); if (name === 'rev') (v as Extract<Value, { t: 'vec' }>).items.reverse(); return v; }
    case 'map': return mkVec(items.map((x) => intr.apply(args[0], [x], span)));
    case 'filter': return mkVec(items.filter((x) => truthy(intr.apply(args[0], [x], span))));
    case 'sum': { let s = 0; for (const x of items) if (x.t === 'int') s += x.v; return mkInt(s); }
    case 'count': return mkInt(items.length);
    case 'contains': return mkBool(args[0].t === 'int' && args[0].v >= recv.from && (recv.inclusive ? args[0].v <= recv.to : args[0].v < recv.to));
    case 'len': return mkInt(items.length);
    case 'for_each': { for (const x of items) intr.apply(args[0], [x], span); return UNIT; }
    case 'fold': { let acc = args[0]; for (const x of items) acc = intr.apply(args[1], [acc, x], span); return acc; }
    default: return undefined;
  }
}

// helpers
function truthy(v: Value): boolean { return v.t === 'bool' ? v.v : v.t !== 'unit'; }
function num(v: Value): number { return v.t === 'int' || v.t === 'float' ? v.v : 0; }
function intInt(v: Value): number { return v.t === 'int' ? v.v : Math.trunc(num(v)); }
function orderingToNum(v: Value): number {
  if (v.t === 'int') return v.v;
  if (v.t === 'enum') return v.variant === 'Less' ? -1 : v.variant === 'Greater' ? 1 : 0;
  return 0;
}
function cmp(a: Value, b: Value): number {
  if ((a.t === 'int' || a.t === 'float') && (b.t === 'int' || b.t === 'float')) return a.v - b.v;
  if (a.t === 'str' && b.t === 'str') return a.v < b.v ? -1 : a.v > b.v ? 1 : 0;
  if (a.t === 'char' && b.t === 'char') return a.v < b.v ? -1 : a.v > b.v ? 1 : 0;
  return 0;
}
