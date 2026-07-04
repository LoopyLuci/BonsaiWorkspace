// OmniCC Universal Language Intermediate Representation (ULIR)
// The backbone of every conversion — all parsers write ULIR, all generators read it.
// Paradigm-agnostic, rich enough for any language construct.

// ─── Core enumerations ────────────────────────────────────────────────────────

export type LanguageFamily =
    | 'c-family'       // C, C++, Java, JS, TS, C#, Go, Kotlin, Swift, Dart, ...
    | 'python-family'  // Python, Ruby, Lua, Perl, PHP, Groovy, Crystal, Nim, ...
    | 'functional'     // Haskell, OCaml, F#, Elm, PureScript, Clojure, Scala, ...
    | 'lisp'           // Common Lisp, Scheme, Racket, Clojure, Fennel, ...
    | 'shell'          // Bash, Zsh, Fish, PowerShell, Batch, Dash, ...
    | 'data'           // SQL, JSON, YAML, TOML, XML, CSV, HCL, ...
    | 'web'            // HTML, CSS, SCSS, React/JSX, Vue, Svelte, Astro, ...
    | 'systems'        // Rust, Zig, Odin, Carbon, D, Assembly, WASM, ...
    | 'ml-scientific'  // R, Julia, MATLAB, Octave, Wolfram, APL, J, K, ...
    | 'concurrent'     // Erlang, Elixir, Pony, Io, Oz, Chapel, ...
    | 'logic'          // Prolog, Datalog, Alloy, MiniZinc, Mercury, ...
    | 'legacy'         // COBOL, Fortran, BASIC, Pascal, Ada, ALGOL, PL/I, ...
    | 'template'       // Jinja, Handlebars, Mustache, Liquid, ERB, ...
    | 'query'          // GraphQL, SPARQL, Cypher, OQL, ...
    | 'omni'           // Titan, Vera, Nexus, Helix, Aether, Axiom, Sylva
    | 'esoteric'       // Brainfuck, Befunge, Whitespace, INTERCAL, ...
    | 'unknown';

export type Paradigm =
    | 'imperative' | 'declarative' | 'functional' | 'oop'
    | 'logic' | 'reactive' | 'concurrent' | 'dataflow' | 'meta'
    | 'array' | 'stack' | 'event-driven' | 'agent'
    | 'scripting' | 'systems' | 'data' | 'unknown';

export type TypeSystem =
    | 'static-strong'    // Java, Rust, Haskell
    | 'static-weak'      // C, C++
    | 'dynamic-strong'   // Python, Ruby
    | 'dynamic-weak'     // JavaScript, PHP (old)
    | 'dynamic-duck'     // Duck-typed: Lua, older Ruby
    | 'gradual'          // TypeScript, Python with hints
    | 'inferred'         // Haskell, OCaml, Rust (mostly)
    | 'dependent'        // Idris, Agda, Coq
    | 'structural'       // Go, TypeScript interfaces
    | 'nominal'          // Java, C#, Kotlin
    | 'none'             // Assembly, COBOL
    | 'unknown';

export type MemoryModel =
    | 'gc'              // Java, Go, C#, Python
    | 'manual'          // C, C++
    | 'ownership'       // Rust
    | 'arc'             // Swift, Objective-C
    | 'stack'           // Most languages for locals
    | 'region'          // Zig, arena allocators
    | 'managed'         // .NET runtime
    | 'none'            // Scripting/declarative
    | 'unknown';

export type Visibility = 'public' | 'private' | 'protected' | 'internal' | 'package' | 'module';
export type ConversionConfidence = 'exact' | 'high' | 'medium' | 'low' | 'partial';

// ─── ULIR Module (top-level compilation unit) ─────────────────────────────────

export interface ULIRModule {
    name: string;
    sourceLanguage: string;
    sourceFamily: LanguageFamily;
    units: ULIRUnit[];
    imports: ULIRImport[];
    exports: string[];
    docComment: string;
    metadata: ULIRMetadata;
    confidence: ConversionConfidence;
    notes: string[];
    widgetUnits?: ULIRUnit[]; // units routed to Widget Bridge
}

