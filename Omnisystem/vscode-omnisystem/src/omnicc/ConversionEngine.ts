// OmniCC ConversionEngine — Main orchestrator for universal language conversion.
// Routes code through the language detection → parsing → ULIR → generation pipeline.
// Supports 1000+ languages via family-based handlers and deep Widget Bridge integration.

import {
    ULIRModule, OmniCCConversionRequest, OmniCCConversionResult,
    ConversionOptions, DEFAULT_OPTIONS, ProjectFile, ProjectFileResult,
} from './ULIR';
import { detectLanguage, detectLanguageBatch } from './LanguageDetector';
import { getLang, getLangByExtension, getLangsByFamily, allLanguages } from './LanguageRegistry';

import { parseCFamily, generateCFamily } from './families/CFamilyHandler';
import { parsePythonFamily, generatePythonFamily } from './families/PythonFamilyHandler';
import { parseFunctionalFamily, generateFunctionalFamily } from './families/FunctionalFamilyHandler';
import { parseShellFamily, generateShellFamily } from './families/ShellFamilyHandler';
import { parseDataFamily, generateDataFamily } from './families/DataFamilyHandler';
import { parseWebFamily, generateWebFamily } from './families/WebFamilyHandler';
import { parseSystemsFamily, generateSystemsFamily } from './families/SystemsFamilyHandler';
import { parseOmniLanguage, generateOmniLanguage } from './families/OmniLanguageHandler';

import { runWidgetBridge, mergeWidgetBridgeResults, buildBridgeSummary, WidgetBridgeSummary } from './WidgetBridge';
import { LanguageFamily } from './ULIR';

// ─── Engine ───────────────────────────────────────────────────────────────────

export class OmniCCConversionEngine {
    private opts: ConversionOptions;

    constructor(opts: Partial<ConversionOptions> = {}) {
        this.opts = { ...DEFAULT_OPTIONS, ...opts };
    }

    // ─── Main entry point ─────────────────────────────────────────────────────

    convert(req: OmniCCConversionRequest): OmniCCConversionResult {
        const startMs = Date.now();

        if (req.projectMode && req.projectFiles && req.projectFiles.length > 0) {
            return this.convertProject(req, startMs);
        }

        return this.convertSnippet(req, startMs);
    }

    // ─── Snippet conversion ───────────────────────────────────────────────────

    private convertSnippet(req: OmniCCConversionRequest, startMs: number): OmniCCConversionResult {
        const opts = req.options ? { ...this.opts, ...req.options } : this.opts;

        // 1. Detect source language
        const detected = detectLanguage(req.source, req.filename, req.sourceLang);
        const sourceLangId = detected.langId !== 'unknown' ? detected.langId : (req.sourceLang ?? 'unknown');
        const targetLangId = req.targetLang ?? 'javascript';

        // 2. Parse to ULIR
        const ir = this.parse(req.source, sourceLangId, opts);

        // 3. Widget Bridge — check for UI patterns
        const bridgeResult = opts.enableWidgetBridge !== false
            ? runWidgetBridge(ir, targetLangId, opts)
            : { detected: false, widgetCount: 0, widgetResults: [], mergedNotes: [], uiUnits: [] };

        const bridgeSummary = buildBridgeSummary(bridgeResult, ir);

        // 4. Generate target code
        const generated = this.generate(ir, targetLangId, opts);

        // 5. Optionally merge widget results
        const finalOutput = bridgeSummary.detected && opts.mergeWidgetBridge !== false
            ? mergeWidgetBridgeResults(generated, bridgeResult, targetLangId)
            : generated;

        // 6. Build notes
        const notes: string[] = [
            `Source: ${sourceLangId} (${detected.confidence}% confidence: ${detected.signals.slice(0, 2).join(', ')})`,
            `Target: ${targetLangId}`,
            `Units: ${ir.units.length} | Family: ${ir.sourceFamily}`,
        ];
        if (bridgeSummary.detected) {
            notes.push(`Widget Bridge: ${bridgeSummary.convertedCount}/${bridgeSummary.widgetCount} UI units converted`);
        }
        notes.push(...(ir.notes ?? []).slice(0, 3));

        return {
            success: true,
            output: finalOutput,
            sourceLanguage: sourceLangId,
            targetLanguage: targetLangId,
            sourceLangId,
            targetLangId,
            confidence: detected.confidence,
            detectionSignals: detected.signals,
            ir,
            notes,
            widgetResults: bridgeResult.widgetResults,
            widgetBridge: bridgeSummary,
            durationMs: Date.now() - startMs,
            linesConverted: req.source.split('\n').length,
        };
    }

    // ─── Project conversion ────────────────────────────────────────────────────

