// Runtime value model for the Titan interpreter.
//
// Values are tagged so the interpreter can dispatch methods and format output.
// Integers use JS `number` (f64) for the runnable subset; this is documented as
// a bootstrap limitation (no 64-bit wraparound). Structs/enums are first-class.

import type * as A from './ast.ts';

export type Value =
  | { t: 'int'; v: number }
  | { t: 'float'; v: number }
  | { t: 'bool'; v: boolean }
  | { t: 'str'; v: string }
  | { t: 'char'; v: string }
  | { t: 'unit' }
  | { t: 'tuple'; items: Value[] }
  | { t: 'vec'; items: Value[] }
  | { t: 'map'; entries: Map<string, [Value, Value]> }
  | { t: 'set'; items: Map<string, Value> }
  | { t: 'struct'; name: string; fields: Map<string, Value> }
  | { t: 'enum'; enumName: string; variant: string; payload: Value[] }
  | { t: 'range'; from: number; to: number; inclusive: boolean }
  | { t: 'fn'; decl: A.FnItem; selfVal: Value | null }
  | { t: 'closure'; decl: A.ClosureExpr; env: Env }
  | { t: 'builtin'; name: string; call: (args: Value[], intr: unknown) => Value };

export const UNIT: Value = { t: 'unit' };
export function mkInt(v: number): Value { return { t: 'int', v: Math.trunc(v) }; }
export function mkFloat(v: number): Value { return { t: 'float', v }; }
export function mkBool(v: boolean): Value { return { t: 'bool', v }; }
export function mkStr(v: string): Value { return { t: 'str', v }; }
export function mkVec(items: Value[]): Value { return { t: 'vec', items }; }

// Option / Result are ordinary enums the interpreter special-cases for ergonomics.
export function some(v: Value): Value { return { t: 'enum', enumName: 'Option', variant: 'Some', payload: [v] }; }
export const NONE: Value = { t: 'enum', enumName: 'Option', variant: 'None', payload: [] };
export function ok(v: Value): Value { return { t: 'enum', enumName: 'Result', variant: 'Ok', payload: [v] }; }
export function err(v: Value): Value { return { t: 'enum', enumName: 'Result', variant: 'Err', payload: [v] }; }

export function isTruthy(v: Value): boolean {
  return v.t === 'bool' ? v.v : v.t !== 'unit';
}

// A hashable key for map/set membership.
export function valueKey(v: Value): string {
  switch (v.t) {
    case 'int': case 'float': return `n:${v.v}`;
    case 'bool': return `b:${v.v}`;
    case 'str': return `s:${v.v}`;
    case 'char': return `c:${v.v}`;
    case 'unit': return 'unit';
    case 'tuple': return 't:' + v.items.map(valueKey).join(',');
    case 'vec': return 'v:' + v.items.map(valueKey).join(',');
    case 'enum': return `e:${v.enumName}:${v.variant}:` + v.payload.map(valueKey).join(',');
    case 'struct': return `st:${v.name}:` + [...v.fields.entries()].map(([k, x]) => k + '=' + valueKey(x)).join(',');
    default: return 'obj:' + Math.random();
  }
}

export function valueEq(a: Value, b: Value): boolean {
  return valueKey(a) === valueKey(b);
}

// Display / Debug formatting.
export function display(v: Value): string {
  switch (v.t) {
    case 'int': return String(v.v);
    case 'float': return Number.isInteger(v.v) ? v.v.toFixed(1) : String(v.v);
    case 'bool': return String(v.v);
    case 'str': return v.v;
    case 'char': return v.v;
    case 'unit': return '()';
    case 'tuple': return '(' + v.items.map(display).join(', ') + ')';
    case 'vec': return '[' + v.items.map(debug).join(', ') + ']';
    case 'set': return '{' + [...v.items.values()].map(debug).join(', ') + '}';
    case 'map': return '{' + [...v.entries.values()].map(([k, val]) => `${debug(k)}: ${debug(val)}`).join(', ') + '}';
    case 'range': return `${v.from}..${v.inclusive ? '=' : ''}${v.to}`;
    case 'enum':
      if (v.payload.length === 0) return v.variant;
      return `${v.variant}(${v.payload.map(debug).join(', ')})`;
    case 'struct': {
      const inner = [...v.fields.entries()].map(([k, x]) => `${k}: ${debug(x)}`).join(', ');
      return `${v.name} { ${inner} }`;
    }
    case 'fn': return `fn ${v.decl.name}`;
    case 'closure': return 'closure';
    case 'builtin': return `builtin ${v.name}`;
  }
}

export function debug(v: Value): string {
  if (v.t === 'str') return JSON.stringify(v.v);
  if (v.t === 'char') return `'${v.v}'`;
  return display(v);
}

// ── Environments (lexical scopes) ────────────────────────────────────────────
export class Env {
  vars: Map<string, Value>;
  parent: Env | null;

  constructor(parent: Env | null = null) {
    this.vars = new Map();
    this.parent = parent;
  }

  child(): Env {
    return new Env(this);
  }

  get(name: string): Value | undefined {
    let e: Env | null = this;
    while (e) {
      const v = e.vars.get(name);
      if (v !== undefined) return v;
      e = e.parent;
    }
    return undefined;
  }

  set(name: string, v: Value): void {
    this.vars.set(name, v);
  }

  // assign to an existing binding in the nearest enclosing scope
  assign(name: string, v: Value): boolean {
    let e: Env | null = this;
    while (e) {
      if (e.vars.has(name)) { e.vars.set(name, v); return true; }
      e = e.parent;
    }
    return false;
  }
}