export interface ULIRMetadata {
    sourceLines: number;
    paradigms: Paradigm[];
    typeSystem: TypeSystem;
    memoryModel: MemoryModel;
    usesAsync: boolean;
    usesGenerics: boolean;
    usesReflection: boolean;
    usesMetaprogramming: boolean;
    hasTests: boolean;
    hasUI: boolean;          // hints Widget Bridge
    hasSideEffects: boolean;
    entryPoint?: string;     // main function name if detected
}

// ─── Imports ──────────────────────────────────────────────────────────────────

export interface ULIRImport {
    path: string;
    alias?: string;
    names: string[];         // named imports
    isDefault: boolean;
    isWildcard: boolean;
    kind: 'module' | 'package' | 'relative' | 'stdlib' | 'external';
    originalSyntax: string;  // preserved for notes
}

// ─── Semantic Units ───────────────────────────────────────────────────────────

export type ULIRUnitKind =
    // Functions & methods
    | 'function' | 'method' | 'constructor' | 'destructor' | 'accessor'
    | 'lambda' | 'closure' | 'generator' | 'coroutine' | 'macro'
    // Types
    | 'class' | 'struct' | 'interface' | 'trait' | 'mixin' | 'protocol'
    | 'enum' | 'union' | 'tagged-union' | 'record' | 'tuple-type'
    | 'type-alias' | 'newtype' | 'opaque-type'
    // Concurrency
    | 'actor' | 'channel' | 'thread' | 'coroutine-def'
    // Declarations
    | 'variable' | 'constant' | 'field' | 'property' | 'parameter'
    | 'namespace' | 'module-decl' | 'package-decl'
    // Formal
    | 'theorem' | 'proof' | 'axiom-decl' | 'invariant'
    // Tests
    | 'test' | 'benchmark' | 'fixture'
    // UI (Widget Bridge hooks)
    | 'widget-component' | 'widget-layout' | 'widget-style' | 'widget-event'
    // Data constructs (SQL/NoSQL)
    | 'table' | 'view' | 'schema' | 'query-unit';

export interface ULIRUnit {
    kind: ULIRUnitKind;
    name: string;
    visibility: Visibility;
    signature: ULIRSignature;
    body: ULIRStatement[];
    attributes: string[];        // decorators, annotations, attributes
    docComment: string;
    sourceLines: [number, number];
    isAsync: boolean;
    isStatic: boolean;
    isAbstract: boolean;
    isFinal: boolean;
    isOverride: boolean;
    isExtern: boolean;
    generics: ULIRGeneric[];
    extends_: string[];          // base classes/interfaces
    implements_: string[];
    children: ULIRUnit[];        // nested: class methods, module items
    originalSource: string;      // raw source for fallback generation
    sourceLanguage?: string;     // lang ID this unit was parsed from
    confidence: ConversionConfidence;
}

// ─── Type System ──────────────────────────────────────────────────────────────

export interface ULIRType {
    name: string;
    nullable: boolean;
    optional: boolean;
    generic: ULIRType[];         // e.g. List<String> → generic = [String]
    isArray: boolean;
    arrayDims: number;
    isFunction: boolean;
    params?: ULIRType[];         // function type params
    returns?: ULIRType;          // function type return
    isUnion: boolean;
    unionMembers?: ULIRType[];
    isIntersection: boolean;
    intersectionMembers?: ULIRType[];
    isTuple: boolean;
    tupleMembers?: ULIRType[];
    isLiteral: boolean;
    literalValue?: string;
    isPrimitive: boolean;
    isMapped: boolean;           // TypeScript mapped types
    originalSrc: string;         // raw type text
}