    private convertProject(req: OmniCCConversionRequest, startMs: number): OmniCCConversionResult {
        const opts = req.options ? { ...this.opts, ...req.options } : this.opts;
        const files = req.projectFiles!;
        const targetLangId = req.targetLang ?? 'javascript';

        // Detect all languages in parallel (batch)
        const detectionMap = detectLanguageBatch(
            files.map(f => ({ path: f.path, content: f.content }))
        );

        const projectResults: ProjectFileResult[] = [];
        let totalLines = 0;

        for (const file of files) {
            const fileStart = Date.now();
            const detection = detectionMap.get(file.path) ?? detectLanguage(file.content, file.path);
            const srcId = detection.langId !== 'unknown' ? detection.langId : 'unknown';

            let output = '';
            let error: string | undefined;
            let ir: ULIRModule | undefined;

            try {
                ir = this.parse(file.content, srcId, opts);
                output = this.generate(ir, targetLangId, opts);
            } catch (e) {
                error = String(e);
                output = `// Conversion failed for ${file.path}: ${error}`;
            }

            const linesIn = file.content.split('\n').length;
            totalLines += linesIn;

            projectResults.push({
                path: file.path,
                targetPath: deriveTargetPath(file.path, targetLangId),
                sourceLangId: srcId,
                targetLangId,
                output,
                linesIn,
                linesOut: output.split('\n').length,
                durationMs: Date.now() - fileStart,
                confidence: detection.confidence,
                error,
                ir,
            });
        }

        const successCount = projectResults.filter(r => !r.error).length;
        const notes = [
            `Project: ${files.length} files, ${successCount} succeeded`,
            `Total lines: ${totalLines.toLocaleString()}`,
            `Target: ${targetLangId}`,
            `Duration: ${Date.now() - startMs}ms`,
        ];

        return {
            success: successCount > 0,
            output: projectResults.map(r => `// File: ${r.path}\n${r.output}`).join('\n\n---\n\n'),
            sourceLanguage: req.sourceLang ?? 'mixed',
            targetLanguage: targetLangId,
            sourceLangId: req.sourceLang ?? 'mixed',
            targetLangId,
            confidence: projectResults.reduce((a, r) => a + r.confidence, 0) / Math.max(projectResults.length, 1),
            detectionSignals: [],
            ir: undefined,
            notes,
            widgetResults: [],
            durationMs: Date.now() - startMs,
            linesConverted: totalLines,
            projectResults,
        };
    }

    // ─── Parse dispatch ───────────────────────────────────────────────────────

    parse(source: string, langId: string, opts: ConversionOptions = this.opts): ULIRModule {
        const family = getLanguageFamily(langId);
        switch (family) {
            case 'c-family':      return parseCFamily(source, langId);
            case 'python-family': return parsePythonFamily(source, langId);
            case 'functional':    return parseFunctionalFamily(source, langId);
            case 'lisp':          return parseFunctionalFamily(source, langId);
            case 'shell':         return parseShellFamily(source, langId);
            case 'data':          return parseDataFamily(source, langId);
            case 'query':         return parseDataFamily(source, langId);
            case 'web':           return parseWebFamily(source, langId);
            case 'systems':       return parseSystemsFamily(source, langId);
            case 'omni':          return parseOmniLanguage(source, langId);
            default:              return parseWithBestEffort(source, langId, family);
        }
    }

    // ─── Generate dispatch ────────────────────────────────────────────────────

    generate(ir: ULIRModule, targetLangId: string, opts: ConversionOptions = this.opts): string {
        const family = getLanguageFamily(targetLangId);
        switch (family) {
            case 'c-family':      return generateCFamily(ir, targetLangId, opts);
            case 'python-family': return generatePythonFamily(ir, targetLangId, opts);
            case 'functional':    return generateFunctionalFamily(ir, targetLangId, opts);
            case 'lisp':          return generateFunctionalFamily(ir, targetLangId, opts);
            case 'shell':         return generateShellFamily(ir, targetLangId, opts);
            case 'data':          return generateDataFamily(ir, targetLangId, opts);
            case 'query':         return generateDataFamily(ir, targetLangId, opts);
            case 'web':           return generateWebFamily(ir, targetLangId, opts);
            case 'systems':       return generateSystemsFamily(ir, targetLangId, opts);
            case 'omni':          return generateOmniLanguage(ir, targetLangId, opts);
            default:              return generateBestEffort(ir, targetLangId, opts);
        }
    }

    // ─── Introspection ─────────────────────────────────────────────────────────

    getSupportedLanguages(): Array<{ id: string; name: string; family: string; canParse: boolean; canGenerate: boolean }> {
        return allLanguages().map(lang => ({
            id: lang.id,
            name: lang.name,
            family: lang.family,
            canParse: true,   // all languages routed through family handlers
            canGenerate: true, // all families have generators
        }));
    }

