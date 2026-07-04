"use strict";
// OmniCC Universal Language Intermediate Representation (ULIR)
// The backbone of every conversion — all parsers write ULIR, all generators read it.
// Paradigm-agnostic, rich enough for any language construct.
Object.defineProperty(exports, "__esModule", { value: true });
exports.DEFAULT_OPTIONS = exports.UNKNOWN_TYPE = exports.ANY_TYPE = exports.BOOL_TYPE = exports.FLOAT_TYPE = exports.INT_TYPE = exports.STRING_TYPE = exports.VOID_TYPE = void 0;
exports.rawExpr = rawExpr;
exports.litExpr = litExpr;
exports.VOID_TYPE = { name: 'void', nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'void' };
exports.STRING_TYPE = { name: 'String', nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'String' };
exports.INT_TYPE = { name: 'Int', nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'Int' };
exports.FLOAT_TYPE = { name: 'Float', nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'Float' };
exports.BOOL_TYPE = { name: 'Bool', nullable: false, optional: false, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: true, isMapped: false, originalSrc: 'Bool' };
exports.ANY_TYPE = { name: 'Any', nullable: true, optional: true, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: false, isMapped: false, originalSrc: 'any' };
exports.UNKNOWN_TYPE = { name: 'Unknown', nullable: true, optional: true, generic: [], isArray: false, arrayDims: 0, isFunction: false, isUnion: false, isIntersection: false, isTuple: false, isLiteral: false, isPrimitive: false, isMapped: false, originalSrc: '?' };
function rawExpr(raw) {
    return { kind: 'raw-expr', raw };
}
function litExpr(kind, value) {
    return { kind: 'literal', raw: value, literalKind: kind, literalValue: value };
}
exports.DEFAULT_OPTIONS = {
    preserveComments: true,
    preserveFormatting: false,
    idiomaticTarget: true,
    includeTests: false,
    strictTypes: true,
    encoding: 'utf-8',
    lineEndings: 'lf',
};
//# sourceMappingURL=ULIR.js.map