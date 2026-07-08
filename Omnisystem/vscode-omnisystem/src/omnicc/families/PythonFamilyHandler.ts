// Python-Family Handler — Python, Ruby, Lua, Perl, PHP, Nim, Crystal, Elixir, Erlang, R, Julia, Groovy
import {
    ULIRModule, ULIRUnit, ULIRParam, ULIRType, ULIRImport, ULIRMetadata,
    STRING_TYPE, VOID_TYPE, INT_TYPE, FLOAT_TYPE, BOOL_TYPE, ANY_TYPE, UNKNOWN_TYPE,
    DEFAULT_OPTIONS, ConversionOptions,
} from '../ULIR';
import { getLang } from '../LanguageRegistry';
import { translateBody } from '../BodyTranslator';

// ─── Parse ────────────────────────────────────────────────────────────────────

export function parsePythonFamily(source: string, langId: string): ULIRModule {
    const lang = getLang(langId);
    const lines = source.split('\n');
    const units: ULIRUnit[] = [];
    const imports: ULIRImport[] = [];
    const notes: string[] = [];

    // Extract imports
    for (const line of lines) {
        const imp = extractImport(line.trim(), langId);
        if (imp) { imports.push(imp); }
    }

    // Extract function/method definitions
    const fnMap = buildBlockMap(lines, langId);
    for (const fn of fnMap) {
        units.push(fn);
    }

    const hasUI = /tkinter|PyQt|wx\.|kivy|gtk|pygame/.test(source);
    const meta: ULIRMetadata = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['scripting', 'oop'],
        typeSystem: lang?.typing ?? 'dynamic-duck',
        memoryModel: lang?.memory ?? 'gc',
        usesAsync: /\basync\s+def\b|\bawait\b|asyncio/.test(source),
        usesGenerics: false,
        usesReflection: /\bgetattr\b|\bhasattr\b|\bdir\(/.test(source),
        usesMetaprogramming: /@\w+|__\w+__/.test(source),
        hasTests: /\btest_\w+|\bunittest\b|\bpytest\b/.test(source),
        hasUI,
        hasSideEffects: /\bprint\b|\bopen\(|\bwrite\(/.test(source),
        entryPoint: /if\s+__name__\s*==\s*['"]__main__['"]/.test(source) ? '__main__' : undefined,
    };

    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'python-family',
        units,
        imports,
        exports: [],
        docComment: '',
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes,
    };
}

function buildBlockMap(lines: string[], langId: string): ULIRUnit[] {
    const units: ULIRUnit[] = [];
    let i = 0;
    while (i < lines.length) {
        const line = lines[i];
        // Python def / async def
        const defMatch = line.match(/^(\s*)(?:(async)\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([\w\[\], |]+))?:/);
        if (defMatch && langId === 'python') {
            const indent = defMatch[1].length;
            const isAsync = !!defMatch[2];
            const name = defMatch[3];
            const paramStr = defMatch[4];
            const retStr = defMatch[5];
            const blockLines = [line];
            i++;
            while (i < lines.length) {
                const l = lines[i];
                if (l.trim() === '' || (l.match(/^\s+/) && l.match(/^\s+/)![0].length > indent)) {
                    blockLines.push(l);
                    i++;
                } else { break; }
            }
            units.push({
                kind: 'function',
                name,
                visibility: name.startsWith('_') ? 'private' : 'public',
                signature: {
                    params: parsePyParams(paramStr),
                    returns: retStr ? { ...UNKNOWN_TYPE, name: retStr, originalSrc: retStr } : UNKNOWN_TYPE,
                    throws: [],
                },
                body: [],
                attributes: extractDecorators(lines, i - blockLines.length),
                docComment: extractDocstring(blockLines),
                sourceLines: [i - blockLines.length, i],
                isAsync,
                isStatic: paramStr.startsWith('cls') || !paramStr.startsWith('self'),
                isAbstract: /@abstractmethod/.test(blockLines[0] ?? ''),
                isFinal: false, isOverride: false, isExtern: false,
                generics: [], extends_: [], implements_: [], children: [],
                originalSource: blockLines.join('\n'),
                confidence: 'high',
            });
            continue;
        }
        // Python class
        const classMatch = line.match(/^class\s+(\w+)(?:\(([^)]*)\))?:/);
        if (classMatch && langId === 'python') {
            units.push({
                kind: 'class',
                name: classMatch[1],
                visibility: 'public',
                signature: { params: [], returns: VOID_TYPE, throws: [] },
                body: [],
                attributes: [],
                docComment: '',
                sourceLines: [i, i],
                isAsync: false, isStatic: false, isAbstract: false,
                isFinal: false, isOverride: false, isExtern: false,
                generics: [],
                extends_: classMatch[2] ? classMatch[2].split(',').map(s => s.trim()) : [],
                implements_: [], children: [],
                originalSource: line,
                confidence: 'high',
            });
        }
        // Ruby def
        const rubyDef = line.match(/^\s*def\s+(\w+[?!]?)\s*(?:\(([^)]*)\))?/);
        if (rubyDef && (langId === 'ruby' || langId === 'crystal')) {
            units.push({
                kind: 'function',
                name: rubyDef[1],
                visibility: 'public',
                signature: { params: parseSimpleParams(rubyDef[2] ?? ''), returns: UNKNOWN_TYPE, throws: [] },
                body: [],
                attributes: [],
                docComment: '',
                sourceLines: [i, i],
                isAsync: false, isStatic: false, isAbstract: false,
                isFinal: false, isOverride: false, isExtern: false,
                generics: [], extends_: [], implements_: [], children: [],
                originalSource: line,
                confidence: 'medium',
            });
        }
        i++;
    }
    return units;
}

