// AST — the Titan abstract syntax tree.
//
// Discriminated unions (not TS enums) so the file is type-strippable and runs
// directly under `node`. Every node carries a Span for diagnostics.

import type { Span } from './diagnostics.ts';

export interface Node {
  span: Span;
}

// ── Types (parsed; interpreted dynamically) ─────────────────────────────────
export interface TypeRef extends Node {
  kind: 'type';
  name: string;           // e.g. "Vec", "i32", "Self", "&mut", "fn"
  args: TypeRef[];        // generic arguments
  ref: boolean;           // & / &mut
  mut: boolean;
}

// ── Program & items ─────────────────────────────────────────────────────────
export type Item = UseItem | StructItem | EnumItem | ImplItem | FnItem | ConstItem | ModItem | TraitItem;

export interface Program {
  items: Item[];
  file: string;
  source: string;
}

export interface UseItem extends Node {
  kind: 'use';
  path: string[];         // ["omnisystem"]
  names: string[];        // ["Vec","HashMap",...]  ([] = glob or single path)
}

export interface Param {
  name: string;
  ty: TypeRef | null;
  isSelf: boolean;
  byRef: boolean;
  mut: boolean;
}

export interface FnItem extends Node {
  kind: 'fn';
  name: string;
  pub: boolean;
  generics: string[];
  params: Param[];
  ret: TypeRef | null;
  body: Block | null;     // null for trait method signatures
}

export interface Field {
  name: string;
  ty: TypeRef | null;
  pub: boolean;
}

export interface StructItem extends Node {
  kind: 'struct';
  name: string;
  pub: boolean;
  generics: string[];
  fields: Field[];        // empty for unit/tuple structs
  tuple: TypeRef[] | null; // tuple-struct element types
}

export interface EnumVariant {
  name: string;
  fields: TypeRef[];      // tuple-style payload
  structFields: Field[] | null;
}

export interface EnumItem extends Node {
  kind: 'enum';
  name: string;
  pub: boolean;
  generics: string[];
  variants: EnumVariant[];
}

export interface ImplItem extends Node {
  kind: 'impl';
  trait: string | null;   // trait being implemented (or null for inherent)
  target: string;         // type name
  generics: string[];
  methods: FnItem[];
  consts: ConstItem[];    // associated constants (Type::NAME)
}

export interface TraitItem extends Node {
  kind: 'trait';
  name: string;
  pub: boolean;
  methods: FnItem[];
}

export interface ConstItem extends Node {
  kind: 'const';
  name: string;
  pub: boolean;
  ty: TypeRef | null;
  value: Expr;
  isStatic: boolean;
}

export interface ModItem extends Node {
  kind: 'mod';
  name: string;
  pub: boolean;
  items: Item[];
}

// ── Statements ──────────────────────────────────────────────────────────────
export type Stmt = LetStmt | ExprStmt | ItemStmt;

export interface LetStmt extends Node {
  kind: 'let';
  pat: Pattern;
  ty: TypeRef | null;
  init: Expr | null;
}

export interface ExprStmt extends Node {
  kind: 'exprStmt';
  expr: Expr;
  semi: boolean;          // had trailing ';' (affects block value)
}

export interface ItemStmt extends Node {
  kind: 'itemStmt';
  item: Item;
}

export interface Block extends Node {
  kind: 'block';
  stmts: Stmt[];
}

// ── Patterns ────────────────────────────────────────────────────────────────
export type Pattern =
  | { kind: 'bindPat'; name: string; mut: boolean; span: Span }
  | { kind: 'wildPat'; span: Span }
  | { kind: 'litPat'; value: Expr; span: Span }
  | { kind: 'tuplePat'; elems: Pattern[]; span: Span }
  | { kind: 'pathPat'; path: string[]; span: Span }        // e.g. Color::Red, None
  | { kind: 'enumPat'; path: string[]; elems: Pattern[]; span: Span } // Some(x), Ok(v)
  | { kind: 'structPat'; path: string[]; fields: { name: string; pat: Pattern }[]; rest: boolean; span: Span }
  | { kind: 'refPat'; inner: Pattern; span: Span }
  | { kind: 'orPat'; alts: Pattern[]; span: Span };

