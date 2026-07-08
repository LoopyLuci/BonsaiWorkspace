"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseTauri = parseTauri;
// Tauri Parser — detects widgets in Tauri app code (HTML+JS+Rust invoke patterns)
const WidgetIR_1 = require("../WidgetIR");
const JsParser_1 = require("./JsParser");
const HTML_TAG_TO_KIND = {
    'button': 'button', 'input': 'input', 'textarea': 'textarea',
    'select': 'select', 'checkbox': 'checkbox',
    'div': 'container', 'section': 'panel', 'article': 'card',
    'nav': 'navbar', 'aside': 'sidebar', 'header': 'navbar', 'footer': 'panel',
    'span': 'label', 'p': 'label', 'h1': 'label', 'h2': 'label', 'h3': 'label',
    'a': 'button', 'img': 'image', 'form': 'form',
    'ul': 'list', 'ol': 'list', 'li': 'listitem',
    'table': 'table', 'progress': 'progress',
    'hr': 'divider', 'label': 'label',
};
function extractHtmlAttr(tag, attr) {
    const rx = new RegExp(`${attr}=["']([^"']+)["']`, 'i');
    return tag.match(rx)?.[1];
}
function parseHtmlTag(src) {
    // Find first significant HTML tag
    const tagRx = /<(button|input|select|textarea|div|section|article|nav|aside|form|a|img|h[1-6]|span|p|ul|ol|li|table|progress|label|hr)(\s[^>]*)?\s*(?:\/?>|>)/i;
    const m = src.match(tagRx);
    if (!m) {
        return null;
    }
    const tagName = m[1].toLowerCase();
    const attrBlock = m[2] ?? '';
    const kind = HTML_TAG_TO_KIND[tagName] ?? 'container';
    const label = extractHtmlAttr(attrBlock, 'id') ? undefined :
        (() => {
            const innerRx = new RegExp(`<${tagName}[^>]*>([^<]{1,100})</${tagName}>`, 'i');
            return src.match(innerRx)?.[1]?.trim();
        })();
    const placeholder = extractHtmlAttr(attrBlock, 'placeholder');
    const value = extractHtmlAttr(attrBlock, 'value');
    const idAttr = extractHtmlAttr(attrBlock, 'id');
    const classAttr = extractHtmlAttr(attrBlock, 'class');
    // Extract inline events
    const events = [];
    const inlineEventRx = /\bon(\w+)=["']([^"']+)["']/g;
    let em;
    while ((em = inlineEventRx.exec(attrBlock)) !== null) {
        events.push({ name: `on${em[1].charAt(0).toUpperCase() + em[1].slice(1)}`, handler: em[2], params: ['event'] });
    }
    // Extract JS addEventListener calls
    if (idAttr) {
        const addListenerRx = new RegExp(`(?:getElementById|querySelector)\\(['"]#?${idAttr}['"]\\)[^;]*\\.addEventListener\\(['"]([^'"]+)['"],\\s*([^)]+)\\)`, 'g');
        let alm;
        while ((alm = addListenerRx.exec(src)) !== null) {
            const evName = 'on' + alm[1].charAt(0).toUpperCase() + alm[1].slice(1);
            events.push({ name: evName, handler: alm[2].trim().slice(0, 150), params: ['event'] });
        }
    }
    const name = idAttr
        ? idAttr.replace(/-/g, '_').replace(/^\w/, c => c.toUpperCase())
        : classAttr?.split(' ')[0].replace(/-/g, '_').replace(/^\w/, c => c.toUpperCase()) ?? tagName;
    return { kind, label, placeholder, value, events, name };
}
function extractTauriInvokes(src) {
    const invokes = [];
    const invokeRx = /invoke\s*\(\s*['"]([^'"]+)['"]/g;
    let m;
    while ((m = invokeRx.exec(src)) !== null) {
        invokes.push(m[1]);
    }
    return invokes;
}
function extractScriptSection(src) {
    const scriptRx = /<script(?:\s[^>]*)?>([^]*?)<\/script>/i;
    return src.match(scriptRx)?.[1] ?? '';
}
function extractHtmlSection(src) {
    // Everything that's not <script> or <style>
    return src.replace(/<script[^>]*>[^]*?<\/script>/gi, '')
        .replace(/<style[^>]*>[^]*?<\/style>/gi, '');
}
function parseTauri(source) {
    const src = source.trim();
    const notes = [];
    // Check for Tauri-specific patterns
    const hasTauriImport = /window\.__TAURI__|@tauri-apps\/api|invoke\(/.test(src);
    if (hasTauriImport) {
        notes.push('Tauri API patterns detected');
    }
    const htmlSection = extractHtmlSection(src);
    const scriptSection = extractScriptSection(src);
    const invokes = extractTauriInvokes(src);
    if (invokes.length > 0) {
        notes.push(`Tauri invoke calls detected: ${invokes.slice(0, 3).join(', ')}`);
    }
    // Try HTML parsing first
    const htmlTag = parseHtmlTag(htmlSection || src);
    // If there's a script section with React/Vue JSX, parse that too
    let jsResult = null;
    if (scriptSection && (scriptSection.includes('<') || scriptSection.includes('createElement'))) {
        jsResult = (0, JsParser_1.parseJs)(scriptSection, 'javascript');
    }
    const best = (htmlTag && htmlTag.kind !== 'unknown') ? 'html' : (jsResult ? 'js' : 'html');
    if (best === 'html' && htmlTag) {
        const node = {
            id: (0, WidgetIR_1.makeId)(htmlTag.name),
            kind: htmlTag.kind,
            name: htmlTag.name,
            label: htmlTag.label,
            placeholder: htmlTag.placeholder,
            value: htmlTag.value,
            events: [
                ...htmlTag.events,
                // Add invoke-based events
                ...invokes.map(inv => ({
                    name: `on${inv.replace(/_(\w)/g, (_, c) => c.toUpperCase()).replace(/^\w/, c => c.toUpperCase())}`,
                    handler: `invoke('${inv}')`,
                    params: [],
                })),
            ],
        };
        const confidence = htmlTag.kind !== 'unknown' ? (htmlTag.events.length > 0 || invokes.length > 0 ? 'high' : 'medium') : 'low';
        return {
            name: htmlTag.name,
            rootWidget: node,
            sourceLanguage: 'tauri',
            confidence,
            notes: [...notes, `HTML element: <${htmlTag.kind}>`],
        };
    }
    if (jsResult) {
        return { ...jsResult, sourceLanguage: 'tauri', notes: [...notes, ...(jsResult.notes ?? [])] };
    }
    return {
        name: 'TauriWidget',
        rootWidget: { id: 'tauri_widget', kind: 'container', name: 'TauriWidget' },
        sourceLanguage: 'tauri',
        confidence: 'low',
        notes: [...notes, 'Could not detect specific widget pattern'],
    };
}
//# sourceMappingURL=TauriParser.js.map