function parsePyParams(raw: string): ULIRParam[] {
    if (!raw.trim()) { return []; }
    return raw.split(',').map((p): ULIRParam | null => {
        const t = p.trim();
        if (t === 'self' || t === 'cls') { return null; }
        const m = t.match(/(\w+)\s*:\s*([\w\[\], |]+)(?:\s*=.*)?/);
        if (m) {
            return { name: m[1], type: { ...UNKNOWN_TYPE, name: m[2], originalSrc: m[2] }, defaultValue: undefined, isVariadic: t.startsWith('**'), isKeyword: t.startsWith('**'), isRef: false, isMut: false };
        }
        const name = t.replace(/[*=].*/, '').trim();
        return name ? { name, type: ANY_TYPE, defaultValue: undefined, isVariadic: t.startsWith('*'), isKeyword: false, isRef: false, isMut: false } : null;
    }).filter((p): p is ULIRParam => p !== null && p.name.length > 0);
}

function parseSimpleParams(raw: string): ULIRParam[] {
    if (!raw.trim()) { return []; }
    return raw.split(',').map(p => ({
        name: p.trim().replace(/[*&:].*/, '').trim() || 'param',
        type: ANY_TYPE,
        defaultValue: undefined,
        isVariadic: p.trim().startsWith('*'),
        isKeyword: false, isRef: false, isMut: false,
    })).filter(p => p.name.length > 0);
}

function extractImport(line: string, langId: string): ULIRImport | null {
    // Python: from x import y / import x
    let m = line.match(/^from\s+([\w.]+)\s+import\s+(.+)$/);
    if (m && langId === 'python') {
        const names = m[2] === '*' ? [] : m[2].split(',').map(n => n.trim().split(' as ')[0].trim());
        return { path: m[1].replace(/\./g, '/'), alias: undefined, names, isDefault: false, isWildcard: m[2] === '*', kind: 'package', originalSyntax: line };
    }
    m = line.match(/^import\s+([\w., ]+)$/);
    if (m && langId === 'python') {
        const first = m[1].split(',')[0].trim();
        return { path: first.replace(/\./g, '/'), alias: undefined, names: [first.split('.').pop() ?? first], isDefault: false, isWildcard: false, kind: 'package', originalSyntax: line };
    }
    // Ruby: require/require_relative
    m = line.match(/^require(?:_relative)?\s+['"]([^'"]+)['"]/);
    if (m) {
        return { path: m[1], alias: undefined, names: [], isDefault: false, isWildcard: true, kind: line.includes('relative') ? 'relative' : 'external', originalSyntax: line };
    }
    // Lua: require
    m = line.match(/require\s*\(?['"]([^'"]+)['"]\)?/);
    if (m && langId === 'lua') {
        return { path: m[1], alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'external', originalSyntax: line };
    }
    return null;
}

