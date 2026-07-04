"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseCFamily = parseCFamily;
exports.generateCFamily = generateCFamily;
// C-Family Handler — C, C++, Java, JavaScript, TypeScript, C#, Go, Kotlin, Swift, Dart, Scala, ...
const ULIR_1 = require("../ULIR");
const LanguageRegistry_1 = require("../LanguageRegistry");
const BodyTranslator_1 = require("../BodyTranslator");
// ─── Parse ────────────────────────────────────────────────────────────────────
function parseCFamily(source, langId) {
    const lang = (0, LanguageRegistry_1.getLang)(langId);
    const lines = source.split('\n');
    const units = [];
    const imports = [];
    const notes = [];
    // Extract imports/usings
    for (const line of lines) {
        const t = line.trim();
        const imp = extractImport(t, langId);
        if (imp) {
            imports.push(imp);
        }
    }
    // Extract top-level functions
    const fnPattern = /(?:(?:public|private|protected|static|async|export|default|override|abstract|final|sealed)\s+)*(?:[\w<>,\[\]?]+\s+)?(\w+)\s*\(([^)]*)\)\s*(?::\s*[\w<>?,\[\]|&]+\s*)?\{/;
    let inBlock = 0;
    let blockStart = -1;
    let blockHeader = '';
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (fnPattern.test(line) && inBlock === 0) {
            blockStart = i;
            blockHeader = line;
        }
        for (const ch of line) {
            if (ch === '{') {
                inBlock++;
            }
            if (ch === '}') {
                inBlock--;
            }
        }
        if (inBlock === 0 && blockStart >= 0) {
            const body = lines.slice(blockStart, i + 1).join('\n');
            const unit = parseFunctionBlock(blockHeader, body, langId);
            if (unit) {
                units.push(unit);
            }
            blockStart = -1;
        }
    }
    // Extract classes/structs
    const classPattern = /(?:(?:public|private|abstract|sealed|final|data|record|case)\s+)*(?:class|struct|interface|trait|record|enum)\s+(\w+)/;
    for (let i = 0; i < lines.length; i++) {
        const m = lines[i].match(classPattern);
        if (m) {
            const classUnit = parseClassBlock(lines, i, langId);
            if (classUnit) {
                units.push(classUnit);
            }
        }
    }
    const hasUI = source.includes('render') || source.includes('Component') ||
        source.includes('Widget') || source.includes('<div') || source.includes('View');
    const meta = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['imperative'],
        typeSystem: lang?.typing ?? 'static-strong',
        memoryModel: lang?.memory ?? 'gc',
        usesAsync: /\basync\b/.test(source) || /\bawait\b/.test(source) || /\bCoroutine\b/.test(source),
        usesGenerics: /<[\w,\s]+>/.test(source) || /\[[\w,\s]+\]/.test(source),
        usesReflection: /reflect\.|Reflection|getClass\(\)|typeof/.test(source),
        usesMetaprogramming: /@\w+|#\[derive|macro_rules!|annotation/.test(source),
        hasTests: /\btest\b|\bspec\b|@Test|#\[test\]|describe\(|it\(|expect\(/.test(source),
        hasUI,
        hasSideEffects: /console\.|print|System\.out|fmt\.Print|putchar|printf/.test(source),
        entryPoint: detectEntryPoint(source, langId),
    };
    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'c-family',
        units,
        imports,
        exports: extractExports(source, langId),
        docComment: extractModuleDoc(source),
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes,
    };
}
function extractImport(line, langId) {
    // JavaScript/TypeScript
    let m = line.match(/^import\s+(?:\{([^}]+)\}|(\w+)|\*\s+as\s+(\w+))\s+from\s+['"]([^'"]+)['"]/);
    if (m) {
        return {
            path: m[4], alias: m[3],
            names: m[1] ? m[1].split(',').map(n => n.trim()) : m[2] ? [m[2]] : [],
            isDefault: !!m[2], isWildcard: !!m[3],
            kind: m[4].startsWith('.') ? 'relative' : 'external',
            originalSyntax: line,
        };
    }
    // Java/Kotlin/C#
    m = line.match(/^(?:import|using)\s+([\w.]+)(?:\.\*)?;?$/);
    if (m) {
        const parts = m[1].split('.');
        return {
            path: m[1], alias: undefined,
            names: [parts[parts.length - 1]],
            isDefault: false, isWildcard: line.includes('.*'),
            kind: 'package',
            originalSyntax: line,
        };
    }
    // C/C++
    m = line.match(/^#include\s+[<"]([\w./]+)[>"]/);
    if (m) {
        return {
            path: m[1], alias: undefined, names: [],
            isDefault: false, isWildcard: true,
            kind: line.includes('<') ? 'stdlib' : 'relative',
            originalSyntax: line,
        };
    }
    return null;
}
function parseFunctionBlock(header, body, langId) {
    const m = header.match(/(\w+)\s*\(([^)]*)\)/);
    if (!m) {
        return null;
    }
    const name = m[1];
    if (['if', 'for', 'while', 'switch', 'catch'].includes(name)) {
        return null;
    }
    const params = parseParamList(m[2], langId);
    const returnType = extractReturnType(header, langId);
    const vis = extractVisibility(header);
    const isAsync = /\basync\b/.test(header) || /\bsuspend\b/.test(header);
    const isStatic = /\bstatic\b/.test(header);
    return {
        kind: 'function',
        name,
        visibility: vis,
        signature: { params, returns: returnType, throws: [], selfParam: undefined },
        body: [{ kind: 'raw', raw: body }],
        attributes: extractAttributes(header),
        docComment: '',
        sourceLines: [0, 0],
        isAsync,
        isStatic,
        isAbstract: /\babstract\b/.test(header),
        isFinal: /\bfinal\b|\bsealed\b/.test(header),
        isOverride: /\boverride\b/.test(header),
        isExtern: /\bextern\b/.test(header),
        generics: [],
        extends_: [],
        implements_: [],
        children: [],
        originalSource: body,
        confidence: 'medium',
    };
}
function parseClassBlock(lines, startIdx, langId) {
    const header = lines[startIdx];
    const m = header.match(/(?:class|struct|interface|trait|record|enum)\s+(\w+)(?:\s*(?:extends|:)\s*([\w,\s]+))?(?:\s+implements\s+([\w,\s]+))?/);
    if (!m) {
        return null;
    }
    const name = m[1];
    const extendsRaw = m[2] ? m[2].split(',').map(s => s.trim()) : [];
    const implRaw = m[3] ? m[3].split(',').map(s => s.trim()) : [];
    const kind = /interface/.test(header) ? 'interface' :
        /trait/.test(header) ? 'trait' :
            /enum/.test(header) ? 'enum' :
                /record/.test(header) ? 'record' : 'class';
    return {
        kind,
        name,
        visibility: extractVisibility(header),
        signature: { params: [], returns: ULIR_1.VOID_TYPE, throws: [] },
        body: [],
        attributes: extractAttributes(header),
        docComment: '',
        sourceLines: [startIdx, startIdx],
        isAsync: false, isStatic: /\bstatic\b/.test(header),
        isAbstract: /\babstract\b/.test(header), isFinal: /\bfinal\b/.test(header),
        isOverride: false, isExtern: false,
        generics: [],
        extends_: extendsRaw,
        implements_: implRaw,
        children: [],
        originalSource: lines[startIdx],
        confidence: 'medium',
    };
}
function parseParamList(raw, langId) {
    if (!raw.trim()) {
        return [];
    }
    return raw.split(',').map(p => {
        const t = p.trim();
        // TypeScript: name: Type
        let m = t.match(/^(\w+)\s*:\s*([\w<>?,\[\]|]+)/);
        if (m) {
            return { name: m[1], type: { ...ULIR_1.STRING_TYPE, name: m[2], originalSrc: m[2] }, defaultValue: undefined, isVariadic: t.includes('...'), isKeyword: false, isRef: false, isMut: false };
        }
        // Java/C#: Type name
        m = t.match(/^([\w<>?,\[\]]+)\s+(\w+)$/);
        if (m) {
            return { name: m[2], type: { ...ULIR_1.STRING_TYPE, name: m[1], originalSrc: m[1] }, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false };
        }
        // Fallback: just name
        return { name: t.replace(/[^a-zA-Z0-9_]/g, '') || 'param', type: ULIR_1.UNKNOWN_TYPE, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false };
    }).filter(p => p.name.length > 0);
}
function extractReturnType(header, langId) {
    // TypeScript: ): ReturnType
    let m = header.match(/\)\s*:\s*([\w<>?,\[\]|]+)/);
    if (m) {
        return { ...ULIR_1.VOID_TYPE, name: m[1], originalSrc: m[1] };
    }
    // Java/C#: returnType functionName(
    m = header.match(/(?:public|private|protected|static|async|final|abstract|override|synchronized|native|unsigned|const|inline)?\s*([\w<>?,\[\]]+)\s+\w+\s*\(/);
    if (m && !['void', 'int', 'string', 'bool', 'boolean', 'float', 'double', 'char', 'byte', 'long', 'short'].includes(m[1].toLowerCase()) && !/\bclass\b|\bstruct\b/.test(m[1])) {
        return { ...ULIR_1.VOID_TYPE, name: m[1], originalSrc: m[1] };
    }
    if (/\bvoid\b/i.test(header)) {
        return ULIR_1.VOID_TYPE;
    }
    return ULIR_1.UNKNOWN_TYPE;
}
function extractVisibility(header) {
    if (/\bprivate\b/.test(header)) {
        return 'private';
    }
    if (/\bprotected\b/.test(header)) {
        return 'protected';
    }
    if (/\binternal\b/.test(header)) {
        return 'internal';
    }
    if (/\bpublic\b|\bexport\b/.test(header)) {
        return 'public';
    }
    return 'public'; // JS/TS default
}
function extractAttributes(header) {
    const attrs = [];
    const m = header.match(/@\w+|#\[[\w:,()\s]+\]/g);
    if (m) {
        attrs.push(...m);
    }
    return attrs;
}
function extractExports(source, langId) {
    const exports = [];
    for (const m of source.matchAll(/export\s+(?:default\s+)?(?:function|class|const|let|var|type|interface)\s+(\w+)/g)) {
        exports.push(m[1]);
    }
    for (const m of source.matchAll(/module\.exports\s*=\s*\{([^}]+)\}/g)) {
        for (const n of m[1].split(',')) {
            exports.push(n.trim().split(':')[0].trim());
        }
    }
    return exports;
}
function detectEntryPoint(source, langId) {
    if (/\bfn main\b/.test(source)) {
        return 'main';
    }
    if (/\bpublic static void main\b/.test(source)) {
        return 'main';
    }
    if (/\bfunc main\b/.test(source)) {
        return 'main';
    }
    if (/\bfunction main\b/.test(source) || /\bmain\(\)/.test(source)) {
        return 'main';
    }
    return undefined;
}
function detectModuleName(source, langId) {
    let m = source.match(/^(?:package|module|namespace)\s+([\w.]+)/m);
    if (m) {
        return m[1].split('.').pop() ?? m[1];
    }
    m = source.match(/^(?:class|struct)\s+(\w+)/m);
    if (m) {
        return m[1];
    }
    return 'Module';
}
function extractModuleDoc(source) {
    const m = source.match(/^\/\*\*([\s\S]*?)\*\//);
    if (m) {
        return m[1].replace(/^\s*\*\s?/gm, '').trim();
    }
    return '';
}
// ─── Generate ─────────────────────────────────────────────────────────────────
function generateCFamily(ir, targetLangId, opts = ULIR_1.DEFAULT_OPTIONS) {
    const lang = (0, LanguageRegistry_1.getLang)(targetLangId);
    const lines = [];
    // File header comment
    lines.push(`// ${ir.name} — Converted to ${lang?.name ?? targetLangId}`);
    lines.push(`// Source: ${ir.sourceLanguage} | Confidence: ${ir.confidence}`);
    lines.push('');
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
function renderImport(imp, targetLangId) {
    const path = imp.path;
    switch (targetLangId) {
        case 'javascript':
            if (imp.isWildcard) {
                return `import * as ${imp.alias ?? 'mod'} from '${path}';`;
            }
            if (imp.names.length > 0) {
                return `import { ${imp.names.join(', ')} } from '${path}';`;
            }
            return `import '${path}';`;
        case 'typescript':
            if (imp.isWildcard) {
                return `import * as ${imp.alias ?? 'mod'} from '${path}';`;
            }
            if (imp.names.length > 0) {
                return `import { ${imp.names.join(', ')} } from '${path}';`;
            }
            return `import '${path}';`;
        case 'python':
            if (imp.names.length > 0) {
                return `from ${path.replace(/[/\\]/g, '.')} import ${imp.names.join(', ')}`;
            }
            return `import ${path.replace(/[/\\]/g, '.')}`;
        case 'java':
        case 'kotlin':
            return `import ${path.replace(/\//g, '.')};`;
        case 'csharp':
            return `using ${path.replace(/\//g, '.')};`;
        case 'go':
            return `import "${path}"`;
        case 'rust':
            return `use ${path.replace(/\//g, '::')};`;
        case 'swift':
            return `import ${path.split('/').pop() ?? path}`;
        default:
            return `// import: ${path}`;
    }
}
function renderUnit(unit, targetLangId, opts, srcLang) {
    if (unit.kind === 'class' || unit.kind === 'struct' || unit.kind === 'interface' || unit.kind === 'record') {
        return renderClass(unit, targetLangId, opts, srcLang);
    }
    return renderFunction(unit, targetLangId, opts, srcLang);
}
function renderFunction(unit, lang, opts, srcLang) {
    const name = unit.name;
    const params = unit.signature.params;
    const ret = unit.signature.returns;
    const src = srcLang ?? unit.sourceLanguage ?? 'javascript';
    const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', src, lang);
    switch (lang) {
        case 'javascript':
            return `${unit.isAsync ? 'async ' : ''}function ${name}(${params.map(p => p.name).join(', ')}) {\n${body}\n}`;
        case 'typescript': {
            const paramStr = params.map(p => `${p.name}: ${mapType(p.type, 'typescript')}`).join(', ');
            const retStr = ret.name !== 'Unknown' ? `: ${mapType(ret, 'typescript')}` : '';
            return `${unit.isAsync ? 'async ' : ''}function ${name}(${paramStr})${retStr} {\n${body}\n}`;
        }
        case 'python': {
            const paramStr = params.map(p => opts.strictTypes ? `${p.name}: ${mapType(p.type, 'python')}` : p.name).join(', ');
            const retAnnot = opts.strictTypes && ret.name !== 'Unknown' ? ` -> ${mapType(ret, 'python')}` : '';
            const asyncPfx = unit.isAsync ? 'async ' : '';
            return `${asyncPfx}def ${toSnakeCase(name)}(${paramStr})${retAnnot}:\n${body}`;
        }
        case 'java': {
            const vis = unit.visibility === 'public' ? 'public ' : 'private ';
            const staticStr = unit.isStatic ? 'static ' : '';
            const paramStr = params.map(p => `${mapType(p.type, 'java')} ${p.name}`).join(', ');
            const retStr = mapType(ret, 'java');
            return `${vis}${staticStr}${retStr} ${name}(${paramStr}) {\n${body}\n}`;
        }
        case 'kotlin': {
            const vis = unit.visibility === 'public' ? '' : 'private ';
            const paramStr = params.map(p => `${p.name}: ${mapType(p.type, 'kotlin')}`).join(', ');
            const retStr = ret.name !== 'Unknown' && ret.name !== 'void' ? `: ${mapType(ret, 'kotlin')}` : '';
            const suspend = unit.isAsync ? 'suspend ' : '';
            return `${vis}${suspend}fun ${name}(${paramStr})${retStr} {\n${body}\n}`;
        }
        case 'csharp': {
            const vis = unit.visibility === 'public' ? 'public' : 'private';
            const asyncStr = unit.isAsync ? 'async ' : '';
            const paramStr = params.map(p => `${mapType(p.type, 'csharp')} ${p.name}`).join(', ');
            const retStr = unit.isAsync ? `Task<${mapType(ret, 'csharp')}>` : mapType(ret, 'csharp');
            return `${vis} ${asyncStr}${retStr} ${name}(${paramStr})\n{\n${body}\n}`;
        }
        case 'go': {
            const paramStr = params.map(p => `${p.name} ${mapType(p.type, 'go')}`).join(', ');
            const retStr = ret.name !== 'void' && ret.name !== 'Unknown' ? ` ${mapType(ret, 'go')}` : '';
            return `func ${name}(${paramStr})${retStr} {\n${body}\n}`;
        }
        case 'rust': {
            const vis = unit.visibility === 'public' ? 'pub ' : '';
            const paramStr = params.map(p => `${p.name}: ${mapType(p.type, 'rust')}`).join(', ');
            const retStr = ret.name !== 'void' && ret.name !== 'Unknown' ? ` -> ${mapType(ret, 'rust')}` : '';
            const asyncStr = unit.isAsync ? 'async ' : '';
            return `${vis}${asyncStr}fn ${toSnakeCase(name)}(${paramStr})${retStr} {\n${body}\n}`;
        }
        case 'swift': {
            const paramStr = params.map(p => `${p.name}: ${mapType(p.type, 'swift')}`).join(', ');
            const retStr = ret.name !== 'void' && ret.name !== 'Unknown' ? ` -> ${mapType(ret, 'swift')}` : '';
            const asyncStr = unit.isAsync ? 'async ' : '';
            return `${asyncStr}func ${name}(${paramStr})${retStr} {\n${body}\n}`;
        }
        case 'dart': {
            const paramStr = params.map(p => `${mapType(p.type, 'dart')} ${p.name}`).join(', ');
            const retStr = mapType(ret, 'dart');
            const asyncStr = unit.isAsync ? 'async ' : '';
            return `${retStr} ${asyncStr}${name}(${paramStr}) {\n${body}\n}`;
        }
        default:
            return `// ${name}(${params.map(p => p.name).join(', ')}):\n${body}`;
    }
}
function renderClass(unit, lang, opts, srcLang) {
    const name = unit.name;
    const src = srcLang ?? unit.sourceLanguage ?? 'javascript';
    const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', src, lang);
    const childrenStr = unit.children.map(c => '    ' + renderFunction(c, lang, opts, src)).join('\n\n');
    const innerBody = childrenStr || body;
    switch (lang) {
        case 'javascript':
            return `class ${name} {\n${innerBody}\n}`;
        case 'typescript':
            return `class ${name} {\n${innerBody}\n}`;
        case 'python':
            return `class ${name}:\n${innerBody || '    pass'}`;
        case 'java':
            return `public class ${name} {\n${innerBody}\n}`;
        case 'kotlin':
            return `class ${name} {\n${innerBody}\n}`;
        case 'csharp':
            return `public class ${name}\n{\n${innerBody}\n}`;
        case 'go':
            return `type ${name} struct {\n}\n\n${innerBody}`;
        case 'rust':
            return `pub struct ${name} {\n}\n\nimpl ${name} {\n${innerBody}\n}`;
        case 'swift':
            return `class ${name} {\n${innerBody}\n}`;
        default:
            return `// class ${name}\n${innerBody}`;
    }
}
function mapType(t, lang) {
    const n = t.name;
    const TYPE_MAP = {
        'String': { javascript: 'string', typescript: 'string', python: 'str', java: 'String', kotlin: 'String', csharp: 'string', go: 'string', rust: 'String', swift: 'String', dart: 'String' },
        'Int': { javascript: 'number', typescript: 'number', python: 'int', java: 'int', kotlin: 'Int', csharp: 'int', go: 'int', rust: 'i64', swift: 'Int', dart: 'int' },
        'Float': { javascript: 'number', typescript: 'number', python: 'float', java: 'double', kotlin: 'Double', csharp: 'double', go: 'float64', rust: 'f64', swift: 'Double', dart: 'double' },
        'Bool': { javascript: 'boolean', typescript: 'boolean', python: 'bool', java: 'boolean', kotlin: 'Boolean', csharp: 'bool', go: 'bool', rust: 'bool', swift: 'Bool', dart: 'bool' },
        'void': { javascript: 'void', typescript: 'void', python: 'None', java: 'void', kotlin: 'Unit', csharp: 'void', go: '', rust: '()', swift: 'Void', dart: 'void' },
        'Any': { javascript: 'any', typescript: 'unknown', python: 'Any', java: 'Object', kotlin: 'Any?', csharp: 'object', go: 'any', rust: 'Box<dyn Any>', swift: 'Any', dart: 'dynamic' },
        'Unknown': { javascript: 'any', typescript: 'unknown', python: 'Any', java: 'Object', kotlin: 'Any?', csharp: 'object', go: 'any', rust: 'Box<dyn Any>', swift: 'Any', dart: 'dynamic' },
        'number': { javascript: 'number', typescript: 'number', python: 'int', java: 'int', kotlin: 'Int', csharp: 'int', go: 'int', rust: 'i64', swift: 'Int', dart: 'int' },
        'boolean': { javascript: 'boolean', typescript: 'boolean', python: 'bool', java: 'boolean', kotlin: 'Boolean', csharp: 'bool', go: 'bool', rust: 'bool', swift: 'Bool', dart: 'bool' },
        'string': { javascript: 'string', typescript: 'string', python: 'str', java: 'String', kotlin: 'String', csharp: 'string', go: 'string', rust: 'String', swift: 'String', dart: 'String' },
    };
    const mapped = TYPE_MAP[n]?.[lang];
    if (mapped !== undefined) {
        return mapped;
    }
    if (t.isArray) {
        return wrapArray(n, lang);
    }
    return n; // pass through unknown type names
}
function wrapArray(base, lang) {
    switch (lang) {
        case 'java': return `List<${base}>`;
        case 'csharp': return `List<${base}>`;
        case 'typescript': return `${base}[]`;
        case 'rust': return `Vec<${base}>`;
        case 'go': return `[]${base}`;
        case 'python': return `list[${base}]`;
        case 'kotlin': return `List<${base}>`;
        case 'swift': return `[${base}]`;
        default: return `Array<${base}>`;
    }
}
function toSnakeCase(s) {
    return s.replace(/([A-Z])/g, c => '_' + c.toLowerCase()).replace(/^_/, '');
}
//# sourceMappingURL=CFamilyHandler.js.map