export const VOID_TYPE:   ULIRType = { name: 'void',    nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'void' };
export const STRING_TYPE: ULIRType = { name: 'String',  nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'String' };
export const INT_TYPE:    ULIRType = { name: 'Int',     nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'Int' };
export const FLOAT_TYPE:  ULIRType = { name: 'Float',   nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'Float' };
export const BOOL_TYPE:   ULIRType = { name: 'Bool',    nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'Bool' };
export const ANY_TYPE:    ULIRType = { name: 'Any',     nullable: true,  optional: true,  generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: false, isMapped: false, originalSrc: 'any' };
export const UNKNOWN_TYPE: ULIRType = { name: 'Unknown', nullable: true, optional: true,  generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: false, isMapped: false, originalSrc: '?' };

// ─── Signatures ───────────────────────────────────────────────────────────────

export interface ULIRSignature {
    params: ULIRParam[];
    returns: ULIRType;
    throws: ULIRType[];
    selfParam?: string;          // 'self', 'this', '@' etc. from source
}

export interface ULIRParam {
    name: string;
    type: ULIRType;
    defaultValue?: ULIRExpression;
    isVariadic: boolean;
    isKeyword: boolean;          // Python kwargs-style
    isRef: boolean;
    isMut: boolean;
    label?: string;              // Swift external label
}

export interface ULIRGeneric {
    name: string;
    bounds: ULIRType[];
    defaultType?: ULIRType;
    isVariadic: boolean;
}

// ─── Statements ───────────────────────────────────────────────────────────────

export type ULIRStatementKind =
    | 'assign' | 'declare' | 'return' | 'yield' | 'throw' | 'break' | 'continue'
    | 'if' | 'else-if' | 'else' | 'switch' | 'case' | 'match'
    | 'for' | 'for-in' | 'for-of' | 'while' | 'do-while' | 'loop'
    | 'try' | 'catch' | 'finally'
    | 'expression-stmt' | 'block' | 'comment' | 'label' | 'goto'
    | 'defer' | 'with' | 'using' | 'async-stmt' | 'await-stmt'
    | 'assert' | 'print' | 'raw';

export interface ULIRStatement {
    kind: ULIRStatementKind;
    condition?: ULIRExpression;
    target?: ULIRExpression;      // LHS of assignment
    value?: ULIRExpression;       // RHS
    type?: ULIRType;              // for declarations
    isMut?: boolean;
    isConst?: boolean;
    body?: ULIRStatement[];
    elseBody?: ULIRStatement[];
    cases?: ULIRCase[];
    iterVar?: string;
    iterSource?: ULIRExpression;
    catchClauses?: ULIRCatch[];
    finallyBody?: ULIRStatement[];
    raw?: string;                 // fallback: unparsed line
    comment?: string;
    lineNumber?: number;
}

export interface ULIRCase {
    pattern: ULIRExpression | string;
    guard?: ULIRExpression;
    body: ULIRStatement[];
    isDefault: boolean;
}

export interface ULIRCatch {
    exceptionType?: ULIRType;
    binding?: string;
    body: ULIRStatement[];
}

// ─── Expressions ─────────────────────────────────────────────────────────────

export type ULIRExpressionKind =
    | 'literal' | 'identifier' | 'call' | 'method-call' | 'new'
    | 'field-access' | 'index' | 'binary-op' | 'unary-op' | 'ternary'
    | 'cast' | 'instanceof' | 'typeof' | 'await' | 'yield-expr'
    | 'lambda-expr' | 'array-lit' | 'object-lit' | 'tuple-lit'
    | 'template-lit' | 'spread' | 'destructure' | 'match-expr' | 'if-expr'
    | 'range' | 'closure-expr' | 'raw-expr';

export interface ULIRExpression {
    kind: ULIRExpressionKind;
    raw: string;                  // ALWAYS set — fallback for generators
    type?: ULIRType;
    // call
    callee?: string;
    args?: ULIRExpression[];
    // binary
    op?: string;
    left?: ULIRExpression;
    right?: ULIRExpression;
    // literal
    literalKind?: 'int' | 'float' | 'string' | 'bool' | 'null' | 'char' | 'regex';
    literalValue?: string;
    // field
    object?: ULIRExpression;
    field?: string;
    // array/object literal
    elements?: ULIRExpression[];
    entries?: Array<{ key: string; value: ULIRExpression }>;
}

