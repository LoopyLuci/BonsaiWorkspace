// Shell-Family Handler — Bash, PowerShell, Fish, Zsh, Batch, Makefile, Dockerfile
import {
    ULIRModule, ULIRUnit, ULIRParam, ULIRImport, ULIRMetadata,
    VOID_TYPE, STRING_TYPE, ANY_TYPE, BOOL_TYPE, UNKNOWN_TYPE,
    DEFAULT_OPTIONS, ConversionOptions,
} from '../ULIR';
import { getLang } from '../LanguageRegistry';
import { translateBody } from '../BodyTranslator';

// ─── Parse ────────────────────────────────────────────────────────────────────

export function parseShellFamily(source: string, langId: string): ULIRModule {
    const lang = getLang(langId);
    const lines = source.split('\n');
    const units: ULIRUnit[] = [];
    const imports: ULIRImport[] = [];

    // Source/dot imports (Bash)
    for (const line of lines) {
        const m = line.trim().match(/^(?:source|\.\s+)(.+)$/);
        if (m) {
            imports.push({ path: m[1].trim().replace(/^["']|["']$/g, ''), alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'relative', originalSyntax: line.trim() });
        }
        // PowerShell: Import-Module
        const psm = line.trim().match(/^Import-Module\s+([\w.-]+)/i);
        if (psm) {
            imports.push({ path: psm[1], alias: undefined, names: [], isDefault: false, isWildcard: true, kind: 'external', originalSyntax: line.trim() });
        }
    }

    // Extract function definitions
    if (langId === 'bash' || langId === 'zsh' || langId === 'fish') {
        extractBashFunctions(source, lines, units, langId);
    } else if (langId === 'powershell') {
        extractPowerShellFunctions(source, lines, units);
    } else if (langId === 'makefile') {
        extractMakeTargets(source, lines, units);
    } else if (langId === 'docker') {
        extractDockerStages(source, lines, units);
    }

    const meta: ULIRMetadata = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['scripting', 'imperative'],
        typeSystem: lang?.typing ?? 'none',
        memoryModel: lang?.memory ?? 'none',
        usesAsync: /\basync\b|&$|Start-Job|spawn/.test(source),
        usesGenerics: false, usesReflection: false, usesMetaprogramming: false,
        hasTests: /\btest\b|\bassert\b|Pester|bats/.test(source),
        hasUI: false,
        hasSideEffects: true, // scripts are inherently side-effectful
        entryPoint: 'main',
    };

    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'shell',
        units,
        imports,
        exports: extractExports(source, langId),
        docComment: '',
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes: langId === 'batch' ? ['Batch commands are case-insensitive; generated code may need manual review'] : [],
    };
}

function extractBashFunctions(source: string, lines: string[], units: ULIRUnit[], langId: string): void {
    // Bash: function name { ... } or name() { ... }
    // Fish: function name ... end
    const pattern = langId === 'fish'
        ? /^function\s+(\w[\w-]*)/m
        : /^(?:function\s+)?(\w[\w:-]*)\s*\(\s*\)\s*\{|^function\s+(\w[\w:-]*)\s*\{/m;

    for (let i = 0; i < lines.length; i++) {
        const m = lines[i].match(langId === 'fish'
            ? /^function\s+(\w[\w-]*)/
            : /^(?:function\s+)?(\w[\w:-]*)\s*(?:\(\s*\))?\s*\{/);
        if (!m) { continue; }
        const name = m[1] ?? m[2];
        if (!name || ['if', 'while', 'for', 'case'].includes(name)) { continue; }

        // Find end of function
        let depth = 0;
        let end = i;
        for (let j = i; j < lines.length; j++) {
            const l = lines[j];
            if (langId === 'fish') {
                if (l.trim() === 'end') { end = j; break; }
            } else {
                for (const ch of l) {
                    if (ch === '{') { depth++; }
                    if (ch === '}') { depth--; }
                }
                if (depth <= 0 && j > i) { end = j; break; }
            }
        }

        // Detect positional params ($1, $2, ...)
        const body = lines.slice(i, end + 1).join('\n');
        const usedParams = [...new Set([...body.matchAll(/\$(\d+)/g)].map(m => parseInt(m[1])))].sort();
        const params: ULIRParam[] = usedParams.map(n => ({
            name: `arg${n}`, type: STRING_TYPE, defaultValue: undefined,
            isVariadic: false, isKeyword: false, isRef: false, isMut: false,
        }));

        units.push({
            kind: 'function', name,
            visibility: name.startsWith('_') ? 'private' : 'public',
            signature: { params, returns: VOID_TYPE, throws: [] },
            body: [],
            attributes: [],
            docComment: extractBashDoc(lines, i),
            sourceLines: [i, end],
            isAsync: /\s*&$/.test(body) || /wait\b/.test(body),
            isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: body,
            confidence: 'medium',
        });
    }
}

function extractPowerShellFunctions(source: string, lines: string[], units: ULIRUnit[]): void {
    for (let i = 0; i < lines.length; i++) {
        const m = lines[i].match(/^function\s+([\w-]+)\s*(?:\(([^)]*)\))?\s*\{?/i);
        if (!m) { continue; }
        const name = m[1];
        const paramMatch = source.match(new RegExp(`function\\s+${name}[^{]*\\{[^}]*param\\s*\\(([^)]+)\\)`, 's'));
        const params: ULIRParam[] = paramMatch
            ? paramMatch[1].split(',').map((p): ULIRParam | null => {
                const n = p.trim().match(/\$(\w+)/);
                return n ? { name: n[1], type: STRING_TYPE, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false } : null;
            }).filter((p): p is ULIRParam => p !== null)
            : [];

        units.push({
            kind: 'function', name,
            visibility: 'public',
            signature: { params, returns: VOID_TYPE, throws: [] },
            body: [], attributes: [],
            docComment: '',
            sourceLines: [i, i],
            isAsync: false, isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: lines[i],
            confidence: 'medium',
        });
    }
}

function extractMakeTargets(source: string, lines: string[], units: ULIRUnit[]): void {
    for (let i = 0; i < lines.length; i++) {
        const m = lines[i].match(/^([\w-]+)\s*:/);
        if (!m || lines[i].startsWith('\t')) { continue; }
        const name = m[1];
        if (name === '.PHONY' || name.startsWith('.')) { continue; }
        units.push({
            kind: 'function', name,
            visibility: 'public',
            signature: { params: [], returns: VOID_TYPE, throws: [] },
            body: [], attributes: [], docComment: '',
            sourceLines: [i, i],
            isAsync: false, isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: lines[i],
            confidence: 'high',
        });
    }
}

function extractDockerStages(source: string, lines: string[], units: ULIRUnit[]): void {
    for (let i = 0; i < lines.length; i++) {
        const m = lines[i].match(/^FROM\s+\S+(?:\s+AS\s+(\w+))?/i);
        if (!m) { continue; }
        const name = m[1] ?? `stage${i}`;
        units.push({
            kind: 'function', name,
            visibility: 'public',
            signature: { params: [], returns: VOID_TYPE, throws: [] },
            body: [], attributes: [], docComment: '',
            sourceLines: [i, i],
            isAsync: false, isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: lines[i],
            confidence: 'high',
        });
    }
}

function extractBashDoc(lines: string[], fnLine: number): string {
    const docs: string[] = [];
    for (let i = fnLine - 1; i >= 0; i--) {
        const t = lines[i].trim();
        if (t.startsWith('#')) { docs.unshift(t.replace(/^#+\s?/, '')); }
        else if (t) { break; }
    }
    return docs.join('\n');
}

function extractExports(source: string, langId: string): string[] {
    const exports: string[] = [];
    for (const m of source.matchAll(/^export\s+(?:function\s+)?(\w+)|^export\s+-f\s+(\w+)/gm)) {
        exports.push(m[1] ?? m[2]);
    }
    return exports;
}

function detectModuleName(source: string, langId: string): string {
    const m = source.match(/^#\s*([A-Z][\w ]+)$/m);
    if (m) { return m[1].replace(/\s+/g, '_'); }
    if (langId === 'makefile') { return 'Makefile'; }
    if (langId === 'docker') { return 'Dockerfile'; }
    return 'Script';
}

// ─── Generate ─────────────────────────────────────────────────────────────────

export function generateShellFamily(ir: ULIRModule, targetLangId: string, opts: ConversionOptions = DEFAULT_OPTIONS): string {
    const lang = getLang(targetLangId);
    const lines: string[] = [];

    const comment = lang?.comment?.line ?? '#';
    lines.push(`${comment} ${ir.name} — Converted to ${lang?.name ?? targetLangId}`);
    lines.push(`${comment} Source: ${ir.sourceLanguage}`);
    lines.push('');

    // Shebang
    const shebang = getShebang(targetLangId);
    if (shebang) { lines.unshift(shebang, ''); }

    // Strict mode
    const strict = getStrictMode(targetLangId);
    if (strict) { lines.push(strict, ''); }

    // Imports
    for (const imp of ir.imports) {
        lines.push(renderImport(imp, targetLangId));
    }
    if (ir.imports.length > 0) { lines.push(''); }

    // Functions
    for (const unit of ir.units) {
        lines.push(renderUnit(unit, targetLangId, opts, ir.sourceLanguage));
        lines.push('');
    }

    // Entry point call
    if (targetLangId === 'bash' || targetLangId === 'zsh') {
        lines.push('main "$@"');
    }

    return lines.join('\n');
}

function getShebang(lang: string): string {
    switch (lang) {
        case 'bash': return '#!/usr/bin/env bash';
        case 'zsh': return '#!/usr/bin/env zsh';
        case 'fish': return '#!/usr/bin/env fish';
        case 'python': return '#!/usr/bin/env python3';
        case 'ruby': return '#!/usr/bin/env ruby';
        case 'perl': return '#!/usr/bin/env perl';
        case 'lua': return '#!/usr/bin/env lua';
        default: return '';
    }
}

function getStrictMode(lang: string): string {
    switch (lang) {
        case 'bash': return 'set -euo pipefail';
        case 'zsh': return 'setopt ERR_EXIT PIPE_FAIL NOUNSET';
        case 'perl': return 'use strict;\nuse warnings;';
        case 'powershell': return 'Set-StrictMode -Version Latest\n$ErrorActionPreference = "Stop"';
        default: return '';
    }
}

function renderImport(imp: ULIRImport, lang: string): string {
    switch (lang) {
        case 'bash':
        case 'zsh': return `source "${imp.path}"`;
        case 'fish': return `source ${imp.path}`;
        case 'powershell': return `Import-Module '${imp.path}'`;
        case 'perl': return `use ${imp.path};`;
        default: return `# import: ${imp.path}`;
    }
}

function renderUnit(unit: ULIRUnit, lang: string, opts: ConversionOptions, srcLang?: string): string {
    const name = unit.name;
    const params = unit.signature.params;
    const src = srcLang ?? unit.sourceLanguage ?? 'bash';
    const body = translateBody(unit.originalSource ?? '', src, lang, '  ');

    switch (lang) {
        case 'bash':
        case 'zsh': {
            const paramDocs = params.map((p, i) => `  local ${p.name}=$${i + 1}`).join('\n');
            return `function ${name}() {\n${paramDocs ? paramDocs + '\n' : ''}${body}\n}`;
        }
        case 'fish': {
            const argDocs = params.map((p, i) => `  set ${p.name} $argv[${i + 1}]`).join('\n');
            return `function ${name}\n${argDocs ? argDocs + '\n' : ''}${body}\nend`;
        }
        case 'powershell': {
            const paramBlock = params.length > 0
                ? `  param(\n${params.map(p => `    [string]$${p.name}`).join(',\n')}\n  )\n`
                : '';
            return `function ${name} {\n${paramBlock}${body}\n}`;
        }
        case 'batch': {
            const paramDocs = params.map((p, i) => `  SET "${p.name}=%~${i + 1}"`).join('\r\n');
            return `:${name}\n${paramDocs ? paramDocs + '\r\n' : ''}${body}\n  goto :EOF`;
        }
        case 'makefile':
            return `${name}:\n\t@echo "Running ${name}"\n${body.replace(/^  /gm, '\t')}`;
        default:
            return `# ${name}\n${body}`;
    }
}
