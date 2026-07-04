"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseSystemsFamily = parseSystemsFamily;
exports.generateSystemsFamily = generateSystemsFamily;
// Systems-Family Handler — Rust, Zig, Go, Odin, C, C++, Assembly (x86/ARM), WASM, V, Nim, D, Ada
const ULIR_1 = require("../ULIR");
const LanguageRegistry_1 = require("../LanguageRegistry");
const BodyTranslator_1 = require("../BodyTranslator");
// ─── Parse ────────────────────────────────────────────────────────────────────
function parseSystemsFamily(source, langId) {
    const lang = (0, LanguageRegistry_1.getLang)(langId);
    const lines = source.split('\n');
    const units = [];
    const imports = [];
    switch (langId) {
        case 'rust':
            extractRustUnits(source, lines, units, imports);
            break;
        case 'zig':
            extractZigUnits(source, lines, units, imports);
            break;
        case 'go':
            extractGoUnits(source, lines, units, imports);
            break;
        case 'odin':
            extractOdinUnits(source, lines, units, imports);
            break;
        case 'c':
        case 'cpp':
            extractCUnits(source, lines, units, imports, langId);
            break;
        case 'asm-x86':
        case 'asm-arm':
            extractAsmUnits(source, lines, units, langId);
            break;
        case 'wasm':
            extractWasmUnits(source, units);
            break;
        case 'v':
            extractVLangUnits(source, lines, units, imports);
            break;
        case 'd':
            extractDLangUnits(source, lines, units, imports);
            break;
        default:
            extractCUnits(source, lines, units, imports, 'c');
            break;
    }
    const meta = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['systems', 'imperative'],
        typeSystem: lang?.typing ?? 'static-strong',
        memoryModel: lang?.memory ?? 'manual',
        usesAsync: /\basync\b|\bawait\b|\.await\b|goroutine\b/.test(source),
        usesGenerics: /\bfn\s+\w+<|<T>|comptime\s+T|anytype/.test(source),
        usesReflection: /\breflect\b|std\.meta\b|@typeOf\b/.test(source),
        usesMetaprogramming: /#\[derive|@compileError|macro_rules!|comptime\b/.test(source),
        hasTests: /#\[test\]|\btest\s+"|\bTEST\b|\btest\b/.test(source),
        hasUI: false,
        hasSideEffects: /\bprintln!\b|\bfmt\.Print\b|\bprintf\b|\bstd\.debug\.print/.test(source),
        entryPoint: /\bfn main\b|\bfunc main\b|\bpub fn main\b/.test(source) ? 'main' : undefined,
    };
    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'systems',
        units,
        imports,
        exports: units.filter(u => u.visibility === 'public').map(u => u.name),
        docComment: '',
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes: [],
    };
}
function extractRustUnits(source, lines, units, imports) {
    // use declarations
    for (const m of source.matchAll(/^use\s+([\w:]+)(?:::\{([^}]+)\})?(?:::\*)?;/gm)) {
        const names = m[2] ? m[2].split(',').map(n => n.trim()) : [];
        imports.push({ path: m[1].replace(/::/g, '/'), alias: undefined, names, isDefault: false, isWildcard: !m[2], kind: 'package', originalSyntax: m[0] });
    }
    // fn definitions
    for (const m of source.matchAll(/^(?:#\[(?:[^\]]+)\]\s*)*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)(?:<([^>]*)>)?\s*\(([^)]*)\)(?:\s*->\s*([\w<>,:&'? ]+))?/gm)) {
        const name = m[1];
        const genStr = m[2];
        const paramStr = m[3];
        const retStr = m[4];
        const vis = /^pub/.test(lines.find(l => l.includes(`fn ${name}`)) ?? '') ? 'public' : 'private';
        const isAsync = source.slice((m.index ?? 0) - 10, m.index ?? 0).includes('async');
        const attrs = extractRustAttrs(lines, source, m.index ?? 0);
        const params = parseRustParams(paramStr);
        units.push({
            kind: 'function',
            name,
            visibility: vis,
            signature: {
                params,
                returns: retStr ? { ...ULIR_1.UNKNOWN_TYPE, name: retStr.trim(), originalSrc: retStr } : ULIR_1.VOID_TYPE,
                throws: [],
            },
            body: [],
            attributes: attrs,
            docComment: extractRustDoc(source, m.index ?? 0),
            sourceLines: [0, 0],
            isAsync,
            isStatic: true,
            isAbstract: false, isFinal: false, isOverride: false, isExtern: /^extern/.test(lines.find(l => l.includes(`fn ${name}`)) ?? ''),
            generics: genStr ? genStr.split(',').map(g => ({ name: g.trim(), bounds: [], isVariadic: false })) : [],
            extends_: [], implements_: [], children: [],
            originalSource: m[0],
            confidence: 'high',
        });
    }
    // struct definitions
    for (const m of source.matchAll(/^(?:pub(?:\([^)]+\))?\s+)?struct\s+(\w+)(?:<([^>]*)>)?\s*\{/gm)) {
        units.push(makeTypeUnit('struct', m[1], m[0], m[1].includes('pub') ? 'public' : 'public'));
    }
    // enum definitions
    for (const m of source.matchAll(/^(?:pub(?:\([^)]+\))?\s+)?enum\s+(\w+)/gm)) {
        units.push(makeTypeUnit('enum', m[1], m[0], 'public'));
    }
    // trait definitions
    for (const m of source.matchAll(/^(?:pub(?:\([^)]+\))?\s+)?trait\s+(\w+)/gm)) {
        units.push(makeTypeUnit('trait', m[1], m[0], 'public'));
    }
    // impl blocks (methods)
    for (const m of source.matchAll(/^impl(?:<[^>]*>)?\s+(\w+)/gm)) {
        // find methods inside impl
        const implBody = source.slice(m.index ?? 0, (m.index ?? 0) + 2000);
        for (const fm of implBody.matchAll(/fn\s+(\w+)\s*\(&?(?:mut\s+)?self[^)]*\)/g)) {
            units.push({
                kind: 'method', name: fm[1],
                visibility: /pub\s+fn/.test(implBody.slice((fm.index ?? 0) - 4, fm.index ?? 0)) ? 'public' : 'private',
                signature: { params: [], returns: ULIR_1.VOID_TYPE, throws: [] },
                body: [], attributes: [], docComment: '',
                sourceLines: [0, 0],
                isAsync: false, isStatic: false, isAbstract: false,
                isFinal: false, isOverride: false, isExtern: false,
                generics: [], extends_: [], implements_: [m[1]], children: [],
                originalSource: fm[0],
                confidence: 'medium',
            });
        }
    }
}
function extractZigUnits(source, lines, units, imports) {
    // const imports
    for (const m of source.matchAll(/^const\s+(\w+)\s*=\s*@import\("([^"]+)"\)/gm)) {
        imports.push({ path: m[2], alias: m[1], names: [], isDefault: true, isWildcard: true, kind: 'external', originalSyntax: m[0] });
    }
    // pub fn / fn
    for (const m of source.matchAll(/^(?:pub\s+)?fn\s+(\w+)\s*\(([^)]*)\)(?:\s+(\w[\w?!*\[\]{}]+))?/gm)) {
        const isPub = source.slice((m.index ?? 0) - 5, m.index ?? 0).includes('pub');
        units.push({
            kind: 'function', name: m[1],
            visibility: isPub ? 'public' : 'private',
            signature: {
                params: m[2].split(',').filter(Boolean).map(p => {
                    const parts = p.trim().split(':');
                    return { name: (parts[0] ?? 'arg').trim(), type: { ...ULIR_1.UNKNOWN_TYPE, name: (parts[1] ?? '?').trim(), originalSrc: parts[1] }, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false };
                }),
                returns: m[3] ? { ...ULIR_1.UNKNOWN_TYPE, name: m[3], originalSrc: m[3] } : ULIR_1.VOID_TYPE,
                throws: [],
            },
            body: [], attributes: [], docComment: '',
            sourceLines: [0, 0],
            isAsync: false, isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: m[0], confidence: 'high',
        });
    }
    // struct / union / enum
    for (const m of source.matchAll(/^(?:pub\s+)?const\s+(\w+)\s*=\s*(?:struct|union|enum)\s*\{/gm)) {
        units.push(makeTypeUnit('struct', m[1], m[0], 'public'));
    }
}
function extractGoUnits(source, lines, units, imports) {
    // import blocks
    for (const m of source.matchAll(/import\s+"([^"]+)"/g)) {
        imports.push({ path: m[1], alias: undefined, names: [m[1].split('/').pop() ?? m[1]], isDefault: false, isWildcard: true, kind: 'external', originalSyntax: m[0] });
    }
    // func definitions (including methods)
    for (const m of source.matchAll(/^func\s+(?:\((\w+)\s+\*?(\w+)\)\s+)?(\w+)\s*\(([^)]*)\)(?:\s+\(?[\w,*\[\] ]+\)?)?\s*\{/gm)) {
        const receiver = m[2];
        const name = m[3];
        const params = m[4].split(',').filter(Boolean).map(p => {
            const parts = p.trim().split(/\s+/);
            return { name: parts[0] ?? 'arg', type: { ...ULIR_1.UNKNOWN_TYPE, name: parts[1] ?? 'interface{}', originalSrc: parts[1] }, defaultValue: undefined, isVariadic: p.includes('...'), isKeyword: false, isRef: false, isMut: false };
        });
        units.push({
            kind: receiver ? 'method' : 'function', name,
            visibility: /^[A-Z]/.test(name) ? 'public' : 'private',
            signature: { params, returns: ULIR_1.UNKNOWN_TYPE, throws: [] },
            body: [], attributes: [], docComment: '',
            sourceLines: [0, 0],
            isAsync: false, isStatic: !receiver, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: receiver ? [m[2]] : [], children: [],
            originalSource: m[0], confidence: 'high',
        });
    }
    // type declarations
    for (const m of source.matchAll(/^type\s+(\w+)\s+(?:struct|interface)\s*\{/gm)) {
        const isInterface = m[0].includes('interface');
        units.push(makeTypeUnit(isInterface ? 'interface' : 'struct', m[1], m[0], /^[A-Z]/.test(m[1]) ? 'public' : 'private'));
    }
}
function extractOdinUnits(source, lines, units, imports) {
    for (const m of source.matchAll(/^import\s+(\w+)\s+"([^"]+)"/gm)) {
        imports.push({ path: m[2], alias: m[1], names: [], isDefault: false, isWildcard: true, kind: 'external', originalSyntax: m[0] });
    }
    for (const m of source.matchAll(/^(\w+)\s*::\s*proc\s*\(([^)]*)\)/gm)) {
        units.push(makeTypeUnit('function', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^(\w+)\s*::\s*struct\s*\{/gm)) {
        units.push(makeTypeUnit('struct', m[1], m[0], 'public'));
    }
}
function extractCUnits(source, lines, units, imports, langId) {
    // #include
    for (const m of source.matchAll(/^#include\s+[<"]([\w./]+)[>"]/gm)) {
        imports.push({ path: m[1], alias: undefined, names: [], isDefault: false, isWildcard: true, kind: m[0].includes('<') ? 'stdlib' : 'relative', originalSyntax: m[0] });
    }
    // Function definitions
    for (const m of source.matchAll(/^(?:(?:static|inline|extern|const|volatile|unsigned)\s+)*(?:[\w*]+\s+)+(\w+)\s*\(([^)]*)\)\s*\{/gm)) {
        const name = m[1];
        if (['if', 'while', 'for', 'switch', 'do'].includes(name)) {
            continue;
        }
        const params = m[2].split(',').filter(Boolean).map(p => {
            const parts = p.trim().split(/\s+/);
            const name = parts[parts.length - 1]?.replace(/[*&]/, '') ?? 'arg';
            return { name, type: ULIR_1.ANY_TYPE, defaultValue: undefined, isVariadic: p.includes('...'), isKeyword: false, isRef: p.includes('&'), isMut: !p.includes('const') };
        });
        units.push({
            kind: 'function', name,
            visibility: /^static/.test(m[0]) ? 'private' : 'public',
            signature: { params, returns: ULIR_1.UNKNOWN_TYPE, throws: [] },
            body: [], attributes: [],
            docComment: extractCDoc(source, m.index ?? 0),
            sourceLines: [0, 0],
            isAsync: false, isStatic: /\bstatic\b/.test(m[0]), isAbstract: false, isFinal: false, isOverride: false,
            isExtern: /\bextern\b/.test(m[0]),
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: m[0], confidence: 'medium',
        });
    }
    // struct/class/enum
    for (const m of source.matchAll(/^(?:typedef\s+)?(?:struct|class|enum)\s+(\w+)/gm)) {
        const kind = m[0].includes('enum') ? 'enum' : m[0].includes('class') ? 'class' : 'struct';
        units.push(makeTypeUnit(kind, m[1], m[0], 'public'));
    }
}
function extractAsmUnits(source, lines, units, langId) {
    for (const m of source.matchAll(/^(\w+):/gm)) {
        const name = m[1];
        if (['section', 'global', 'extern', 'bits'].includes(name)) {
            continue;
        }
        units.push(makeTypeUnit('function', name, m[0], name.startsWith('.') ? 'private' : 'public'));
    }
}
function extractWasmUnits(source, units) {
    for (const m of source.matchAll(/\(func\s+\$(\w+)/g)) {
        units.push(makeTypeUnit('function', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/\(export\s+"(\w+)"/g)) {
        units.push(makeTypeUnit('variable', m[1], m[0], 'public'));
    }
}
function extractVLangUnits(source, lines, units, imports) {
    for (const m of source.matchAll(/^import\s+([\w.]+)/gm)) {
        imports.push({ path: m[1].replace(/\./g, '/'), alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'external', originalSyntax: m[0] });
    }
    for (const m of source.matchAll(/^(?:pub\s+)?fn\s+(?:\(\w+\s+\w+\)\s+)?(\w+)\s*\(/gm)) {
        units.push(makeTypeUnit('function', m[1], m[0], m[0].startsWith('pub') ? 'public' : 'private'));
    }
}
function extractDLangUnits(source, lines, units, imports) {
    for (const m of source.matchAll(/^import\s+([\w.]+);/gm)) {
        imports.push({ path: m[1].replace(/\./g, '/'), alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'external', originalSyntax: m[0] });
    }
    for (const m of source.matchAll(/^(?:public\s+)?(?:auto\s+|void\s+|int\s+|\w+\s+)?(\w+)\s*\([^)]*\)\s*\{/gm)) {
        units.push(makeTypeUnit('function', m[1], m[0], m[0].startsWith('public') ? 'public' : 'private'));
    }
}
function parseRustParams(raw) {
    if (!raw.trim() || raw.trim() === '&self' || raw.trim() === '&mut self' || raw.trim() === 'self') {
        return [];
    }
    return raw.split(',').map((p) => {
        const t = p.trim();
        if (t === '&self' || t === 'self' || t === '&mut self') {
            return null;
        }
        const m = t.match(/(\w+)\s*:\s*(.+)/);
        if (m) {
            return { name: m[1], type: { ...ULIR_1.UNKNOWN_TYPE, name: m[2].trim(), originalSrc: m[2] }, defaultValue: undefined, isVariadic: t.includes('..'), isKeyword: false, isRef: t.includes('&'), isMut: t.includes('mut') };
        }
        return { name: t.replace(/[^a-zA-Z0-9_]/g, '') || 'arg', type: ULIR_1.ANY_TYPE, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false };
    }).filter((p) => p !== null && p.name.length > 0);
}
function extractRustAttrs(lines, source, index) {
    const attrs = [];
    const pre = source.slice(Math.max(0, index - 200), index);
    for (const m of pre.matchAll(/#\[([^\]]+)\]/g)) {
        attrs.push(`#[${m[1]}]`);
    }
    return attrs.slice(-3);
}
function extractRustDoc(source, index) {
    const pre = source.slice(Math.max(0, index - 500), index);
    const docs = [];
    for (const m of pre.matchAll(/^\/\/\/\s?(.*)$/gm)) {
        docs.push(m[1]);
    }
    return docs.slice(-5).join('\n');
}
function extractCDoc(source, index) {
    const pre = source.slice(Math.max(0, index - 300), index);
    const m = pre.match(/\/\*\*([\s\S]*?)\*\//);
    return m ? m[1].replace(/^\s*\*\s?/gm, '').trim() : '';
}
function makeTypeUnit(kind, name, src, vis) {
    return {
        kind, name, visibility: vis,
        signature: { params: [], returns: ULIR_1.VOID_TYPE, throws: [] },
        body: [], attributes: [], docComment: '',
        sourceLines: [0, 0],
        isAsync: false, isStatic: false, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
        generics: [], extends_: [], implements_: [], children: [],
        originalSource: src.slice(0, 200), confidence: 'high',
    };
}
function detectModuleName(source, langId) {
    if (langId === 'rust') {
        const m = source.match(/^mod\s+(\w+)/m);
        if (m) {
            return m[1];
        }
    }
    if (langId === 'go') {
        const m = source.match(/^package\s+(\w+)/m);
        if (m) {
            return m[1];
        }
    }
    if (langId === 'zig') {
        const m = source.match(/^const\s+(\w+)\s*=\s*@import/m);
        if (m) {
            return m[1];
        }
    }
    return 'Module';
}
// ─── Generate ─────────────────────────────────────────────────────────────────
function generateSystemsFamily(ir, targetLangId, opts = ULIR_1.DEFAULT_OPTIONS) {
    switch (targetLangId) {
        case 'rust': return generateRust(ir, opts);
        case 'zig': return generateZig(ir, opts);
        case 'go': return generateGo(ir, opts);
        case 'c': return generateC(ir, opts);
        case 'cpp': return generateCpp(ir, opts);
        case 'odin': return generateOdin(ir, opts);
        case 'v': return generateV(ir, opts);
        default: return generateC(ir, opts);
    }
}
function generateRust(ir, opts) {
    const lines = [`// ${ir.name} — Converted to Rust`, `// Source: ${ir.sourceLanguage}`, ''];
    for (const imp of ir.imports) {
        lines.push(`use ${imp.path.replace(/\//g, '::')}${imp.names.length > 0 ? '::{' + imp.names.join(', ') + '}' : ''};`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    for (const unit of ir.units) {
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'rust');
        if (unit.kind === 'struct') {
            lines.push(`pub struct ${unit.name} {\n${body}\n}`);
        }
        else if (unit.kind === 'enum') {
            lines.push(`pub enum ${unit.name} {\n${body}\n}`);
        }
        else if (unit.kind === 'trait') {
            lines.push(`pub trait ${unit.name} {\n${body}\n}`);
        }
        else {
            const vis = unit.visibility === 'public' ? 'pub ' : '';
            const async_ = unit.isAsync ? 'async ' : '';
            const pStr = unit.signature.params.map(p => `${p.name}: ${p.type.name !== 'Unknown' ? p.type.name : 'String'}`).join(', ');
            const ret = unit.signature.returns.name !== 'void' && unit.signature.returns.name !== 'Unknown' ? ` -> ${unit.signature.returns.name}` : '';
            lines.push(`${vis}${async_}fn ${toSnakeCase(unit.name)}(${pStr})${ret} {\n${body}\n}`);
        }
        lines.push('');
    }
    return lines.join('\n');
}
function generateZig(ir, opts) {
    const lines = [`// ${ir.name} — Converted to Zig`, `// Source: ${ir.sourceLanguage}`, ''];
    for (const imp of ir.imports) {
        lines.push(`const ${imp.alias ?? imp.path.split('/').pop()} = @import("${imp.path}");`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    for (const unit of ir.units) {
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'zig');
        if (unit.kind === 'struct') {
            lines.push(`pub const ${unit.name} = struct {\n${body}\n};`);
        }
        else {
            const pStr = unit.signature.params.map(p => `${p.name}: ${p.type.name !== 'Unknown' ? p.type.name : 'anytype'}`).join(', ');
            const ret = unit.signature.returns.name !== 'void' && unit.signature.returns.name !== 'Unknown' ? unit.signature.returns.name : 'void';
            lines.push(`pub fn ${toSnakeCase(unit.name)}(${pStr}) ${ret} {\n${body}\n}`);
        }
        lines.push('');
    }
    return lines.join('\n');
}
function generateGo(ir, opts) {
    const lines = [`// ${ir.name} — Converted to Go`, `// Source: ${ir.sourceLanguage}`, '', `package ${ir.name.toLowerCase().replace(/\W/g, '') || 'main'}`, ''];
    if (ir.imports.length > 0) {
        lines.push('import (');
        for (const imp of ir.imports) {
            lines.push(`\t"${imp.path}"`);
        }
        lines.push(')', '');
    }
    for (const unit of ir.units) {
        const name = toPascalCase(unit.name);
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'go');
        if (unit.kind === 'struct') {
            lines.push(`type ${name} struct {\n${body}\n}`);
        }
        else if (unit.kind === 'interface') {
            lines.push(`type ${name} interface {\n${body}\n}`);
        }
        else {
            const pStr = unit.signature.params.map(p => `${p.name} ${p.type.name !== 'Unknown' ? p.type.name : 'string'}`).join(', ');
            lines.push(`func ${name}(${pStr}) {\n${body}\n}`);
        }
        lines.push('');
    }
    return lines.join('\n');
}
function generateC(ir, opts) {
    const lines = [`/* ${ir.name} — Converted to C */`, `/* Source: ${ir.sourceLanguage} */`, ''];
    for (const imp of ir.imports) {
        const isStd = imp.kind === 'stdlib';
        lines.push(`#include ${isStd ? '<' : '"'}${imp.path}${isStd ? '>' : '"'}`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    for (const unit of ir.units) {
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'c');
        if (unit.kind === 'struct') {
            lines.push(`typedef struct ${unit.name} {\n${body}\n} ${unit.name};`);
        }
        else {
            const pStr = unit.signature.params.length > 0
                ? unit.signature.params.map(p => `char* ${p.name}`).join(', ')
                : 'void';
            lines.push(`void ${toSnakeCase(unit.name)}(${pStr}) {\n${body}\n}`);
        }
        lines.push('');
    }
    return lines.join('\n');
}
function generateCpp(ir, opts) {
    const lines = [`// ${ir.name} — Converted to C++`, `// Source: ${ir.sourceLanguage}`, '', '#include <iostream>', '#include <string>', '#include <vector>', ''];
    lines.push(`namespace ${ir.name.toLowerCase() || 'ns'} {`, '');
    for (const unit of ir.units) {
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'cpp');
        if (unit.kind === 'class' || unit.kind === 'struct') {
            lines.push(`class ${unit.name} {\npublic:\n    ${unit.name}() {}\n${body}\n};`);
        }
        else {
            const pStr = unit.signature.params.map(p => `std::string ${p.name}`).join(', ');
            lines.push(`void ${unit.name}(${pStr}) {\n${body}\n}`);
        }
        lines.push('');
    }
    lines.push('}');
    return lines.join('\n');
}
function generateOdin(ir, opts) {
    const lines = [`// ${ir.name} — Converted to Odin`, `// Source: ${ir.sourceLanguage}`, '', `package ${ir.name.toLowerCase()}`, ''];
    for (const imp of ir.imports) {
        lines.push(`import ${imp.alias ?? imp.path.split('/').pop()} "${imp.path}"`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    for (const unit of ir.units) {
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'odin');
        if (unit.kind === 'struct') {
            const fields = unit.children.length > 0
                ? unit.children.map(c => `    ${toSnakeCase(c.name)}: string`).join(',\n')
                : (body || '    data: string');
            lines.push(`${unit.name} :: struct {`, fields, '}');
        }
        else {
            const pStr = unit.signature.params.map(p => `${p.name}: string`).join(', ');
            lines.push(`${toSnakeCase(unit.name)} :: proc(${pStr}) {`, body || `    fmt.println("${unit.name}")`, '}');
        }
        lines.push('');
    }
    return lines.join('\n');
}
function generateV(ir, opts) {
    const lines = [`// ${ir.name} — Converted to V`, `// Source: ${ir.sourceLanguage}`, '', `module ${ir.name.toLowerCase()}`, ''];
    for (const imp of ir.imports) {
        lines.push(`import ${imp.path.replace(/\//g, '.')}`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    for (const unit of ir.units) {
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'v');
        if (unit.kind === 'struct') {
            const fields = unit.children.length > 0
                ? unit.children.map(c => `    ${toSnakeCase(c.name)} string`).join('\n')
                : (body || '    data string');
            lines.push(`pub struct ${unit.name} {`, fields, '}');
        }
        else {
            const vis = unit.visibility === 'public' ? 'pub ' : '';
            const pStr = unit.signature.params.map(p => `${p.name} string`).join(', ');
            lines.push(`${vis}fn ${toSnakeCase(unit.name)}(${pStr}) {`, body || `    println('${unit.name}')`, '}');
        }
        lines.push('');
    }
    return lines.join('\n');
}
function toSnakeCase(s) {
    return s.replace(/([A-Z])/g, c => '_' + c.toLowerCase()).replace(/^_/, '');
}
function toPascalCase(s) {
    return s.charAt(0).toUpperCase() + s.slice(1).replace(/_(\w)/g, (_, c) => c.toUpperCase());
}
//# sourceMappingURL=SystemsFamilyHandler.js.map