export function rawExpr(raw: string): ULIRExpression {
    return { kind: 'raw-expr', raw };
}

export function litExpr(kind: ULIRExpression['literalKind'], value: string): ULIRExpression {
    return { kind: 'literal', raw: value, literalKind: kind, literalValue: value };
}

// ─── Conversion request / result ──────────────────────────────────────────────

export interface OmniCCConversionRequest {
    source: string;
    sourceLang?: string;       // 'auto' or language id (optional — defaults to auto-detect)
    targetLang: string;
    filename?: string;         // hint for language detection
    widgetNameHint?: string;
    // Project-mode
    projectMode?: boolean;
    projectFiles?: ProjectFile[];
    options?: ConversionOptions;
}

export interface ProjectFile {
    path: string;
    content: string;
    relativePath?: string;
    detectedLang?: string;
}

export interface ConversionOptions {
    preserveComments: boolean;
    preserveFormatting: boolean;
    idiomaticTarget: boolean;    // use target-lang idioms vs literal translation
    includeTests: boolean;
    strictTypes: boolean;        // add type annotations when target is typed
    targetRuntime?: string;      // e.g. 'node', 'browser', 'jvm', 'clr'
    encoding: string;
    lineEndings: 'lf' | 'crlf' | 'cr';
    enableWidgetBridge?: boolean; // route UI patterns to Widget Converter
    mergeWidgetBridge?: boolean;  // inline widget output into main output
}

export const DEFAULT_OPTIONS: ConversionOptions = {
    preserveComments: true,
    preserveFormatting: false,
    idiomaticTarget: true,
    includeTests: false,
    strictTypes: true,
    encoding: 'utf-8',
    lineEndings: 'lf',
};

export interface OmniCCConversionResult {
    // Core output
    success: boolean;
    output: string;                 // generated code (alias: code)
    code?: string;                  // same as output — kept for compatibility
    sourceLanguage: string;
    targetLanguage: string;
    sourceLangId?: string;          // alias for sourceLanguage
    targetLangId?: string;          // alias for targetLanguage
    confidence: number;             // 0–100 numeric confidence
    confidenceLevel?: ConversionConfidence;
    detectionSignals?: string[];
    notes: string[];
    warnings?: string[];
    error?: string;
    ir?: ULIRModule;
    widgetResults?: import('../conversion/WidgetIR').ConversionResult[];
    widgetBridge?: {
        detected: boolean;
        widgetCount: number;
        convertedCount: number;
        confidence: number;
        previewHtml: string;
    };
    fileExtension?: string;
    linesConverted: number;
    linesTotal?: number;
    durationMs: number;
    // Project mode
    projectResults?: ProjectFileResult[];
}

export interface ProjectFileResult {
    // Canonical names
    sourcePath?: string;
    outputPath?: string;
    // Extended names (used by ConversionEngine)
    path?: string;
    targetPath?: string;
    output?: string;
    code?: string;
    sourceLangId?: string;
    targetLangId?: string;
    linesIn?: number;
    linesOut?: number;
    durationMs?: number;
    confidence: number;
    notes?: string[];
    error?: string;
    ir?: ULIRModule;
}

// ─── Language description (for registry) ─────────────────────────────────────

export interface LanguageDef {
    id: string;
    name: string;
    aliases: string[];
    extensions: string[];
    family: LanguageFamily;
    paradigms: Paradigm[];
    typing: TypeSystem;
    memory: MemoryModel;
    year: number;
    popularity: number;   // 1–10
    color: string;        // hex, for UI
    description: string;
    keywords: string[];   // language-unique detection keywords
    shebang?: string;     // e.g. '#!/usr/bin/env python3'
    comment: { line: string; blockStart?: string; blockEnd?: string };
    indentStyle: 'braces' | 'indent' | 'begin-end' | 'none' | 'parentheses' | 'mixed';
    features: string[];
    fileExtensionMap?: Record<string, string>; // for project conversion: .js → .ts etc.
}