// ── Expressions ─────────────────────────────────────────────────────────────
export type Expr =
  | IntLit | FloatLit | StrLit | CharLit | BoolLit
  | PathExpr | FieldExpr | IndexExpr | CallExpr | MethodCallExpr
  | UnaryExpr | BinaryExpr | AssignExpr | RangeExpr
  | IfExpr | MatchExpr | WhileExpr | ForExpr | LoopExpr | BlockExpr
  | ReturnExpr | BreakExpr | ContinueExpr
  | StructLitExpr | ArrayLit | TupleExpr | ClosureExpr | RefExpr | DerefExpr
  | TryExpr | CastExpr | MacroExpr;

export interface IntLit extends Node { kind: 'int'; value: number; }
export interface FloatLit extends Node { kind: 'float'; value: number; }
export interface StrLit extends Node { kind: 'str'; value: string; }
export interface CharLit extends Node { kind: 'char'; value: string; }
export interface BoolLit extends Node { kind: 'bool'; value: boolean; }

export interface PathExpr extends Node { kind: 'path'; segments: string[]; }
export interface FieldExpr extends Node { kind: 'field'; obj: Expr; name: string; }
export interface IndexExpr extends Node { kind: 'index'; obj: Expr; index: Expr; }
export interface CallExpr extends Node { kind: 'call'; callee: Expr; args: Expr[]; }
export interface MethodCallExpr extends Node { kind: 'method'; recv: Expr; name: string; args: Expr[]; }
export interface UnaryExpr extends Node { kind: 'unary'; op: string; operand: Expr; }
export interface BinaryExpr extends Node { kind: 'binary'; op: string; left: Expr; right: Expr; }
export interface AssignExpr extends Node { kind: 'assign'; op: string; target: Expr; value: Expr; }
export interface RangeExpr extends Node { kind: 'range'; from: Expr | null; to: Expr | null; inclusive: boolean; }
export interface IfExpr extends Node { kind: 'if'; cond: Expr; then: Block; else: Expr | Block | null; letPat: Pattern | null; }
export interface MatchArm { pat: Pattern; guard: Expr | null; body: Expr; }
export interface MatchExpr extends Node { kind: 'match'; scrut: Expr; arms: MatchArm[]; }
export interface WhileExpr extends Node { kind: 'while'; cond: Expr; body: Block; letPat: Pattern | null; }
export interface ForExpr extends Node { kind: 'for'; pat: Pattern; iter: Expr; body: Block; }
export interface LoopExpr extends Node { kind: 'loop'; body: Block; }
export interface BlockExpr extends Node { kind: 'blockExpr'; block: Block; }
export interface ReturnExpr extends Node { kind: 'return'; value: Expr | null; }
export interface BreakExpr extends Node { kind: 'break'; value: Expr | null; }
export interface ContinueExpr extends Node { kind: 'continue'; }
export interface StructLitExpr extends Node { kind: 'structLit'; path: string[]; fields: { name: string; value: Expr }[]; spread: Expr | null; }
export interface ArrayLit extends Node { kind: 'array'; elems: Expr[]; repeat: Expr | null; }
export interface TupleExpr extends Node { kind: 'tuple'; elems: Expr[]; }
export interface ClosureExpr extends Node { kind: 'closure'; params: Param[]; body: Expr; }
export interface RefExpr extends Node { kind: 'ref'; mut: boolean; expr: Expr; }
export interface DerefExpr extends Node { kind: 'deref'; expr: Expr; }
export interface TryExpr extends Node { kind: 'try'; expr: Expr; }
export interface CastExpr extends Node { kind: 'cast'; expr: Expr; ty: TypeRef; }
export interface MacroExpr extends Node { kind: 'macro'; name: string; args: Expr[]; raw: string; repeat: Expr | null; }