    getConversionPaths(sourceLangId: string): string[] {
        // Returns list of target languages for a given source language
        return allLanguages().map(l => l.id).filter(id => id !== sourceLangId);
    }
}

// ─── Family resolution ─────────────────────────────────────────────────────────

function getLanguageFamily(langId: string): LanguageFamily {
    const lang = getLang(langId);
    if (lang) { return lang.family; }

    // Fallback family guesses by common IDs
    const C_FAMILY = new Set(['javascript', 'typescript', 'java', 'kotlin', 'csharp', 'cs', 'cpp', 'c++', 'dart', 'scala', 'groovy', 'swift', 'php', 'objective-c', 'actionscript', 'coffeescript']);
    const PY_FAMILY = new Set(['python', 'ruby', 'lua', 'perl', 'r', 'julia', 'nim', 'crystal', 'elixir', 'erlang', 'groovy', 'python3']);
    const FUNCTIONAL = new Set(['haskell', 'ocaml', 'fsharp', 'elm', 'purescript', 'idris', 'agda', 'coq', 'lean']);
    const LISP = new Set(['commonlisp', 'scheme', 'racket', 'clojure', 'clojurescript', 'janet', 'hy', 'arc', 'chicken']);
    const SHELL = new Set(['bash', 'sh', 'zsh', 'fish', 'powershell', 'batch', 'bat', 'cmd', 'makefile', 'dockerfile', 'docker', 'nushell']);
    const DATA = new Set(['json', 'yaml', 'yml', 'toml', 'xml', 'csv', 'tsv', 'ini', 'cfg', 'properties', 'dotenv', 'graphql', 'gql', 'protobuf', 'proto', 'hcl', 'tf', 'avro', 'parquet', 'flatbuf']);
    const QUERY = new Set(['sql', 'mysql', 'postgresql', 'sqlite', 'mssql', 'oracle', 'cassandra', 'mongodb', 'redis', 'influxql', 'prql', 'surrealql', 'cypher', 'sparql', 'xquery']);
    const WEB = new Set(['html', 'htm', 'css', 'scss', 'sass', 'less', 'jsx', 'tsx', 'vue', 'svelte', 'angular', 'astro', 'mdx', 'pug', 'haml', 'ejs', 'handlebars', 'mustache', 'twig', 'jinja', 'jinja2', 'liquid', 'razor', 'htmx']);
    const SYSTEMS = new Set(['rust', 'zig', 'go', 'golang', 'odin', 'c', 'cpp', 'asm', 'asm-x86', 'asm-arm', 'wasm', 'wat', 'v', 'd', 'ada', 'cobol', 'fortran', 'pascal', 'delphi', 'forth', 'fasm', 'nasm', 'gas']);
    const OMNI = new Set(['titan', 'vera', 'nexus', 'helix', 'aether', 'axiom', 'sylva']);

    if (OMNI.has(langId)) { return 'omni'; }
    if (WEB.has(langId)) { return 'web'; }
    if (SYSTEMS.has(langId)) { return 'systems'; }
    if (QUERY.has(langId)) { return 'query'; }
    if (DATA.has(langId)) { return 'data'; }
    if (LISP.has(langId)) { return 'lisp'; }
    if (FUNCTIONAL.has(langId)) { return 'functional'; }
    if (SHELL.has(langId)) { return 'shell'; }
    if (PY_FAMILY.has(langId)) { return 'python-family'; }
    if (C_FAMILY.has(langId)) { return 'c-family'; }

    return 'unknown';
}

// ─── Best-effort fallbacks for unknown/rare languages ─────────────────────────

