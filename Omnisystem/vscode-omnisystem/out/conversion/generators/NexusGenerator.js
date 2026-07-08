"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateNexus = generateNexus;
function kindToNexusLayout(kind) {
    const map = {
        button: { flex: 'inline-flex', align: 'center', justify: 'center' },
        input: { flex: 'flex', align: 'center' },
        textarea: { flex: 'flex', align: 'stretch' },
        card: { flex: 'column', align: 'stretch' },
        panel: { flex: 'column', align: 'stretch' },
        modal: { flex: 'column' },
        navbar: { flex: 'flex', align: 'center', justify: 'space-between' },
        sidebar: { flex: 'column', align: 'stretch' },
        form: { flex: 'column', align: 'stretch' },
        tabgroup: { flex: 'column' },
        list: { flex: 'column' },
        grid: { flex: 'grid' },
        table: { flex: 'column' },
        container: { flex: 'column', align: 'stretch' },
    };
    return map[kind] ?? { flex: 'column' };
}
function kindToSpacing(kind) {
    const map = {
        button: { padding: 2 },
        card: { gap: 4, padding: 6 },
        panel: { gap: 4, padding: 4 },
        form: { gap: 6, padding: 4 },
        navbar: { gap: 4, padding: 4 },
        sidebar: { gap: 2, padding: 4 },
        list: { gap: 2 },
        grid: { gap: 4 },
        container: { gap: 4, padding: 4 },
        modal: { gap: 4, padding: 6 },
        tabgroup: { gap: 0 },
    };
    return map[kind] ?? { gap: 2 };
}
function renderNexusNode(node, depth = 1) {
    const pad = '    '.repeat(depth);
    const { flex, align, justify } = kindToNexusLayout(node.kind);
    const { gap, padding } = kindToSpacing(node.kind);
    const name = node.name ?? node.kind;
    const owClass = `ow-${node.kind}`;
    const rules = [];
    if (flex === 'grid') {
        rules.push('grid');
        rules.push('grid-cols-auto');
    }
    else {
        rules.push(flex ?? 'flex');
        if (align) {
            rules.push(`align-${align.replace('-', '_')}`);
        }
        if (justify) {
            rules.push(`justify-${justify.replace('-', '_')}`);
        }
    }
    if (gap !== undefined) {
        rules.push(`gap-${gap}`);
    }
    if (padding !== undefined) {
        rules.push(`p-${padding}`);
    }
    const rulesStr = rules.join(' ');
    // If has children, render them recursively
    if (node.children && node.children.length > 0) {
        const childBlocks = node.children.map(c => renderNexusNode(c, depth + 1)).join('\n\n');
        return `${pad}// ${name} (${node.kind})\n${pad}${rulesStr} class="${owClass}" {\n${childBlocks}\n${pad}}`;
    }
    // Leaf node — render as a slot
    const slotContent = node.label ? `"${node.label}"` :
        node.kind === 'button' ? '"Button"' :
            node.kind === 'input' ? 'input-field' :
                node.kind;
    return `${pad}// ${name} (${node.kind})\n${pad}slot ${node.id ?? name.toLowerCase()} {\n${pad}    class: "${owClass}"\n${pad}    content: ${slotContent}\n${pad}}`;
}
function renderBreakpoints(node) {
    // Generate appropriate breakpoint overrides based on widget type
    const bps = [];
    if (['container', 'panel', 'grid', 'card'].includes(node.kind)) {
        bps.push('        sm: 640px { flex column gap-2 }');
        bps.push('        md: 768px { flex column gap-3 }');
        bps.push('        lg: 1024px { flex row gap-4 }');
        bps.push('        xl: 1280px { flex row gap-6 }');
    }
    else if (['navbar', 'tabgroup'].includes(node.kind)) {
        bps.push('        sm: 640px { flex column }');
        bps.push('        md: 768px { flex row }');
    }
    else {
        bps.push('        sm: 640px');
        bps.push('        md: 768px');
        bps.push('        lg: 1024px');
    }
    return bps.join('\n');
}
function generateNexus(ir) {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '') || 'Widget';
    const capitalName = name.charAt(0).toUpperCase() + name.slice(1);
    const breakpoints = renderBreakpoints(node);
    const body = renderNexusNode(node, 1);
    return `// ${capitalName}Layout — Nexus OW Responsive Layout
// Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
// Confidence: ${ir.confidence}

layout ${capitalName}Layout {

    breakpoints {
${breakpoints}
    }

    // Widget: ${node.kind} → OW class: ow-${node.kind}
${body}

    // ── Theme tokens ──────────────────────────────────────────────────
    // All spacing, color, and typography use OW CSS custom properties.
    // Override per theme via [data-theme="omni-*"] selectors.
    tokens {
        color:       var(--ow-text)
        background:  var(--ow-bg-card)
        border:      1px solid var(--ow-border)
        border-radius: var(--ow-r-md)
        padding:     var(--ow-space-md)
        gap:         var(--ow-space-sm)
        font-size:   var(--ow-text-sm)
        font-family: var(--ow-font-sans)
        transition:  all var(--ow-ease) var(--ow-duration)
    }
}
`;
}
//# sourceMappingURL=NexusGenerator.js.map