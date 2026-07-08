"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseWebFamily = parseWebFamily;
exports.generateWebFamily = generateWebFamily;
// Web-Family Handler — HTML, CSS, SCSS, JSX/TSX, Vue, Svelte, Angular, Astro, HTMX, Tailwind
const ULIR_1 = require("../ULIR");
const LanguageRegistry_1 = require("../LanguageRegistry");
const BodyTranslator_1 = require("../BodyTranslator");
// ─── Parse ────────────────────────────────────────────────────────────────────
function parseWebFamily(source, langId) {
    const lang = (0, LanguageRegistry_1.getLang)(langId);
    const lines = source.split('\n');
    const units = [];
    const imports = [];
    switch (langId) {
        case 'html':
            extractHTMLUnits(source, units);
            break;
        case 'css':
        case 'scss':
        case 'less':
            extractCSSUnits(source, units, langId);
            break;
        case 'jsx':
        case 'tsx':
        case 'javascript':
        case 'typescript':
            extractJSXUnits(source, units, imports, langId);
            break;
        case 'vue':
            extractVueUnits(source, units, imports);
            break;
        case 'svelte':
            extractSvelteUnits(source, units, imports);
            break;
        case 'angular':
            extractAngularUnits(source, units, imports);
            break;
        case 'astro':
            extractAstroUnits(source, units, imports);
            break;
        default:
            extractHTMLUnits(source, units);
            break;
    }
    // Extract imports for JS-based frameworks
    if (['jsx', 'tsx', 'vue', 'svelte', 'angular', 'astro'].includes(langId)) {
        for (const line of lines) {
            const imp = extractImport(line.trim());
            if (imp) {
                imports.push(imp);
            }
        }
    }
    const hasUI = true; // all web formats have UI
    const meta = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['declarative', 'event-driven'],
        typeSystem: lang?.typing ?? 'dynamic-weak',
        memoryModel: 'gc',
        usesAsync: /\basync\b|\bawait\b|fetch\(|\.then\(/.test(source),
        usesGenerics: /<\w+>/.test(source),
        usesReflection: false,
        usesMetaprogramming: /@Component|@NgModule|@Injectable/.test(source),
        hasTests: /\bdescribe\b|\bit\(|\btest\(|\bexpect\(/.test(source),
        hasUI,
        hasSideEffects: /\bfetch\b|\baxios\b|\blocalStorage\b|\bdocument\b/.test(source),
    };
    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'web',
        units,
        imports,
        exports: units.filter(u => u.visibility === 'public').map(u => u.name),
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes: [],
        docComment: '',
    };
}
function extractHTMLUnits(source, units) {
    // Custom elements / web components
    for (const m of source.matchAll(/<([\w][\w-]*)\s/g)) {
        const tag = m[1];
        if (tag.includes('-') || /^[A-Z]/.test(tag)) { // custom element or component
            units.push(makeWebUnit('widget-component', tag, m[0]));
        }
    }
    // Template sections (main, section, article, aside, nav, header, footer)
    for (const m of source.matchAll(/<(main|section|article|aside|nav|header|footer|form)(\s[^>]*)?>/) ?? []) {
        units.push(makeWebUnit('widget-layout', m[1], m[0]));
    }
    // Script blocks
    for (const m of source.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)) {
        if (m[1].trim()) {
            units.push(makeWebUnit('function', 'inlineScript', m[0].slice(0, 100)));
        }
    }
    // Style blocks
    for (const m of source.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) {
        if (m[1].trim()) {
            units.push(makeWebUnit('widget-style', 'inlineStyle', m[0].slice(0, 100)));
        }
    }
}
function extractCSSUnits(source, units, langId) {
    // Rule sets
    for (const m of source.matchAll(/([.#]?[\w-]+(?:[,\s]+[.#]?[\w-]+)*)\s*\{([^}]+)\}/g)) {
        const selector = m[1].trim();
        if (selector.startsWith('@')) {
            continue;
        }
        units.push(makeWebUnit('widget-style', selectorToName(selector), m[0].slice(0, 200)));
    }
    // SCSS/CSS custom properties (variables)
    for (const m of source.matchAll(/--[\w-]+:/g)) {
        units.push(makeWebUnit('constant', m[0].replace(':', ''), m[0]));
    }
    // SCSS mixins
    for (const m of source.matchAll(/@mixin\s+([\w-]+)/g)) {
        units.push(makeWebUnit('function', m[1], m[0]));
    }
    // SCSS @extend / @include
    for (const m of source.matchAll(/@function\s+([\w-]+)/g)) {
        units.push(makeWebUnit('function', m[1], m[0]));
    }
    // @keyframes
    for (const m of source.matchAll(/@keyframes\s+([\w-]+)/g)) {
        units.push(makeWebUnit('constant', `anim_${m[1]}`, m[0]));
    }
    // @media queries
    let mediaCount = 0;
    for (const m of source.matchAll(/@media\s+[^{]+\{/g)) {
        units.push(makeWebUnit('widget-style', `media_${++mediaCount}`, m[0]));
    }
}
function extractJSXUnits(source, units, imports, langId) {
    // React/JSX components (function that returns JSX)
    for (const m of source.matchAll(/(?:export\s+(?:default\s+)?)?(?:function|const|let)\s+([A-Z][\w]*)\s*(?:=\s*(?:\([^)]*\)\s*=>|function\s*\()|\()/g)) {
        const body = source.slice(m.index ?? 0, (m.index ?? 0) + 500);
        const isComponent = body.includes('return') && (body.includes('<') || body.includes('jsx'));
        units.push(makeWebUnit(isComponent ? 'widget-component' : 'function', m[1], m[0]));
    }
    // Hooks
    for (const m of source.matchAll(/(?:const|let)\s+(use\w+)\s*=/g)) {
        units.push(makeWebUnit('function', m[1], m[0]));
    }
    // Event handlers
    for (const m of source.matchAll(/(?:const|let)\s+(handle\w+|on\w+)\s*=/g)) {
        units.push(makeWebUnit('widget-event', m[1], m[0]));
    }
}
function extractVueUnits(source, units, imports) {
    // Vue SFC sections
    const templateMatch = source.match(/<template>([\s\S]*?)<\/template>/);
    if (templateMatch) {
        units.push(makeWebUnit('widget-layout', 'template', templateMatch[0].slice(0, 100)));
    }
    const scriptMatch = source.match(/<script[^>]*>([\s\S]*?)<\/script>/);
    if (scriptMatch) {
        // Components, methods, computed
        for (const m of scriptMatch[1].matchAll(/(?:methods|computed|setup)\s*\(\s*\)\s*\{|(\w+)\s*\([^)]*\)\s*\{/g)) {
            if (m[1]) {
                units.push(makeWebUnit('function', m[1], m[0]));
            }
        }
        // export default component
        units.push(makeWebUnit('widget-component', detectModuleName(source, 'vue'), scriptMatch[0].slice(0, 100)));
    }
    const styleMatch = source.match(/<style[^>]*>([\s\S]*?)<\/style>/);
    if (styleMatch) {
        units.push(makeWebUnit('widget-style', 'styles', styleMatch[0].slice(0, 100)));
    }
}
function extractSvelteUnits(source, units, imports) {
    const scriptMatch = source.match(/<script[^>]*>([\s\S]*?)<\/script>/);
    if (scriptMatch) {
        // Props (exported let variables)
        for (const m of scriptMatch[1].matchAll(/export\s+let\s+(\w+)/g)) {
            units.push(makeWebUnit('constant', m[1], m[0]));
        }
        // Functions
        for (const m of scriptMatch[1].matchAll(/(?:function|const\s+)(\w+)\s*(?:=\s*(?:async\s+)?\(|\()/g)) {
            units.push(makeWebUnit('function', m[1], m[0]));
        }
    }
    // Template as component
    units.push(makeWebUnit('widget-component', detectModuleName(source, 'svelte'), source.slice(0, 100)));
    // Style
    const styleMatch = source.match(/<style[^>]*>([\s\S]*?)<\/style>/);
    if (styleMatch) {
        units.push(makeWebUnit('widget-style', 'styles', styleMatch[0].slice(0, 100)));
    }
}
function extractAngularUnits(source, units, imports) {
    // @Component, @Directive, @Pipe, @Injectable, @NgModule
    for (const m of source.matchAll(/@(Component|Directive|Pipe|Injectable|NgModule)\s*\([^)]*\)[^]*?(?:class|export class)\s+(\w+)/g)) {
        const kind = m[1] === 'Component' || m[1] === 'Directive' ? 'widget-component' : 'class';
        units.push(makeWebUnit(kind, m[2], m[0].slice(0, 150)));
    }
    // Methods
    for (const m of source.matchAll(/(?:public|private|protected)?\s+(ng\w+|\w+)\s*\([^)]*\)\s*(?::\s*\w+)?\s*\{/g)) {
        units.push(makeWebUnit('function', m[1], m[0]));
    }
}
function extractAstroUnits(source, units, imports) {
    // Frontmatter
    const frontmatterMatch = source.match(/^---\n([\s\S]*?)\n---/);
    if (frontmatterMatch) {
        units.push(makeWebUnit('constant', 'frontmatter', frontmatterMatch[0].slice(0, 100)));
    }
    // Props (Astro.props)
    for (const m of source.matchAll(/const\s+\{\s*([^}]+)\s*\}\s*=\s*Astro\.props/g)) {
        const props = m[1].split(',').map(p => p.trim());
        for (const p of props) {
            units.push(makeWebUnit('constant', p, m[0]));
        }
    }
    units.push(makeWebUnit('widget-component', detectModuleName(source, 'astro'), source.slice(0, 100)));
}
function extractImport(line) {
    const m = line.match(/^import\s+(?:\{([^}]+)\}|(\w+)|\*\s+as\s+(\w+))\s+from\s+['"]([^'"]+)['"]/);
    if (m) {
        return {
            path: m[4],
            alias: m[3],
            names: m[1] ? m[1].split(',').map(n => n.trim()) : m[2] ? [m[2]] : [],
            isDefault: !!m[2], isWildcard: !!m[3],
            kind: m[4].startsWith('.') ? 'relative' : 'external',
            originalSyntax: line,
        };
    }
    return null;
}
function makeWebUnit(kind, name, src) {
    return {
        kind,
        name: sanitizeName(name),
        visibility: 'public',
        signature: { params: [], returns: ULIR_1.VOID_TYPE, throws: [] },
        body: [],
        attributes: [],
        docComment: '',
        sourceLines: [0, 0],
        isAsync: false, isStatic: false, isAbstract: false,
        isFinal: false, isOverride: false, isExtern: false,
        generics: [], extends_: [], implements_: [], children: [],
        originalSource: src,
        confidence: 'medium',
    };
}
function selectorToName(selector) {
    return selector.replace(/[.#\s,>+~:[\]()]/g, '_').replace(/^_+|_+$/g, '').replace(/__+/g, '_') || 'rule';
}
function sanitizeName(name) {
    return name.replace(/[^a-zA-Z0-9_$]/g, '_').replace(/^(\d)/, '_$1') || 'unnamed';
}
function detectModuleName(source, langId) {
    // Vue: <script> export default { name: 'X' }
    const m = source.match(/name:\s*['"]([^'"]+)['"]/);
    if (m) {
        return m[1];
    }
    // Angular: @Component({ selector: 'app-x' })
    const a = source.match(/selector:\s*['"]([^'"]+)['"]/);
    if (a) {
        return a[1].replace(/^app-/, '').replace(/-(\w)/g, (_, c) => c.toUpperCase());
    }
    return 'Component';
}
// ─── Generate ─────────────────────────────────────────────────────────────────
function generateWebFamily(ir, targetLangId, opts = ULIR_1.DEFAULT_OPTIONS) {
    switch (targetLangId) {
        case 'html': return generateHTML(ir, opts);
        case 'css': return generateCSS(ir, opts);
        case 'scss': return generateSCSS(ir, opts);
        case 'jsx':
        case 'tsx': return generateJSX(ir, opts, targetLangId === 'tsx');
        case 'vue': return generateVue(ir, opts);
        case 'svelte': return generateSvelte(ir, opts);
        case 'angular': return generateAngular(ir, opts);
        case 'astro': return generateAstro(ir, opts);
        default: return generateHTML(ir, opts);
    }
}
function generateHTML(ir, opts) {
    const components = ir.units.filter(u => u.kind === 'widget-component');
    const layouts = ir.units.filter(u => u.kind === 'widget-layout');
    const lines = [
        '<!DOCTYPE html>',
        `<!-- ${ir.name} — Converted to HTML -->`,
        `<!-- Source: ${ir.sourceLanguage} -->`,
        '<html lang="en">',
        '<head>',
        '  <meta charset="UTF-8">',
        `  <title>${ir.name}</title>`,
        '</head>',
        '<body>',
    ];
    for (const unit of [...layouts, ...components]) {
        const tag = unit.kind === 'widget-layout' ? 'section' : 'div';
        lines.push(`  <${tag} id="${unit.name}">`);
        lines.push(`    <!-- ${unit.name} -->`);
        lines.push(`  </${tag}>`);
    }
    lines.push('</body>', '</html>');
    return lines.join('\n');
}
function generateCSS(ir, opts) {
    const lines = [`/* ${ir.name} — Converted to CSS */`, `/* Source: ${ir.sourceLanguage} */`, ''];
    const styleUnits = ir.units.filter(u => u.kind === 'widget-style' || u.kind === 'class');
    const otherUnits = ir.units.filter(u => u.kind !== 'widget-style' && u.kind !== 'class');
    for (const unit of [...styleUnits, ...otherUnits]) {
        const sel = unit.name.startsWith('anim_') ? `@keyframes ${unit.name.replace('anim_', '')}` : `.${unit.name}`;
        lines.push(`${sel} {`);
        lines.push(`  /* ${unit.name} */`);
        lines.push('}');
        lines.push('');
    }
    return lines.join('\n');
}
function generateSCSS(ir, opts) {
    const lines = [`// ${ir.name} — Converted to SCSS`, `// Source: ${ir.sourceLanguage}`, ''];
    // Variables
    lines.push('// Design tokens');
    lines.push('$primary: #6366f1;', '$secondary: #8b5cf6;', '$spacing: 8px;', '');
    // Mixins for functions
    for (const unit of ir.units.filter(u => u.kind === 'function')) {
        lines.push(`@mixin ${unit.name}($args...) {`);
        lines.push((0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'scss', '  '));
        lines.push('}');
        lines.push('');
    }
    // Rules for classes/styles
    for (const unit of ir.units.filter(u => u.kind !== 'function')) {
        lines.push(`.${unit.name} {`);
        lines.push((0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, 'scss', '  '));
        lines.push('}');
        lines.push('');
    }
    return lines.join('\n');
}
function generateJSX(ir, opts, isTS) {
    const lang = isTS ? 'tsx' : 'jsx';
    const lines = [
        `// ${ir.name} — Converted to ${isTS ? 'TSX' : 'JSX'}`,
        `// Source: ${ir.sourceLanguage}`,
        `import React from 'react';`,
        '',
    ];
    for (const imp of ir.imports) {
        lines.push(`import { ${imp.names.join(', ')} } from '${imp.path}';`);
    }
    if (ir.imports.length > 0) {
        lines.push('');
    }
    const components = ir.units.filter(u => u.kind === 'widget-component');
    const functions = ir.units.filter(u => u.kind === 'function' || u.kind === 'widget-event');
    for (const unit of functions) {
        const pStr = isTS ? unit.signature.params.map(p => `${p.name}: string`).join(', ') : unit.signature.params.map(p => p.name).join(', ');
        const body = (0, BodyTranslator_1.translateBody)(unit.originalSource ?? '', ir.sourceLanguage, isTS ? 'typescript' : 'javascript', '  ');
        lines.push(`const ${unit.name} = (${pStr}) => {\n${body}\n};`);
        lines.push('');
    }
    for (const unit of components) {
        const propsType = isTS ? `interface ${unit.name}Props {}\n\n` : '';
        lines.push(propsType + `export const ${unit.name}${isTS ? `: React.FC<${unit.name}Props>` : ''} = () => {`);
        lines.push('  return (');
        lines.push(`    <div className="${unit.name.toLowerCase()}">`);
        lines.push(`      {/* ${unit.name} */}`);
        lines.push('    </div>');
        lines.push('  );');
        lines.push('};');
        lines.push('');
    }
    if (components.length === 0) {
        lines.push(`export const ${ir.name} = () => (`);
        lines.push(`  <div className="${ir.name.toLowerCase()}">`);
        lines.push(`    {/* ${ir.name} */}`);
        lines.push('  </div>');
        lines.push(');');
    }
    return lines.join('\n');
}
function generateVue(ir, opts) {
    const props = ir.units.filter(u => u.kind === 'constant').map(u => `  ${u.name}: { type: String, default: '' }`).join(',\n');
    const methods = ir.units.filter(u => u.kind === 'function').map(u => `    ${u.name}() {\n${(0, BodyTranslator_1.translateBody)(u.originalSource ?? '', ir.sourceLanguage, 'javascript', '      ')}\n    }`).join(',\n');
    return [
        `<!-- ${ir.name} — Converted to Vue -->`,
        `<!-- Source: ${ir.sourceLanguage} -->`,
        '',
        '<template>',
        `  <div class="${ir.name.toLowerCase()}">`,
        `    <!-- ${ir.name} -->`,
        '  </div>',
        '</template>',
        '',
        '<script>',
        `export default {`,
        `  name: '${ir.name}',`,
        props ? `  props: {\n${props}\n  },` : '',
        methods ? `  methods: {\n${methods}\n  },` : '',
        '};',
        '</script>',
        '',
        '<style scoped>',
        `.${ir.name.toLowerCase()} {`,
        '  display: flex;',
        '  flex-direction: column;',
        '  gap: 0.5rem;',
        '}',
        '</style>',
    ].filter((l, i, arr) => !(l === '' && arr[i - 1] === '')).join('\n');
}
function generateSvelte(ir, opts) {
    const props = ir.units.filter(u => u.kind === 'constant').map(u => `  export let ${u.name} = '';`).join('\n');
    const fns = ir.units.filter(u => u.kind === 'function').map(u => {
        const pStr = u.signature.params.map(p => p.name).join(', ');
        return `  function ${u.name}(${pStr}) {\n${(0, BodyTranslator_1.translateBody)(u.originalSource ?? '', ir.sourceLanguage, 'javascript', '    ')}\n  }`;
    }).join('\n\n');
    return [
        `<!-- ${ir.name} — Converted to Svelte -->`,
        `<!-- Source: ${ir.sourceLanguage} -->`,
        '',
        '<script>',
        props, fns,
        '</script>',
        '',
        `<div class="${ir.name.toLowerCase()}">`,
        `  <!-- ${ir.name} -->`,
        '</div>',
        '',
        '<style>',
        `.${ir.name.toLowerCase()} {`,
        '  display: flex;',
        '  flex-direction: column;',
        '  gap: 0.5rem;',
        '}',
        '</style>',
    ].filter((l, i, arr) => !(l === '' && arr[i - 1] === '')).join('\n');
}
function generateAngular(ir, opts) {
    const selector = ir.name.replace(/([A-Z])/g, m => '-' + m.toLowerCase()).replace(/^-/, '');
    return [
        `// ${ir.name} — Converted to Angular`,
        `// Source: ${ir.sourceLanguage}`,
        `import { Component, Input } from '@angular/core';`,
        '',
        `@Component({`,
        `  selector: 'app-${selector}',`,
        `  template: \``,
        `    <div class="${selector}">`,
        `      <!-- ${ir.name} -->`,
        `    </div>`,
        `  \`,`,
        `  styles: [\`.${selector} { display: flex; flex-direction: column; gap: 0.5rem; }\`]`,
        `})`,
        `export class ${ir.name}Component {`,
        ...ir.units.filter(u => u.kind === 'constant').map(u => `  @Input() ${u.name} = '';`),
        '',
        ...ir.units.filter(u => u.kind === 'function').map(u => {
            const pStr = u.signature.params.map(p => `${p.name}: string`).join(', ');
            return `  ${u.name}(${pStr}): void {\n${(0, BodyTranslator_1.translateBody)(u.originalSource ?? '', ir.sourceLanguage, 'typescript', '    ')}\n  }`;
        }),
        '}',
    ].join('\n');
}
function generateAstro(ir, opts) {
    const props = ir.units.filter(u => u.kind === 'constant').map(u => `const { ${u.name} } = Astro.props;`).join('\n');
    return [
        '---',
        `// ${ir.name} — Converted to Astro`,
        `// Source: ${ir.sourceLanguage}`,
        ...ir.imports.map(i => `import { ${i.names.join(', ')} } from '${i.path}';`),
        '',
        props || 'const {} = Astro.props;',
        '---',
        '',
        `<div class="${ir.name.toLowerCase()}">`,
        `  {/* ${ir.name} */}`,
        '</div>',
        '',
        '<style>',
        `.${ir.name.toLowerCase()} {`,
        '  display: flex;',
        '  flex-direction: column;',
        '  gap: 0.5rem;',
        '}',
        '</style>',
    ].join('\n');
}
//# sourceMappingURL=WebFamilyHandler.js.map