function parseWithBestEffort(source: string, langId: string, family: LanguageFamily): ULIRModule {
    // Try each handler in order of likelihood
    const source8k = source.slice(0, 8000);
    if (/def\s+\w+\s*\(|class\s+\w+:/.test(source8k)) {
        return parsePythonFamily(source, langId);
    }
    if (/function\s+\w+\s*\(|const\s+\w+\s*=/.test(source8k)) {
        return parseCFamily(source, langId);
    }
    if (/fn\s+\w+\s*\(|struct\s+\w+/.test(source8k)) {
        return parseSystemsFamily(source, langId);
    }
    if (/\w+\s*::\s*(?:String|Int|Bool|IO)|let\s+\w+\s*=/.test(source8k)) {
        return parseFunctionalFamily(source, langId);
    }
    if (/SELECT|INSERT|CREATE TABLE/i.test(source8k)) {
        return parseDataFamily(source, langId);
    }
    if (/<\w+[\s>]/.test(source8k)) {
        return parseWebFamily(source, langId);
    }
    // Ultra-fallback: empty ULIR module
    return {
        name: 'UnknownModule',
        sourceLanguage: langId,
        sourceFamily: family,
        units: [],
        imports: [],
        exports: [],
        docComment: '',
        metadata: {
            sourceLines: source.split('\n').length,
            paradigms: ['unknown'],
            typeSystem: 'unknown',
            memoryModel: 'unknown',
            usesAsync: false, usesGenerics: false, usesReflection: false,
            usesMetaprogramming: false, hasTests: false, hasUI: false, hasSideEffects: false,
        },
        confidence: 'low',
        notes: [`Language ${langId} not fully supported; best-effort parse attempted`],
    };
}

function generateBestEffort(ir: ULIRModule, targetLangId: string, opts: ConversionOptions): string {
    // Map unknown targets to closest known family
    const targetFamily = getLanguageFamily(targetLangId);

    // Try to route via closest known handler
    if (targetFamily !== 'unknown') {
        switch (targetFamily) {
            case 'c-family': return generateCFamily(ir, 'javascript', opts);
            case 'python-family': return generatePythonFamily(ir, 'python', opts);
            case 'systems': return generateSystemsFamily(ir, 'c', opts);
            case 'web': return generateWebFamily(ir, 'html', opts);
            case 'data': return generateDataFamily(ir, 'json', opts);
            case 'shell': return generateShellFamily(ir, 'bash', opts);
            case 'functional': return generateFunctionalFamily(ir, 'haskell', opts);
            case 'omni': return generateOmniLanguage(ir, 'titan', opts);
        }
    }

    // Final fallback: pseudocode comment structure
    const comment = '//' ;
    const lines = [
        `${comment} ${ir.name} — Converted to ${targetLangId}`,
        `${comment} Source: ${ir.sourceLanguage} | Family: ${ir.sourceFamily}`,
        `${comment} Note: ${targetLangId} is not directly supported. Showing pseudocode.`,
        '',
    ];
    for (const imp of ir.imports) {
        lines.push(`${comment} import: ${imp.path} (${imp.names.join(', ') || '*'})`);
    }
    if (ir.imports.length > 0) { lines.push(''); }
    for (const unit of ir.units) {
        const params = unit.signature.params.map(p => `${p.name}: ${p.type.name}`).join(', ');
        lines.push(`${comment} ${unit.kind} ${unit.name}(${params})`);
        if (unit.docComment) { lines.push(`${comment}   ${unit.docComment}`); }
        lines.push('');
    }
    return lines.join('\n');
}

// ─── Target path derivation ───────────────────────────────────────────────────

function deriveTargetPath(sourcePath: string, targetLangId: string): string {
    const ext = getExtForLang(targetLangId);
    // Replace extension
    const dotIdx = sourcePath.lastIndexOf('.');
    if (dotIdx >= 0) {
        return sourcePath.slice(0, dotIdx) + ext;
    }
    return sourcePath + ext;
}

function getExtForLang(langId: string): string {
    const EXT: Record<string, string> = {
        javascript: '.js', typescript: '.ts', python: '.py', java: '.java',
        kotlin: '.kt', csharp: '.cs', go: '.go', rust: '.rs', swift: '.swift',
        dart: '.dart', scala: '.scala', ruby: '.rb', lua: '.lua', perl: '.pl',
        php: '.php', r: '.r', julia: '.jl', elixir: '.ex', erlang: '.erl',
        haskell: '.hs', ocaml: '.ml', fsharp: '.fs', clojure: '.clj',
        bash: '.sh', powershell: '.ps1', fish: '.fish', zsh: '.zsh',
        sql: '.sql', json: '.json', yaml: '.yml', toml: '.toml', xml: '.xml',
        graphql: '.graphql', protobuf: '.proto', hcl: '.tf',
        html: '.html', css: '.css', scss: '.scss', jsx: '.jsx', tsx: '.tsx',
        vue: '.vue', svelte: '.svelte', astro: '.astro',
        c: '.c', cpp: '.cpp', zig: '.zig', odin: '.odin', v: '.v', d: '.d',
        titan: '.titan', vera: '.vera', nexus: '.nexus', helix: '.helix',
        aether: '.aether', axiom: '.axiom', sylva: '.sylva',
    };
    return EXT[langId] ?? '.txt';
}

// ─── Convenience factory ──────────────────────────────────────────────────────

export function createEngine(opts?: Partial<ConversionOptions>): OmniCCConversionEngine {
    return new OmniCCConversionEngine(opts);
}

export function quickConvert(
    source: string,
    targetLang: string,
    sourceLang?: string,
    filename?: string,
): OmniCCConversionResult {
    const engine = new OmniCCConversionEngine();
    return engine.convert({ source, targetLang, sourceLang, filename });
}