function extractDecorators(lines: string[], fnLine: number): string[] {
    const attrs: string[] = [];
    for (let i = fnLine - 1; i >= 0; i--) {
        const t = lines[i].trim();
        if (t.startsWith('@')) { attrs.unshift(t); }
        else if (t) { break; }
    }
    return attrs;
}

function extractDocstring(blockLines: string[]): string {
    const joined = blockLines.slice(1, 6).join('\n');
    const m = joined.match(/"""([\s\S]*?)"""|'''([\s\S]*?)'''/);
    return m ? (m[1] ?? m[2] ?? '').trim() : '';
}

function detectModuleName(source: string, langId: string): string {
    const m = source.match(/^__name__\s*=\s*['"]([^'"]+)['"]/m);
    if (m) { return m[1]; }
    const mod = source.match(/^module\s+(\w+)/m);
    if (mod) { return mod[1]; }
    return 'Module';
}

// ─── Generate ─────────────────────────────────────────────────────────────────

export function generatePythonFamily(ir: ULIRModule, targetLangId: string, opts: ConversionOptions = DEFAULT_OPTIONS): string {
    const lang = getLang(targetLangId);
    const lines: string[] = [];

    lines.push(`# ${ir.name} — Converted to ${lang?.name ?? targetLangId}`);
    lines.push(`# Source: ${ir.sourceLanguage}`);
    lines.push('');

    for (const imp of ir.imports) {
        lines.push(renderImport(imp, targetLangId));
    }
    if (ir.imports.length > 0) { lines.push(''); }

    for (const unit of ir.units) {
        lines.push(renderUnit(unit, targetLangId, opts, ir.sourceLanguage));
        lines.push('');
    }

    return lines.join('\n');
}

function renderImport(imp: ULIRImport, lang: string): string {
    switch (lang) {
        case 'python':
            if (imp.names.length > 0) { return `from ${imp.path.replace(/\//g, '.')} import ${imp.names.join(', ')}`; }
            return `import ${imp.path.replace(/\//g, '.')}`;
        case 'ruby': return `require '${imp.path}'`;
        case 'lua': return `local ${imp.path.split('/').pop()} = require('${imp.path}')`;
        case 'perl': return `use ${imp.path.replace(/\//g, '::')} qw(${imp.names.join(' ')});`;
        case 'php': return `require_once '${imp.path}.php';`;
        case 'elixir': return `alias ${imp.path.split('/').map(s => s.charAt(0).toUpperCase() + s.slice(1)).join('.')}`;
        case 'r': return `library(${imp.path.split('/').pop()})`;
        case 'julia': return `using ${imp.path.split('/').join('.')}`;
        default: return `# import: ${imp.path}`;
    }
}

function renderUnit(unit: ULIRUnit, lang: string, opts: ConversionOptions, srcLang?: string): string {
    if (unit.kind === 'class') { return renderClass(unit, lang, srcLang); }
    return renderFunction(unit, lang, opts, srcLang);
}

