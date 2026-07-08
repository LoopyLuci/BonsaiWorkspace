"use strict";
// Widget Conversion Engine — main orchestrator
// Routes source code through the appropriate parser → IR → generator pipeline
Object.defineProperty(exports, "__esModule", { value: true });
exports.detectLanguage = detectLanguage;
exports.convert = convert;
exports.renderOWPreview = renderOWPreview;
const WidgetIR_1 = require("./WidgetIR");
const JsParser_1 = require("./parsers/JsParser");
const CssParser_1 = require("./parsers/CssParser");
const TauriParser_1 = require("./parsers/TauriParser");
const PythonParser_1 = require("./parsers/PythonParser");
const OmniParser_1 = require("./parsers/OmniParser");
const VeraGenerator_1 = require("./generators/VeraGenerator");
const NexusGenerator_1 = require("./generators/NexusGenerator");
const TitanGenerator_1 = require("./generators/TitanGenerator");
const JsGenerator_1 = require("./generators/JsGenerator");
const TsGenerator_1 = require("./generators/TsGenerator");
const CssGenerator_1 = require("./generators/CssGenerator");
const TauriGenerator_1 = require("./generators/TauriGenerator");
const PythonGenerator_1 = require("./generators/PythonGenerator");
// ─── Detection ────────────────────────────────────────────────────────────────
function detectLanguage(source, hint) {
    if (hint) {
        return hint;
    }
    const s = source.trim();
    // Vera: component X { props { } render { } }
    if (/^\s*component\s+\w+\s*\{/.test(s)) {
        return 'vera';
    }
    // Nexus: layout X { breakpoints { } }
    if (/^\s*layout\s+\w+\s*\{/.test(s)) {
        return 'nexus';
    }
    // Titan: pub struct / pub fn / mod X
    if (/\bpub\s+(struct|fn|enum|mod)\b/.test(s)) {
        return 'titan';
    }
    if (/\bactor\s+\w+\s*\{/.test(s)) {
        return 'titan';
    }
    // Python GUI
    if (/import\s+tkinter|from\s+tkinter|QPushButton|QLineEdit|QWidget|import\s+PyQt/.test(s)) {
        return 'python';
    }
    if (/\.mainloop\(\)|tk\.Tk\(\)|ttk\.\w+\(/.test(s)) {
        return 'python';
    }
    if (/window\.__TAURI__|@tauri-apps\/api|invoke\(/.test(s)) {
        return 'tauri';
    }
    // CSS
    if (/^\s*[.#][\w-]+\s*\{/.test(s) || /^[a-z][\w-]*\s*\{/.test(s)) {
        return 'css';
    }
    if (/var\(--[\w-]+\)/.test(s) && !/</.test(s)) {
        return 'css';
    }
    // TypeScript (must come before JS — more specific)
    if (/:\s*(string|number|boolean|HTMLElement|void|never|unknown)\b/.test(s) ||
        /interface\s+\w+|type\s+\w+\s*=|<\w+>/.test(s) ||
        /export\s+(default\s+)?(?:function|class|const|interface|type)\b/.test(s)) {
        return 'typescript';
    }
    // JavaScript
    return 'javascript';
}
// ─── Parser dispatch ──────────────────────────────────────────────────────────
function parse(source, lang) {
    switch (lang) {
        case 'javascript': return (0, JsParser_1.parseJs)(source, 'javascript');
        case 'typescript': return (0, JsParser_1.parseJs)(source, 'typescript');
        case 'css': return (0, CssParser_1.parseCss)(source);
        case 'tauri': return (0, TauriParser_1.parseTauri)(source);
        case 'python': return (0, PythonParser_1.parsePython)(source);
        case 'vera': return (0, OmniParser_1.parseOmni)(source, 'vera');
        case 'nexus': return (0, OmniParser_1.parseOmni)(source, 'nexus');
        case 'titan': return (0, OmniParser_1.parseOmni)(source, 'titan');
        default: return (0, JsParser_1.parseJs)(source, 'javascript');
    }
}
// ─── Generator dispatch ───────────────────────────────────────────────────────
function generate(ir, target) {
    switch (target) {
        case 'vera': return (0, VeraGenerator_1.generateVera)(ir);
        case 'nexus': return (0, NexusGenerator_1.generateNexus)(ir);
        case 'titan': return (0, TitanGenerator_1.generateTitan)(ir);
        case 'javascript': return (0, JsGenerator_1.generateJs)(ir);
        case 'typescript': return (0, TsGenerator_1.generateTs)(ir);
        case 'css': return (0, CssGenerator_1.generateCss)(ir);
        case 'tauri': return (0, TauriGenerator_1.generateTauri)(ir);
        case 'python': return (0, PythonGenerator_1.generatePython)(ir);
        default: return (0, VeraGenerator_1.generateVera)(ir);
    }
}
function convert(input) {
    const { source, widgetNameHint } = input;
    if (!source.trim()) {
        return {
            code: '',
            widgetType: 'unknown',
            widgetName: 'Widget',
            confidence: 'low',
            notes: ['No source code provided'],
            targetLanguage: input.targetLang || 'vera',
            fileExtension: WidgetIR_1.LANGUAGE_EXTENSIONS[input.targetLang] || '.vera',
        };
    }
    // Detect or use provided source language
    const srcLang = input.sourceLang === 'auto' || !input.sourceLang
        ? detectLanguage(source)
        : input.sourceLang;
    const targetLang = input.targetLang || 'vera';
    // Identity conversion (same language): just return cleaned-up source
    if (srcLang === targetLang) {
        return {
            code: source,
            widgetType: 'container',
            widgetName: widgetNameHint ?? 'Widget',
            confidence: 'high',
            notes: [`Source and target are both ${srcLang} — returned as-is`],
            targetLanguage: targetLang,
            fileExtension: WidgetIR_1.LANGUAGE_EXTENSIONS[targetLang],
        };
    }
    // Parse to IR
    let ir;
    try {
        ir = parse(source, srcLang);
    }
    catch (err) {
        return {
            code: `// Parse error: ${err}\n// Source language: ${srcLang}`,
            widgetType: 'unknown',
            widgetName: 'Widget',
            confidence: 'low',
            notes: [`Parse error: ${err}`],
            targetLanguage: targetLang,
            fileExtension: WidgetIR_1.LANGUAGE_EXTENSIONS[targetLang],
        };
    }
    // Override name if hint provided
    if (widgetNameHint && widgetNameHint.trim()) {
        ir.name = widgetNameHint.trim();
        ir.rootWidget.name = widgetNameHint.trim();
        ir.rootWidget.id = widgetNameHint.trim().toLowerCase().replace(/[^a-z0-9]/g, '_');
    }
    // Generate target code
    let code;
    try {
        code = generate(ir, targetLang);
    }
    catch (err) {
        return {
            code: `// Generation error: ${err}\n// Target language: ${targetLang}`,
            widgetType: ir.rootWidget.kind,
            widgetName: ir.name,
            confidence: 'low',
            notes: [...(ir.notes ?? []), `Generation error: ${err}`],
            targetLanguage: targetLang,
            fileExtension: WidgetIR_1.LANGUAGE_EXTENSIONS[targetLang],
        };
    }
    const notes = [...(ir.notes ?? [])];
    if (srcLang !== targetLang) {
        notes.push(`Converted ${srcLang} → ${targetLang}`);
    }
    return {
        code,
        widgetType: ir.rootWidget.kind,
        widgetName: ir.name,
        confidence: ir.confidence,
        notes,
        targetLanguage: targetLang,
        fileExtension: WidgetIR_1.LANGUAGE_EXTENSIONS[targetLang],
    };
}
// ─── Preview HTML for OW widget ───────────────────────────────────────────────
function renderOWPreview(kind, name) {
    const label = name || kind;
    switch (kind) {
        case 'button':
            return `<button class="ow-btn ow-btn-primary" style="pointer-events:none">${label}</button>`;
        case 'input':
            return `<input class="ow-input" placeholder="${label}" style="pointer-events:none" readonly />`;
        case 'textarea':
            return `<textarea class="ow-input" style="pointer-events:none;height:80px" readonly>${label}</textarea>`;
        case 'toggle':
            return `<div class="ow-toggle-wrap" role="switch" aria-checked="false" tabindex="-1" style="pointer-events:none">
              <div class="ow-toggle"></div>
            </div><span class="ow-label" style="margin-left:8px">${label}</span>`;
        case 'checkbox':
            return `<label class="ow-checkbox" style="pointer-events:none">
              <input type="checkbox" disabled /> <span class="ow-checkbox-label">${label}</span>
            </label>`;
        case 'select':
            return `<select class="ow-select" style="pointer-events:none" disabled>
              <option>${label}</option>
            </select>`;
        case 'slider':
            return `<input type="range" class="ow-slider" value="50" style="pointer-events:none" disabled />`;
        case 'badge':
        case 'chip':
            return `<span class="ow-badge">${label}</span>`;
        case 'card':
            return `<div class="ow-card" style="max-width:300px">
              <div class="ow-card-title">${label}</div>
              <div class="ow-card-body" style="color:var(--ow-text-dim);font-size:12px">Card content area</div>
            </div>`;
        case 'progress':
            return `<div class="ow-progress">
              <div class="ow-progress-fill" style="width:65%"></div>
            </div><div style="font-size:11px;color:var(--ow-text-dim);margin-top:4px">${label} — 65%</div>`;
        case 'spinner':
            return `<div class="ow-spinner" role="status" aria-label="${label}">
              <div class="ow-spinner-ring"></div>
            </div>`;
        case 'alert':
            return `<div class="ow-alert" role="alert">
              <strong>${label}</strong> — Alert message content here.
            </div>`;
        case 'modal':
            return `<div class="ow-card" style="max-width:300px;border:1px solid var(--ow-border)">
              <div class="ow-card-title">${label}</div>
              <div class="ow-card-body" style="color:var(--ow-text-dim);font-size:12px;padding:8px 0">Modal content area</div>
              <div style="display:flex;gap:8px;margin-top:12px">
                <button class="ow-btn ow-btn-ghost" style="pointer-events:none">Cancel</button>
                <button class="ow-btn ow-btn-primary" style="pointer-events:none">Confirm</button>
              </div>
            </div>`;
        case 'navbar':
            return `<nav class="ow-navbar" style="display:flex;gap:16px;padding:12px;background:var(--ow-bg-card);border:1px solid var(--ow-border);border-radius:8px;pointer-events:none">
              <span style="color:var(--ow-accent);font-weight:700">${label}</span>
              <span style="color:var(--ow-text-dim);font-size:12px">Nav Item</span>
              <span style="color:var(--ow-text-dim);font-size:12px">Nav Item</span>
            </nav>`;
        case 'list':
            return `<ul class="ow-list" style="pointer-events:none;padding-left:20px">
              <li class="ow-list-item">${label} Item 1</li>
              <li class="ow-list-item">${label} Item 2</li>
              <li class="ow-list-item">${label} Item 3</li>
            </ul>`;
        case 'tabgroup':
            return `<div class="ow-tabs" style="pointer-events:none">
              <div role="tablist" style="display:flex;gap:4px;border-bottom:1px solid var(--ow-border);margin-bottom:12px">
                <button role="tab" aria-selected="true" class="ow-btn ow-btn-primary ow-btn-sm">${label}</button>
                <button role="tab" class="ow-btn ow-btn-ghost ow-btn-sm">Tab 2</button>
              </div>
              <div role="tabpanel" style="color:var(--ow-text-dim);font-size:12px">Tab content area</div>
            </div>`;
        case 'divider':
            return `<hr class="ow-divider" style="border:none;border-top:1px solid var(--ow-border);margin:8px 0" />`;
        case 'label':
            return `<span class="ow-label" style="color:var(--ow-text)">${label}</span>`;
        default:
            return `<div class="ow-card" style="max-width:320px;pointer-events:none">
              <div style="color:var(--ow-accent);font-weight:600;margin-bottom:4px">${label}</div>
              <div style="color:var(--ow-text-dim);font-size:12px">Widget type: ${kind}</div>
            </div>`;
    }
}
//# sourceMappingURL=WidgetConversionEngine.js.map