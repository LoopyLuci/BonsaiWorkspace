"use strict";
// WidgetBridge — Deep integration between OmniCC and the Widget Converter system.
// When OmniCC detects UI patterns in parsed code, this bridge routes those units
// to the Widget Converter for specialized conversion, then merges results back.
Object.defineProperty(exports, "__esModule", { value: true });
exports.detectUIPatterns = detectUIPatterns;
exports.runWidgetBridge = runWidgetBridge;
exports.mergeWidgetBridgeResults = mergeWidgetBridgeResults;
exports.buildBridgeSummary = buildBridgeSummary;
const WidgetConversionEngine_1 = require("../conversion/WidgetConversionEngine");
const WIDGET_KINDS = new Set([
    'widget-component', 'widget-layout', 'widget-style', 'widget-event',
]);
const UI_UNIT_NAMES = new Set([
    'Button', 'Input', 'TextInput', 'TextField', 'Select', 'Dropdown', 'Checkbox', 'Toggle',
    'Switch', 'Radio', 'Slider', 'DatePicker', 'TimePicker', 'ColorPicker', 'FileInput',
    'Modal', 'Dialog', 'Drawer', 'Sidebar', 'Tooltip', 'Popover', 'Toast', 'Alert', 'Badge',
    'Card', 'Panel', 'Section', 'Container', 'Box', 'Flex', 'Grid', 'Stack', 'Row', 'Column',
    'List', 'Table', 'DataGrid', 'Tree', 'Tabs', 'TabPanel', 'Accordion', 'Carousel',
    'Avatar', 'Icon', 'Image', 'Label', 'Text', 'Heading', 'Paragraph', 'Link',
    'Form', 'FormField', 'FormGroup', 'Layout', 'Nav', 'NavBar', 'Header', 'Footer',
    'Sidebar', 'MainContent', 'Page', 'View', 'Screen', 'Widget',
    'render', 'renderComponent', 'renderView', 'renderPage',
    'template', 'inlineStyle', 'inlineScript',
]);
// ─── Detection ────────────────────────────────────────────────────────────────
function detectUIPatterns(ir) {
    const uiUnits = [];
    let score = 0;
    if (ir.metadata.hasUI) {
        score += 30;
    }
    for (const unit of ir.units) {
        const isWidgetKind = WIDGET_KINDS.has(unit.kind);
        const isWidgetName = UI_UNIT_NAMES.has(unit.name) || UI_UNIT_NAMES.has(unit.name.split('.').pop() ?? '');
        const hasRenderReturn = unit.body.some(s => s.kind === 'return' || s.kind === 'raw');
        const srcHasJSX = (unit.originalSource ?? '').includes('<') && (unit.originalSource ?? '').includes('>');
        const srcHasComponent = /component|Component|render|View|Widget/.test(unit.originalSource ?? '');
        if (isWidgetKind || isWidgetName || srcHasJSX || srcHasComponent) {
            uiUnits.push(unit);
            score += isWidgetKind ? 25 : isWidgetName ? 15 : srcHasJSX ? 20 : 10;
        }
    }
    // Check widgetUnits field directly
    if (ir.widgetUnits && ir.widgetUnits.length > 0) {
        uiUnits.push(...ir.widgetUnits.filter(u => !uiUnits.includes(u)));
        score += ir.widgetUnits.length * 20;
    }
    return {
        hasUI: score > 0,
        uiUnits: [...new Map(uiUnits.map(u => [u.name, u])).values()],
        confidence: Math.min(100, score),
    };
}
// ─── Bridge ───────────────────────────────────────────────────────────────────
function runWidgetBridge(ir, targetLangId, opts) {
    const { hasUI, uiUnits, confidence } = detectUIPatterns(ir);
    if (!hasUI || uiUnits.length === 0) {
        return { detected: false, widgetCount: 0, widgetResults: [], mergedNotes: [], uiUnits: [] };
    }
    const widgetResults = [];
    const mergedNotes = [
        `WidgetBridge: detected ${uiUnits.length} UI unit(s) from ${ir.sourceLanguage} (confidence: ${confidence}%)`,
    ];
    for (const unit of uiUnits.slice(0, 20)) { // cap at 20 widget units per conversion
        const widgetSource = buildWidgetSource(unit, ir.sourceLanguage);
        if (!widgetSource) {
            continue;
        }
        const srcLang = mapToWidgetLang(ir.sourceLanguage);
        const tgtLang = mapToWidgetLang(targetLangId);
        if (srcLang && tgtLang) {
            const result = (0, WidgetConversionEngine_1.convert)({ source: widgetSource, sourceLang: srcLang, targetLang: tgtLang, widgetNameHint: guessWidgetName(unit) });
            if (result) {
                widgetResults.push(result);
                if (result.notes && result.notes.length > 0) {
                    mergedNotes.push(...result.notes.map((n) => `  [${unit.name}] ${n}`));
                }
            }
        }
        else {
            mergedNotes.push(`  [${unit.name}] no widget converter path: ${ir.sourceLanguage} → ${targetLangId}`);
        }
    }
    mergedNotes.push(`WidgetBridge: converted ${widgetResults.length}/${uiUnits.length} UI units`);
    return { detected: true, widgetCount: uiUnits.length, widgetResults, mergedNotes, uiUnits };
}
// ─── Merge ─────────────────────────────────────────────────────────────────────
function mergeWidgetBridgeResults(mainOutput, bridgeResult, targetLangId) {
    if (!bridgeResult.detected || bridgeResult.widgetResults.length === 0) {
        return mainOutput;
    }
    const comment = getLineComment(targetLangId);
    const sections = [mainOutput];
    sections.push('');
    sections.push(`${comment} ─── Widget Bridge Results (${bridgeResult.widgetResults.length} UI units) ───`);
    for (const result of bridgeResult.widgetResults) {
        sections.push('');
        sections.push(`${comment} Widget: ${result.widgetName ?? 'unnamed'} [${result.targetLanguage}]`);
        sections.push(result.code);
    }
    return sections.join('\n');
}
// ─── Helpers ──────────────────────────────────────────────────────────────────
function buildWidgetSource(unit, langId) {
    const src = unit.originalSource;
    if (!src || src.trim().length < 5) {
        // Synthesize minimal source from unit info
        const name = unit.name.split('.').pop() ?? unit.name;
        return synthesizeWidgetSource(name, langId);
    }
    return src;
}
function synthesizeWidgetSource(name, langId) {
    // Generate minimal widget source for languages we can synthesize
    switch (langId) {
        case 'javascript':
        case 'jsx':
            return `function ${name}() { return <div className="${name.toLowerCase()}">{children}</div>; }`;
        case 'typescript':
        case 'tsx':
            return `const ${name}: React.FC = () => <div className="${name.toLowerCase()}">{children}</div>;`;
        case 'vera':
            return `component ${name} { render { div .${name.toLowerCase()} { } } }`;
        case 'nexus':
            return `layout ${name} { container .main { } }`;
        case 'python':
            return `class ${name}(tk.Frame):\n    def __init__(self, parent):\n        super().__init__(parent)`;
        case 'css':
            return `.${name.toLowerCase()} { display: flex; flex-direction: column; }`;
        default:
            return `// ${name}`;
    }
}
function guessWidgetName(unit) {
    const name = unit.name.split('.').pop() ?? unit.name;
    // Strip common suffixes that aren't part of the widget name
    return name.replace(/(?:Component|Widget|Panel|View|Screen|Page)$/, '') || name;
}
// Map OmniCC language IDs to Widget Converter language IDs
function mapToWidgetLang(langId) {
    const MAP = {
        // JS ecosystem
        'javascript': 'js', 'js': 'js',
        'typescript': 'ts', 'ts': 'ts',
        'jsx': 'js', 'tsx': 'ts',
        'vue': 'js', 'svelte': 'js', 'angular': 'ts',
        // CSS
        'css': 'css', 'scss': 'css', 'less': 'css',
        // Python UI
        'python': 'python',
        // Tauri
        'tauri': 'tauri',
        // Omni
        'vera': 'vera', 'nexus': 'nexus', 'titan': 'titan',
        // HTML → treat as CSS for widget purposes
        'html': 'css',
    };
    return MAP[langId] ?? null;
}
function getLineComment(langId) {
    switch (langId) {
        case 'python':
        case 'ruby':
        case 'bash':
        case 'yaml': return '#';
        case 'html': return '<!--';
        case 'css':
        case 'scss': return '/*';
        case 'sql': return '--';
        case 'haskell':
        case 'elm':
        case 'lua': return '--';
        default: return '//';
    }
}
function buildBridgeSummary(result, ir) {
    const { hasUI, confidence } = detectUIPatterns(ir);
    const previewHtml = result.widgetResults.length > 0
        ? generatePreviewHtml(result.widgetResults[0])
        : '';
    return {
        detected: result.detected,
        widgetCount: result.widgetCount,
        convertedCount: result.widgetResults.length,
        confidence,
        previewHtml,
    };
}
function generatePreviewHtml(result) {
    const name = result.widgetName ?? 'Widget';
    return `<div class="ow-widget-bridge-preview" data-widget="${name}">
  <div class="ow-badge ow-badge--primary">${name}</div>
  <pre class="ow-code">${escapeHtml(result.code.slice(0, 300))}</pre>
</div>`;
}
function escapeHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}
//# sourceMappingURL=WidgetBridge.js.map