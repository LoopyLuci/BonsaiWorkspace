"use strict";
// OmniCC ConversionEngine — Main orchestrator for universal language conversion.
// Routes code through the language detection → parsing → ULIR → generation pipeline.
// Supports 1000+ languages via family-based handlers and deep Widget Bridge integration.
Object.defineProperty(exports, "__esModule", { value: true });
exports.OmniCCConversionEngine = void 0;
exports.createEngine = createEngine;
exports.quickConvert = quickConvert;
const ULIR_1 = require("./ULIR");
const LanguageDetector_1 = require("./LanguageDetector");
const LanguageRegistry_1 = require("./LanguageRegistry");
const CFamilyHandler_1 = require("./families/CFamilyHandler");
const PythonFamilyHandler_1 = require("./families/PythonFamilyHandler");
const FunctionalFamilyHandler_1 = require("./families/FunctionalFamilyHandler");
const ShellFamilyHandler_1 = require("./families/ShellFamilyHandler");
const DataFamilyHandler_1 = require("./families/DataFamilyHandler");
const WebFamilyHandler_1 = require("./families/WebFamilyHandler");
const SystemsFamilyHandler_1 = require("./families/SystemsFamilyHandler");
const OmniLanguageHandler_1 = require("./families/OmniLanguageHandler");
const WidgetBridge_1 = require("./WidgetBridge");
// ─── Engine ───────────────────────────────────────────────────────────────────
class OmniCCConversionEngine {
    constructor(opts = {}) {
        this.opts = { ...ULIR_1.DEFAULT_OPTIONS, ...opts };
    }
    // ─── Main entry point ─────────────────────────────────────────────────────
    convert(req) {
        const startMs = Date.now();
        if (req.projectMode && req.projectFiles && req.projectFiles.length > 0) {
            return this.convertProject(req, startMs);
        }
        return this.convertSnippet(req, startMs);
    }
    // ─── Snippet conversion ───────────────────────────────────────────────────
    convertSnippet(req, startMs) {
        const opts = req.options ? { ...this.opts, ...req.options } : this.opts;
        // 1. Detect source language
        const detected = (0, LanguageDetector_1.detectLanguage)(req.source, req.filename, req.sourceLang);
        const sourceLangId = detected.langId !== 'unknown' ? detected.langId : (req.sourceLang ?? 'unknown');
        const targetLangId = req.targetLang ?? 'javascript';
        // 2. Parse to ULIR
        const ir = this.parse(req.source, sourceLangId, opts);
        // 3. Widget Bridge — check for UI patterns
        const bridgeResult = opts.enableWidgetBridge !== false
            ? (0, WidgetBridge_1.runWidgetBridge)(ir, targetLangId, opts)
            : { detected: false, widgetCount: 0, widgetResults: [], mergedNotes: [], uiUnits: [] };
        const bridgeSummary = (0, WidgetBridge_1.buildBridgeSummary)(bridgeResult, ir);
        // 4. Generate target code
        const generated = this.generate(ir, targetLangId, opts);
        // 5. Optionally merge widget results
        const finalOutput = bridgeSummary.detected && opts.mergeWidgetBridge !== false
            ? (0, WidgetBridge_1.mergeWidgetBridgeResults)(generated, bridgeResult, targetLangId)
            : generated;
        // 6. Build notes
        const notes = [
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
    convertProject(req, startMs) {
        const opts = req.options ? { ...this.opts, ...req.options } : this.opts;
        const files = req.projectFiles;
        const targetLangId = req.targetLang ?? 'javascript';
        // Detect all languages in parallel (batch)
        const detectionMap = (0, LanguageDetector_1.detectLanguageBatch)(files.map(f => ({ path: f.path, content: f.content })));
        const projectResults = [];
        let totalLines = 0;
        for (const file of files) {
            const fileStart = Date.now();
            const detection = detectionMap.get(file.path) ?? (0, LanguageDetector_1.detectLanguage)(file.content, file.path);
            const srcId = detection.langId !== 'unknown' ? detection.langId : 'unknown';
            let output = '';
            let error;
            let ir;
            try {
                ir = this.parse(file.content, srcId, opts);
                output = this.generate(ir, targetLangId, opts);
            }
            catch (e) {
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
    parse(source, langId, opts = this.opts) {
        const family = getLanguageFamily(langId);
        switch (family) {
            case 'c-family': return (0, CFamilyHandler_1.parseCFamily)(source, langId);
            case 'python-family': return (0, PythonFamilyHandler_1.parsePythonFamily)(source, langId);
            case 'functional': return (0, FunctionalFamilyHandler_1.parseFunctionalFamily)(source, langId);
            case 'lisp': return (0, FunctionalFamilyHandler_1.parseFunctionalFamily)(source, langId);
            case 'shell': return (0, ShellFamilyHandler_1.parseShellFamily)(source, langId);
            case 'data': return (0, DataFamilyHandler_1.parseDataFamily)(source, langId);
            case 'query': return (0, DataFamilyHandler_1.parseDataFamily)(source, langId);
            case 'web': return (0, WebFamilyHandler_1.parseWebFamily)(source, langId);
            case 'systems': return (0, SystemsFamilyHandler_1.parseSystemsFamily)(source, langId);
            case 'omni': return (0, OmniLanguageHandler_1.parseOmniLanguage)(source, langId);
            default: return parseWithBestEffort(source, langId, family);
        }
    }
    // ─── Generate dispatch ────────────────────────────────────────────────────
    generate(ir, targetLangId, opts = this.opts) {
        const family = getLanguageFamily(targetLangId);
        switch (family) {
            case 'c-family': return (0, CFamilyHandler_1.generateCFamily)(ir, targetLangId, opts);
            case 'python-family': return (0, PythonFamilyHandler_1.generatePythonFamily)(ir, targetLangId, opts);
            case 'functional': return (0, FunctionalFamilyHandler_1.generateFunctionalFamily)(ir, targetLangId, opts);
            case 'lisp': return (0, FunctionalFamilyHandler_1.generateFunctionalFamily)(ir, targetLangId, opts);
            case 'shell': return (0, ShellFamilyHandler_1.generateShellFamily)(ir, targetLangId, opts);
            case 'data': return (0, DataFamilyHandler_1.generateDataFamily)(ir, targetLangId, opts);
            case 'query': return (0, DataFamilyHandler_1.generateDataFamily)(ir, targetLangId, opts);
            case 'web': return (0, WebFamilyHandler_1.generateWebFamily)(ir, targetLangId, opts);
            case 'systems': return (0, SystemsFamilyHandler_1.generateSystemsFamily)(ir, targetLangId, opts);
            case 'omni': return (0, OmniLanguageHandler_1.generateOmniLanguage)(ir, targetLangId, opts);
            default: return generateBestEffort(ir, targetLangId, opts);
        }
    }
    // ─── Introspection ─────────────────────────────────────────────────────────
    getSupportedLanguages() {
        return (0, LanguageRegistry_1.allLanguages)().map(lang => ({
            id: lang.id,
            name: lang.name,
            family: lang.family,
            canParse: true, // all languages routed through family handlers
            canGenerate: true, // all families have generators
        }));
    }
    getConversionPaths(sourceLangId) {
        // Returns list of target languages for a given source language
        return (0, LanguageRegistry_1.allLanguages)().map(l => l.id).filter(id => id !== sourceLangId);
    }
}
exports.OmniCCConversionEngine = OmniCCConversionEngine;
// ─── Family resolution ─────────────────────────────────────────────────────────
function getLanguageFamily(langId) {
    const lang = (0, LanguageRegistry_1.getLang)(langId);
    if (lang) {
        return lang.family;
    }
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
    if (OMNI.has(langId)) {
        return 'omni';
    }
    if (WEB.has(langId)) {
        return 'web';
    }
    if (SYSTEMS.has(langId)) {
        return 'systems';
    }
    if (QUERY.has(langId)) {
        return 'query';
    }
    if (DATA.has(langId)) {
        return 'data';
    }
    if (LISP.has(langId)) {
        return 'lisp';
    }
    if (FUNCTIONAL.has(langId)) {
        return 'functional';
    }
    if (SHELL.has(langId)) {
        return 'shell';
    }
    if (PY_FAMILY.has(langId)) {
        return 'python-family';
    }
    if (C_FAMILY.has(langId)) {
        return 'c-family';
    }
    return 'unknown';
}
// ─── Best-effort fallbacks for unknown/rare languages ─────────────────────────
function parseWithBestEffort(source, langId, family) {
    // Try each handler in order of likelihood
    const source8k = source.slice(0, 8000);
    if (/def\s+\w+\s*\(|class\s+\w+:/.test(source8k)) {
        return (0, PythonFamilyHandler_1.parsePythonFamily)(source, langId);
    }
    if (/function\s+\w+\s*\(|const\s+\w+\s*=/.test(source8k)) {
        return (0, CFamilyHandler_1.parseCFamily)(source, langId);
    }
    if (/fn\s+\w+\s*\(|struct\s+\w+/.test(source8k)) {
        return (0, SystemsFamilyHandler_1.parseSystemsFamily)(source, langId);
    }
    if (/\w+\s*::\s*(?:String|Int|Bool|IO)|let\s+\w+\s*=/.test(source8k)) {
        return (0, FunctionalFamilyHandler_1.parseFunctionalFamily)(source, langId);
    }
    if (/SELECT|INSERT|CREATE TABLE/i.test(source8k)) {
        return (0, DataFamilyHandler_1.parseDataFamily)(source, langId);
    }
    if (/<\w+[\s>]/.test(source8k)) {
        return (0, WebFamilyHandler_1.parseWebFamily)(source, langId);
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
function generateBestEffort(ir, targetLangId, opts) {
    // Map unknown targets to closest known family
    const targetFamily = getLanguageFamily(targetLangId);
    // Try to route via closest known handler
    if (targetFamily !== 'unknown') {
        switch (targetFamily) {
            case 'c-family': return (0, CFamilyHandler_1.generateCFamily)(ir, 'javascript', opts);
            case 'python-family': return (0, PythonFamilyHandler_1.generatePythonFamily)(ir, 'python', opts);
            case 'systems': return (0, SystemsFamilyHandler_1.generateSystemsFamily)(ir, 'c', opts);
            case 'web': return (0, WebFamilyHandler_1.generateWebFamily)(ir, 'html', opts);
            case 'data': return (0, DataFamilyHandler_1.generateDataFamily)(ir, 'json', opts);
            case 'shell': return (0, ShellFamilyHandler_1.generateShellFamily)(ir, 'bash', opts);
            case 'functional': return (0, FunctionalFamilyHandler_1.generateFunctionalFamily)(ir, 'haskell', opts);
            case 'omni': return (0, OmniLanguageHandler_1.generateOmniLanguage)(ir, 'titan', opts);
        }
    }
    // Final fallback: pseudocode comment structure
    const comment = '//';
    const lines = [
        `${comment} ${ir.name} — Converted to ${targetLangId}`,
        `${comment} Source: ${ir.sourceLanguage} | Family: ${ir.sourceFamily}`,
        `${comment} Note: ${targetLangId} is not directly supported. Showing pseudocode.`,
        '',
    ];
    for (const imp of ir.imports) {
        lines.push(`${comment} import: ${imp.path} (${imp.names.join(', ') || '*'})`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    for (const unit of ir.units) {
        const params = unit.signature.params.map(p => `${p.name}: ${p.type.name}`).join(', ');
        lines.push(`${comment} ${unit.kind} ${unit.name}(${params})`);
        if (unit.docComment) {
            lines.push(`${comment}   ${unit.docComment}`);
        }
        lines.push('');
    }
    return lines.join('\n');
}
// ─── Target path derivation ───────────────────────────────────────────────────
function deriveTargetPath(sourcePath, targetLangId) {
    const ext = getExtForLang(targetLangId);
    // Replace extension
    const dotIdx = sourcePath.lastIndexOf('.');
    if (dotIdx >= 0) {
        return sourcePath.slice(0, dotIdx) + ext;
    }
    return sourcePath + ext;
}
function getExtForLang(langId) {
    const EXT = {
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
function createEngine(opts) {
    return new OmniCCConversionEngine(opts);
}
function quickConvert(source, targetLang, sourceLang, filename) {
    const engine = new OmniCCConversionEngine();
    return engine.convert({ source, targetLang, sourceLang, filename });
}
//# sourceMappingURL=ConversionEngine.js.map