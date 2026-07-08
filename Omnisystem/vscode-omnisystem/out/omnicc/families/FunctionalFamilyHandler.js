"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseFunctionalFamily = parseFunctionalFamily;
exports.generateFunctionalFamily = generateFunctionalFamily;
// Functional-Family Handler — Haskell, OCaml, F#, Elm, Clojure, Common Lisp, Scheme, Racket, Erlang, Elixir
const ULIR_1 = require("../ULIR");
const LanguageRegistry_1 = require("../LanguageRegistry");
const BodyTranslator_1 = require("../BodyTranslator");
// ─── Parse ────────────────────────────────────────────────────────────────────
function parseFunctionalFamily(source, langId) {
    const lang = (0, LanguageRegistry_1.getLang)(langId);
    const lines = source.split('\n');
    const units = [];
    const imports = [];
    // Imports
    for (const line of lines) {
        const imp = extractImport(line.trim(), langId);
        if (imp) {
            imports.push(imp);
        }
    }
    // Extract definitions based on language family
    if (langId === 'haskell') {
        extractHaskellDefs(source, lines, units);
    }
    else if (langId === 'ocaml' || langId === 'fsharp') {
        extractMLDefs(source, lines, units, langId);
    }
    else if (langId === 'clojure' || langId === 'commonlisp' || langId === 'scheme' || langId === 'racket') {
        extractLispDefs(source, lines, units, langId);
    }
    else if (langId === 'elm') {
        extractElmDefs(source, lines, units);
    }
    else if (langId === 'elixir' || langId === 'erlang') {
        extractBeamDefs(source, lines, units, langId);
    }
    const meta = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['functional'],
        typeSystem: lang?.typing ?? 'static-strong',
        memoryModel: lang?.memory ?? 'gc',
        usesAsync: /\basync\b|\bawait\b|async\/await|Future\b|Promise\b|Task\.async/.test(source),
        usesGenerics: /forall\s+\w+|'[a-z]\b|[a-z]\s+->\s+\w+/.test(source),
        usesReflection: false,
        usesMetaprogramming: /\bquote\b|\bmacro\b|\bTemplate\b|'(.+')/.test(source),
        hasTests: /\btest\b|#\[test\]|HUnit\b|QuickCheck\b/.test(source),
        hasUI: /\bview\b.*\bHtml\b|\bElement\b.*\bHtml\b/.test(source),
        hasSideEffects: /\bIO\b|\bputStrLn\b|\bprintln!\b|printf/.test(source),
    };
    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'functional',
        units,
        imports,
        exports: [],
        docComment: '',
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes: [],
    };
}
function extractHaskellDefs(source, lines, units) {
    // Type signatures + definitions
    const sigMap = new Map();
    for (const line of lines) {
        const m = line.match(/^(\w+)\s*::\s*(.+)$/);
        if (m && !/^(import|type|data|newtype|class|instance)/.test(line)) {
            sigMap.set(m[1], m[2].trim());
        }
    }
    for (const [name, typeSig] of sigMap) {
        // Check there's a corresponding definition
        if (new RegExp(`^${name}\\s+`).test(source)) {
            units.push(makeFnUnit(name, [], ULIR_1.UNKNOWN_TYPE, 'haskell', `-- ${name} :: ${typeSig}`));
        }
    }
    // Data types
    for (const m of source.matchAll(/^data\s+(\w+)/gm)) {
        units.push({ kind: 'type-alias', name: m[1], visibility: 'public', signature: { params: [], returns: ULIR_1.VOID_TYPE, throws: [] }, body: [], attributes: [], docComment: '', sourceLines: [0, 0], isAsync: false, isStatic: false, isAbstract: false, isFinal: false, isOverride: false, isExtern: false, generics: [], extends_: [], implements_: [], children: [], originalSource: m[0], confidence: 'high' });
    }
}
function extractMLDefs(source, lines, units, langId) {
    // OCaml/F#: let name args = body
    for (const m of source.matchAll(/^let\s+(?:rec\s+)?(\w+)\s*((?:\w+\s+)*)/gm)) {
        const name = m[1];
        if (['in', 'and', 'type', 'module', 'open'].includes(name)) {
            continue;
        }
        const paramNames = m[2].trim().split(/\s+/).filter(Boolean);
        units.push(makeFnUnit(name, paramNames.map(n => ({ name: n, type: ULIR_1.ANY_TYPE, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false })), ULIR_1.UNKNOWN_TYPE, langId, m[0]));
    }
    // Type definitions
    for (const m of source.matchAll(/^type\s+(\w+)/gm)) {
        units.push({ kind: 'type-alias', name: m[1], visibility: 'public', signature: { params: [], returns: ULIR_1.VOID_TYPE, throws: [] }, body: [], attributes: [], docComment: '', sourceLines: [0, 0], isAsync: false, isStatic: false, isAbstract: false, isFinal: false, isOverride: false, isExtern: false, generics: [], extends_: [], implements_: [], children: [], originalSource: m[0], confidence: 'high' });
    }
}
function extractLispDefs(source, lines, units, langId) {
    // (defn name [params] body) — Clojure
    // (defun name (params) body) — Common Lisp
    // (define (name params) body) — Scheme
    const patterns = [
        /\(defn\s+(\w[\w?!-]*)\s+\[([^\]]*)\]/g,
        /\(defun\s+(\w[\w?!-]*)\s+\(([^)]*)\)/g,
        /\(define\s+\((\w[\w?!-]*)\s*([^)]*)\)/g,
        /\(def\s+(\w[\w?!-]*)\s+/g,
    ];
    for (const p of patterns) {
        for (const m of source.matchAll(p)) {
            const name = m[1];
            const paramStr = m[2] ?? '';
            const params = paramStr.trim().split(/\s+/).filter(Boolean)
                .map(n => ({ name: n, type: ULIR_1.ANY_TYPE, defaultValue: undefined, isVariadic: n.startsWith('&'), isKeyword: n.startsWith(':'), isRef: false, isMut: false }));
            units.push(makeFnUnit(name, params, ULIR_1.UNKNOWN_TYPE, langId, m[0]));
        }
    }
}
function extractElmDefs(source, lines, units) {
    // Elm: name = ... / name params = ...
    for (const m of source.matchAll(/^(\w+)\s+((?:\w+\s+)*)?=/gm)) {
        const name = m[1];
        if (name === 'type' || name === 'import' || name === 'module') {
            continue;
        }
        units.push(makeFnUnit(name, [], ULIR_1.UNKNOWN_TYPE, 'elm', m[0]));
    }
}
function extractBeamDefs(source, lines, units, langId) {
    if (langId === 'elixir') {
        for (const m of source.matchAll(/def\s+(\w+)\(([^)]*)\)/g)) {
            const params = m[2] ? m[2].split(',').map(p => ({ name: p.trim().split(':')[0].trim() || 'arg', type: ULIR_1.ANY_TYPE, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false })) : [];
            units.push(makeFnUnit(m[1], params, ULIR_1.UNKNOWN_TYPE, 'elixir', m[0]));
        }
    }
    else {
        for (const m of source.matchAll(/^(\w+)\(([^)]*)\)\s*->/gm)) {
            units.push(makeFnUnit(m[1], [], ULIR_1.UNKNOWN_TYPE, 'erlang', m[0]));
        }
    }
}
function makeFnUnit(name, params, ret, langId, src) {
    return {
        kind: 'function',
        name,
        visibility: 'public',
        signature: { params, returns: ret, throws: [] },
        body: [],
        attributes: [],
        docComment: '',
        sourceLines: [0, 0],
        isAsync: false, isStatic: false, isAbstract: false,
        isFinal: false, isOverride: false, isExtern: false,
        generics: [], extends_: [], implements_: [], children: [],
        originalSource: src,
        confidence: 'medium',
    };
}
function extractImport(line, langId) {
    // Haskell: import Data.List (sort, nub)
    let m = line.match(/^import\s+(?:qualified\s+)?([A-Z][\w.]+)\s*(?:\(([^)]+)\))?(?:\s+as\s+(\w+))?/);
    if (m && langId === 'haskell') {
        return { path: m[1].replace(/\./g, '/'), alias: m[3], names: m[2] ? m[2].split(',').map(n => n.trim()) : [], isDefault: false, isWildcard: !m[2], kind: 'package', originalSyntax: line };
    }
    // OCaml: open List
    m = line.match(/^open\s+(\w+)/);
    if (m && langId === 'ocaml') {
        return { path: m[1], alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'package', originalSyntax: line };
    }
    // F#: open System.Collections.Generic
    m = line.match(/^open\s+([\w.]+)/);
    if (m && langId === 'fsharp') {
        return { path: m[1].replace(/\./g, '/'), alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'package', originalSyntax: line };
    }
    // Clojure: (ns my.ns (:require [clojure.string :as str]))
    m = line.match(/\(:require\s+\[([^\]]+)\]/);
    if (m) {
        const parts = m[1].trim().split(/\s+/);
        return { path: parts[0].replace(/\./g, '/'), alias: parts[parts.indexOf(':as') + 1], names: [], isDefault: false, isWildcard: true, kind: 'package', originalSyntax: line };
    }
    // Elixir: import Mod / alias Mod
    m = line.match(/^(?:import|alias|use)\s+([\w.]+)/);
    if (m && langId === 'elixir') {
        return { path: m[1].replace(/\./g, '/'), alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'package', originalSyntax: line };
    }
    // Elm: import List exposing (sort)
    m = line.match(/^import\s+([\w.]+)(?:\s+exposing\s+\(([^)]+)\))?/);
    if (m && langId === 'elm') {
        return { path: m[1].replace(/\./g, '/'), alias: undefined, names: m[2] ? m[2].split(',').map(n => n.trim()) : [], isDefault: false, isWildcard: !m[2], kind: 'package', originalSyntax: line };
    }
    return null;
}
function detectModuleName(source, langId) {
    const m = source.match(/^module\s+([\w.]+)/m);
    if (m) {
        return m[1].split('.').pop() ?? m[1];
    }
    return 'Module';
}
// ─── Generate ─────────────────────────────────────────────────────────────────
function generateFunctionalFamily(ir, targetLangId, opts = ULIR_1.DEFAULT_OPTIONS) {
    const lang = (0, LanguageRegistry_1.getLang)(targetLangId);
    const lines = [];
    const commentStyle = lang?.comment?.line ?? '--';
    lines.push(`${commentStyle} ${ir.name} — Converted to ${lang?.name ?? targetLangId}`);
    lines.push(`${commentStyle} Source: ${ir.sourceLanguage}`);
    lines.push('');
    // Module declaration
    const modDecl = renderModuleDecl(ir.name, targetLangId);
    if (modDecl) {
        lines.push(modDecl);
        lines.push('');
    }
    // Imports
    for (const imp of ir.imports) {
        lines.push(renderImport(imp, targetLangId));
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    // Units
    for (const unit of ir.units) {
        lines.push(renderUnit(unit, targetLangId, opts, ir.sourceLanguage));
        lines.push('');
    }
    return lines.join('\n');
}
function renderModuleDecl(name, lang) {
    switch (lang) {
        case 'haskell': return `module ${name} where`;
        case 'ocaml': return `(* module ${name} *)`;
        case 'fsharp': return `module ${name}`;
        case 'clojure': return `(ns ${name.toLowerCase()})`;
        case 'elm': return `module ${name} exposing (..)`;
        case 'elixir': return `defmodule ${name} do`;
        case 'erlang': return `-module(${name.toLowerCase()}).`;
        default: return '';
    }
}
function renderImport(imp, lang) {
    const path = imp.path.replace(/\//g, '.');
    switch (lang) {
        case 'haskell':
            if (imp.names.length > 0) {
                return `import ${path} (${imp.names.join(', ')})`;
            }
            return `import ${path}`;
        case 'ocaml': return `open ${path.split('.').pop() ?? path}`;
        case 'fsharp': return `open ${path}`;
        case 'clojure': return `(require '[${path.replace(/\./g, '/')} :as ${imp.alias ?? (path.split('.').pop() ?? 'mod')}])`;
        case 'elm':
            if (imp.names.length > 0) {
                return `import ${path} exposing (${imp.names.join(', ')})`;
            }
            return `import ${path}`;
        case 'elixir': return `alias ${path.split('.').map(s => s.charAt(0).toUpperCase() + s.slice(1)).join('.')}`;
        case 'erlang': return `-include("${imp.path}.hrl").`;
        default: return `-- import ${path}`;
    }
}
function renderUnit(unit, lang, opts, srcLang) {
    if (unit.kind === 'type-alias') {
        return renderTypeAlias(unit, lang);
    }
    return renderFunction(unit, lang, opts, srcLang);
}
function renderFunction(unit, lang, opts, srcLang) {
    const name = unit.name;
    const params = unit.signature.params;
    const src = srcLang ?? unit.sourceLanguage ?? 'haskell';
    const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', src, lang);
    switch (lang) {
        case 'haskell': {
            const paramStr = params.map(p => p.name).join(' ');
            const typeStr = params.length > 0
                ? params.map(_ => 'a').join(' -> ') + ' -> b'
                : 'b';
            return `${name} :: ${typeStr}\n${name} ${paramStr} =\n${body}`;
        }
        case 'ocaml': {
            const paramStr = params.length > 0 ? params.map(p => p.name).join(' ') : '()';
            return `let ${name} ${paramStr} =\n${body}`;
        }
        case 'fsharp': {
            const paramStr = params.length > 0 ? params.map(p => p.name).join(' ') : '()';
            return `let ${name} ${paramStr} =\n${body}`;
        }
        case 'clojure': {
            const paramStr = params.map(p => p.name).join(' ');
            return `(defn ${name} [${paramStr}]\n${body})`;
        }
        case 'commonlisp': {
            const paramStr = params.map(p => p.name).join(' ');
            return `(defun ${name} (${paramStr})\n${body})`;
        }
        case 'scheme':
        case 'racket': {
            const paramStr = params.map(p => p.name).join(' ');
            return `(define (${name} ${paramStr})\n${body})`;
        }
        case 'elm': {
            const paramStr = params.map(p => p.name).join(' ');
            return `${name} ${paramStr} =\n${body}`;
        }
        case 'elixir': {
            const paramStr = params.map(p => p.name).join(', ');
            return `def ${name}(${paramStr}) do\n${body}\nend`;
        }
        case 'erlang': {
            const paramStr = params.map(p => p.name.charAt(0).toUpperCase() + p.name.slice(1)).join(', ');
            return `${name}(${paramStr}) ->\n${body}.`;
        }
        default:
            return `-- ${name}\n${body}`;
    }
}
function renderTypeAlias(unit, lang) {
    switch (lang) {
        case 'haskell': return `data ${unit.name} = ${unit.name} deriving (Show, Eq)`;
        case 'ocaml': return `type ${unit.name} = { data: string }`;
        case 'fsharp': return `type ${unit.name} = { Data: string }`;
        case 'clojure': return `(defrecord ${unit.name} [])`;
        case 'elm': return `type ${unit.name} = ${unit.name}`;
        case 'elixir': return `defmodule ${unit.name} do\n  defstruct []\nend`;
        default: return `-- type ${unit.name}`;
    }
}
//# sourceMappingURL=FunctionalFamilyHandler.js.map