function renderFunction(unit: ULIRUnit, lang: string, opts: ConversionOptions, srcLang?: string): string {
    const name = unit.name;
    const params = unit.signature.params;
    const src = srcLang ?? unit.sourceLanguage ?? 'python';
    const body = translateBody(unit.originalSource ?? '', src, lang);

    switch (lang) {
        case 'python': {
            const pStr = ['self', ...params.map(p => opts.strictTypes ? `${p.name}: ${mapType(p.type, 'python')}` : p.name)].join(', ');
            const ret = opts.strictTypes && unit.signature.returns.name !== 'Unknown' ? ` -> ${mapType(unit.signature.returns, 'python')}` : '';
            const asyncPfx = unit.isAsync ? 'async ' : '';
            return `${asyncPfx}def ${name}(${pStr})${ret}:\n${body}`;
        }
        case 'ruby': {
            const pStr = params.map(p => p.name).join(', ');
            return `def ${name}(${pStr})\n${body}\nend`;
        }
        case 'lua': {
            const pStr = params.map(p => p.name).join(', ');
            return `function ${name}(${pStr})\n${body}\nend`;
        }
        case 'perl': {
            const pStr = params.map(p => `$${p.name}`).join(', ');
            return `sub ${name} {\n  my (${pStr}) = @_;\n${body}\n}`;
        }
        case 'php': {
            const pStr = params.map(p => `$${p.name}`).join(', ');
            return `function ${name}(${pStr}) {\n${body}\n}`;
        }
        case 'elixir': {
            const pStr = params.map(p => p.name).join(', ');
            return `def ${name}(${pStr}) do\n${body}\nend`;
        }
        case 'r': {
            const pStr = params.map(p => p.name).join(', ');
            return `${name} <- function(${pStr}) {\n${body}\n}`;
        }
        case 'julia': {
            const pStr = params.map(p => p.name).join(', ');
            return `function ${name}(${pStr})\n${body}\nend`;
        }
        case 'nim': {
            const pStr = params.map(p => `${p.name}: ${mapType(p.type, 'nim')}`).join(', ');
            const ret = unit.signature.returns.name !== 'Unknown' && unit.signature.returns.name !== 'void' ? `: ${mapType(unit.signature.returns, 'nim')}` : '';
            return `proc ${name}*(${pStr})${ret} =\n${body}`;
        }
        case 'crystal': {
            const pStr = params.map(p => `${p.name} : ${mapType(p.type, 'crystal')}`).join(', ');
            return `def ${name}(${pStr})\n${body}\nend`;
        }
        default:
            return `# function ${name}(${params.map(p => p.name).join(', ')})\n${body}`;
    }
}

function renderClass(unit: ULIRUnit, lang: string, srcLang?: string): string {
    const src = srcLang ?? unit.sourceLanguage ?? 'python';
    const body = translateBody(unit.originalSource ?? '', src, lang);
    switch (lang) {
        case 'python': return `class ${unit.name}:\n${body || '    pass'}`;
        case 'ruby': return `class ${unit.name}\n${body}\nend`;
        case 'elixir': return `defmodule ${unit.name} do\n${body}\nend`;
        case 'lua': return `${unit.name} = {}\n${unit.name}.__index = ${unit.name}\nfunction ${unit.name}.new()\n${body}\n  return setmetatable({}, ${unit.name})\nend`;
        case 'nim': return `type\n  ${unit.name}* = object`;
        default: return `# class ${unit.name}\n${body}`;
    }
}

function mapType(t: ULIRType, lang: string): string {
    const n = t.name;
    const TYPE_MAP: Record<string, Record<string, string>> = {
        'String':  { python: 'str', ruby: 'String', lua: 'string', elixir: 'String.t()', r: 'character', julia: 'String', nim: 'string', crystal: 'String' },
        'Int':     { python: 'int', ruby: 'Integer', lua: 'number', elixir: 'integer()', r: 'integer', julia: 'Int64', nim: 'int', crystal: 'Int32' },
        'Float':   { python: 'float', ruby: 'Float', lua: 'number', elixir: 'float()', r: 'numeric', julia: 'Float64', nim: 'float', crystal: 'Float64' },
        'Bool':    { python: 'bool', ruby: 'TrueClass | FalseClass', lua: 'boolean', elixir: 'boolean()', r: 'logical', julia: 'Bool', nim: 'bool', crystal: 'Bool' },
        'void':    { python: 'None', ruby: 'nil', lua: 'nil', elixir: ':ok', r: 'NULL', julia: 'Nothing', nim: 'void', crystal: 'Nil' },
        'Any':     { python: 'Any', ruby: 'Object', lua: 'any', elixir: 'term()', r: 'ANY', julia: 'Any', nim: 'auto', crystal: 'T' },
    };
    return TYPE_MAP[n]?.[lang] ?? n;
}
