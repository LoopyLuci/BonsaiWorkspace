export type LanguageFamily = 'c-family' | 'python-family' | 'functional' | 'lisp' | 'shell' | 'data' | 'web' | 'systems' | 'ml-scientific' | 'concurrent' | 'logic' | 'legacy' | 'template' | 'query' | 'omni' | 'esoteric' | 'unknown';
export type Paradigm = 'imperative' | 'declarative' | 'functional' | 'oop' | 'logic' | 'reactive' | 'concurrent' | 'dataflow' | 'meta' | 'array' | 'stack' | 'event-driven' | 'agent' | 'scripting' | 'systems' | 'data' | 'unknown';
export type TypeSystem = 'static-strong' | 'static-weak' | 'dynamic-strong' | 'dynamic-weak' | 'dynamic-duck' | 'gradual' | 'inferred' | 'dependent' | 'structural' | 'nominal' | 'none' | 'unknown';
export type MemoryModel = 'gc' | 'manual' | 'ownership' | 'arc' | 'stack' | 'region' | 'managed' | 'none' | 'unknown';
export type Visibility = 'public' | 'private' | 'protected' | 'internal' | 'package' | 'module';
export type ConversionConfidence = 'exact' | 'high' | 'medium' | 'low' | 'partial';
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
    widgetUnits?: ULIRUnit[];
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
    hasUI: boolean;
    hasSideEffects: boolean;
    entryPoint?: string;
}
export interface ULIRImport {
    path: string;
    alias?: string;
    names: string[];
    isDefault: boolean;
    isWildcard: boolean;
    kind: 'module' | 'package' | 'relative' | 'stdlib' | 'external';
    originalSyntax: string;
}
export type ULIRUnitKind = 'function' | 'method' | 'constructor' | 'destructor' | 'accessor' | 'lambda' | 'closure' | 'generator' | 'coroutine' | 'macro' | 'class' | 'struct' | 'interface' | 'trait' | 'mixin' | 'protocol' | 'enum' | 'union' | 'tagged-union' | 'record' | 'tuple-type' | 'type-alias' | 'newtype' | 'opaque-type' | 'actor' | 'channel' | 'thread' | 'coroutine-def' | 'variable' | 'constant' | 'field' | 'property' | 'parameter' | 'namespace' | 'module-decl' | 'package-decl' | 'theorem' | 'proof' | 'axiom-decl' | 'invariant' | 'test' | 'benchmark' | 'fixture' | 'widget-component' | 'widget-layout' | 'widget-style' | 'widget-event' | 'table' | 'view' | 'schema' | 'query-unit';
export interface ULIRUnit {
    kind: ULIRUnitKind;
    name: string;
    visibility: Visibility;
    signature: ULIRSignature;
    body: ULIRStatement[];
    attributes: string[];
    docComment: string;
    sourceLines: [number, number];
    isAsync: boolean;
    isStatic: boolean;
    isAbstract: boolean;
    isFinal: boolean;
    isOverride: boolean;
    isExtern: boolean;
    generics: ULIRGeneric[];
    extends_: string[];
    implements_: string[];
    children: ULIRUnit[];
    originalSource: string;
    sourceLanguage?: string;
    confidence: ConversionConfidence;
}
export interface ULIRType {
    name: string;
    nullable: boolean;
    optional: boolean;
    generic: ULIRType[];
    isArray: boolean;
    arrayDims: number;
    isFunction: boolean;
    params?: ULIRType[];
    returns?: ULIRType;
    isUnion: boolean;
    unionMembers?: ULIRType[];
    isIntersection: boolean;
    intersectionMembers?: ULIRType[];
    isTuple: boolean;
    tupleMembers?: ULIRType[];
    isLiteral: boolean;
    literalValue?: string;
    isPrimitive: boolean;
    isMapped: boolean;
    originalSrc: string;
}
export declare const VOID_TYPE: ULIRType;
export declare const STRING_TYPE: ULIRType;
export declare const INT_TYPE: ULIRType;
export declare const FLOAT_TYPE: ULIRType;
export declare const BOOL_TYPE: ULIRType;
export declare const ANY_TYPE: ULIRType;
export declare const UNKNOWN_TYPE: ULIRType;
export interface ULIRSignature {
    params: ULIRParam[];
    returns: ULIRType;
    throws: ULIRType[];
    selfParam?: string;
}
export interface ULIRParam {
    name: string;
    type: ULIRType;
    defaultValue?: ULIRExpression;
    isVariadic: boolean;
    isKeyword: boolean;
    isRef: boolean;
    isMut: boolean;
    label?: string;
}
export interface ULIRGeneric {
    name: string;
    bounds: ULIRType[];
    defaultType?: ULIRType;
    isVariadic: boolean;
}
export type ULIRStatementKind = 'assign' | 'declare' | 'return' | 'yield' | 'throw' | 'break' | 'continue' | 'if' | 'else-if' | 'else' | 'switch' | 'case' | 'match' | 'for' | 'for-in' | 'for-of' | 'while' | 'do-while' | 'loop' | 'try' | 'catch' | 'finally' | 'expression-stmt' | 'block' | 'comment' | 'label' | 'goto' | 'defer' | 'with' | 'using' | 'async-stmt' | 'await-stmt' | 'assert' | 'print' | 'raw';
export interface ULIRStatement {
    kind: ULIRStatementKind;
    condition?: ULIRExpression;
    target?: ULIRExpression;
    value?: ULIRExpression;
    type?: ULIRType;
    isMut?: boolean;
    isConst?: boolean;
    body?: ULIRStatement[];
    elseBody?: ULIRStatement[];
    cases?: ULIRCase[];
    iterVar?: string;
    iterSource?: ULIRExpression;
    catchClauses?: ULIRCatch[];
    finallyBody?: ULIRStatement[];
    raw?: string;
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
export type ULIRExpressionKind = 'literal' | 'identifier' | 'call' | 'method-call' | 'new' | 'field-access' | 'index' | 'binary-op' | 'unary-op' | 'ternary' | 'cast' | 'instanceof' | 'typeof' | 'await' | 'yield-expr' | 'lambda-expr' | 'array-lit' | 'object-lit' | 'tuple-lit' | 'template-lit' | 'spread' | 'destructure' | 'match-expr' | 'if-expr' | 'range' | 'closure-expr' | 'raw-expr';
export interface ULIRExpression {
    kind: ULIRExpressionKind;
    raw: string;
    type?: ULIRType;
    callee?: string;
    args?: ULIRExpression[];
    op?: string;
    left?: ULIRExpression;
    right?: ULIRExpression;
    literalKind?: 'int' | 'float' | 'string' | 'bool' | 'null' | 'char' | 'regex';
    literalValue?: string;
    object?: ULIRExpression;
    field?: string;
    elements?: ULIRExpression[];
    entries?: Array<{
        key: string;
        value: ULIRExpression;
    }>;
}
export declare function rawExpr(raw: string): ULIRExpression;
export declare function litExpr(kind: ULIRExpression['literalKind'], value: string): ULIRExpression;
export interface OmniCCConversionRequest {
    source: string;
    sourceLang?: string;
    targetLang: string;
    filename?: string;
    widgetNameHint?: string;
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
    idiomaticTarget: boolean;
    includeTests: boolean;
    strictTypes: boolean;
    targetRuntime?: string;
    encoding: string;
    lineEndings: 'lf' | 'crlf' | 'cr';
    enableWidgetBridge?: boolean;
    mergeWidgetBridge?: boolean;
}
export declare const DEFAULT_OPTIONS: ConversionOptions;
export interface OmniCCConversionResult {
    success: boolean;
    output: string;
    code?: string;
    sourceLanguage: string;
    targetLanguage: string;
    sourceLangId?: string;
    targetLangId?: string;
    confidence: number;
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
    projectResults?: ProjectFileResult[];
}
export interface ProjectFileResult {
    sourcePath?: string;
    outputPath?: string;
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
    popularity: number;
    color: string;
    description: string;
    keywords: string[];
    shebang?: string;
    comment: {
        line: string;
        blockStart?: string;
        blockEnd?: string;
    };
    indentStyle: 'braces' | 'indent' | 'begin-end' | 'none' | 'parentheses' | 'mixed';
    features: string[];
    fileExtensionMap?: Record<string, string>;
}
//# sourceMappingURL=ULIR.d.ts.map