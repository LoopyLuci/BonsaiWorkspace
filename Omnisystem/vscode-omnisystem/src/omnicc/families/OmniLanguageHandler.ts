// Omni-Language Handler — Titan, Vera, Nexus, Helix, Aether, Axiom, Sylva
// These are Omnisystem's native languages. Parse and generate with full fidelity.
import {
    ULIRModule, ULIRUnit, ULIRParam, ULIRImport, ULIRMetadata,
    STRING_TYPE, VOID_TYPE, INT_TYPE, FLOAT_TYPE, BOOL_TYPE, ANY_TYPE, UNKNOWN_TYPE,
    DEFAULT_OPTIONS, ConversionOptions,
} from '../ULIR';
import { getLang } from '../LanguageRegistry';
import { translateBody } from '../BodyTranslator';

// ─── Parse ────────────────────────────────────────────────────────────────────

export function parseOmniLanguage(source: string, langId: string): ULIRModule {
    const lang = getLang(langId);
    const lines = source.split('\n');
    const units: ULIRUnit[] = [];
    const imports: ULIRImport[] = [];

    switch (langId) {
        case 'titan': extractTitanUnits(source, lines, units, imports); break;
        case 'vera': extractVeraUnits(source, lines, units, imports); break;
        case 'nexus': extractNexusUnits(source, lines, units, imports); break;
        case 'helix': extractHelixUnits(source, lines, units, imports); break;
        case 'aether': extractAetherUnits(source, lines, units, imports); break;
        case 'axiom': extractAxiomUnits(source, lines, units, imports); break;
        case 'sylva': extractSylvaUnits(source, lines, units, imports); break;
        default: break;
    }

    const meta: ULIRMetadata = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['systems', 'functional'],
        typeSystem: lang?.typing ?? 'static-strong',
        memoryModel: lang?.memory ?? 'gc',
        usesAsync: /\bactor\b|\bmessage\b|\bawait\b|\bspawn\b/.test(source),
        usesGenerics: /<[\w,\s]+>/.test(source),
        usesReflection: false,
        usesMetaprogramming: /\btheorem\b|\bproof\b|\bassert\b|\bcomptime\b/.test(source),
        hasTests: /\b#\[test\]|\btest\s+\w+\b/.test(source),
        hasUI: /\bcomponent\b|\brender\b|\blayout\b|\bwidget\b/.test(source),
        hasSideEffects: /\bprint\b|\blog\b|\bemit\b/.test(source),
        entryPoint: /\bfn main\b/.test(source) ? 'main' : undefined,
    };

    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'omni',
        units,
        imports,
        exports: units.filter(u => u.visibility === 'public').map(u => u.name),
        docComment: '',
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes: [],
    };
}

// Titan: Systems language (Rust-like syntax)
function extractTitanUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    // use imports
    for (const m of source.matchAll(/^use\s+([\w:]+)(?:::\{([^}]+)\})?;/gm)) {
        imports.push({ path: m[1].replace(/::/g, '/'), alias: undefined, names: m[2] ? m[2].split(',').map(n => n.trim()) : [], isDefault: false, isWildcard: !m[2], kind: 'package', originalSyntax: m[0] });
    }
    // fn definitions
    for (const m of source.matchAll(/^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)(?:<([^>]*)>)?\s*\(([^)]*)\)(?:\s*->\s*([\w<>,:&'? ]+))?/gm)) {
        const isPub = source.slice(Math.max(0, (m.index ?? 0) - 4), m.index ?? 0).trimEnd().endsWith('pub');
        const isAsync = m[0].includes('async');
        units.push({
            kind: 'function',
            name: m[1],
            visibility: isPub || m[0].startsWith('pub') ? 'public' : 'private',
            signature: {
                params: parseTitanParams(m[3]),
                returns: m[4] ? { ...UNKNOWN_TYPE, name: m[4].trim(), originalSrc: m[4] } : VOID_TYPE,
                throws: [],
            },
            body: [], attributes: extractTitanAttrs(source, m.index ?? 0),
            docComment: extractTitanDoc(source, m.index ?? 0),
            sourceLines: [0, 0],
            isAsync, isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: m[2] ? m[2].split(',').map(g => ({ name: g.trim(), bounds: [], isVariadic: false })) : [],
            extends_: [], implements_: [], children: [],
            originalSource: m[0], confidence: 'high',
        });
    }
    // struct/enum/actor
    for (const m of source.matchAll(/^(?:pub\s+)?(?:struct|enum|actor|trait)\s+(\w+)/gm)) {
        const kind = m[0].includes('actor') ? 'class' : m[0].includes('enum') ? 'enum' : m[0].includes('trait') ? 'trait' : 'struct';
        units.push(makeOmniUnit(kind as ULIRUnit['kind'], m[1], m[0], m[0].includes('pub') ? 'public' : 'private'));
    }
}

// Vera: UI component language
function extractVeraUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    // import statements
    for (const m of source.matchAll(/^import\s+\{([^}]+)\}\s+from\s+['"]([^'"]+)['"]/gm)) {
        imports.push({ path: m[2], alias: undefined, names: m[1].split(',').map(n => n.trim()), isDefault: false, isWildcard: false, kind: m[2].startsWith('.') ? 'relative' : 'external', originalSyntax: m[0] });
    }
    // component blocks
    for (const m of source.matchAll(/^(?:export\s+)?component\s+(\w+)(?:\s+extends\s+(\w+))?\s*\{/gm)) {
        units.push(makeOmniUnit('widget-component', m[1], m[0], 'public'));
        // Sub-parse props, state, render, handlers
        const body = extractBlock(source, m.index ?? 0);
        for (const pm of body.matchAll(/\bprop\s+(\w+)\s*:\s*([\w<>?,\[\]]+)/g)) {
            units.push(makeOmniUnit('constant', `${m[1]}.${pm[1]}`, pm[0], 'public'));
        }
        for (const sm of body.matchAll(/\bstate\s+(\w+)\s*:\s*([\w<>?,\[\]]+)/g)) {
            units.push(makeOmniUnit('variable', `${m[1]}.${sm[1]}`, sm[0], 'private'));
        }
        for (const hm of body.matchAll(/\bon_(\w+)\s*\([^)]*\)\s*\{/g)) {
            units.push(makeOmniUnit('widget-event', `${m[1]}.on_${hm[1]}`, hm[0], 'private'));
        }
    }
    // Standalone functions
    for (const m of source.matchAll(/^fn\s+(\w+)\s*\(([^)]*)\)/gm)) {
        units.push({
            kind: 'function', name: m[1],
            visibility: 'private',
            signature: { params: parseTitanParams(m[2]), returns: VOID_TYPE, throws: [] },
            body: [], attributes: [], docComment: '',
            sourceLines: [0, 0],
            isAsync: false, isStatic: true, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
            generics: [], extends_: [], implements_: [], children: [],
            originalSource: m[0], confidence: 'medium',
        });
    }
}

// Nexus: Layout language
function extractNexusUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    for (const m of source.matchAll(/^(?:export\s+)?layout\s+(\w+)(?:\s+implements\s+([\w,\s]+))?\s*\{/gm)) {
        units.push(makeOmniUnit('widget-layout', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^breakpoint\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('constant', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^(?:grid|flex|container|row|col|stack)\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('widget-layout', m[1], m[0], 'public'));
    }
}

// Helix: Graphics/shader language
function extractHelixUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    for (const m of source.matchAll(/^(?:pub\s+)?(?:pipeline|shader|compute|vertex|fragment|geometry|tessellation)\s+(\w+)\s*\{/gm)) {
        const kind = m[0].includes('pipeline') ? 'class' : 'function';
        units.push(makeOmniUnit(kind, m[1], m[0], m[0].startsWith('pub') ? 'public' : 'public'));
    }
    for (const m of source.matchAll(/^(?:struct|uniform|buffer)\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('struct', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^fn\s+(\w+)\s*\(([^)]*)\)/gm)) {
        units.push(makeOmniUnit('function', m[1], m[0], 'public'));
    }
}

// Aether: Actor/concurrency language
function extractAetherUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    for (const m of source.matchAll(/^(?:pub\s+)?actor\s+(\w+)(?:\s+extends\s+(\w+))?\s*\{/gm)) {
        units.push(makeOmniUnit('class', m[1], m[0], m[0].startsWith('pub') ? 'public' : 'public'));
        // Find message types in actor
        const body = extractBlock(source, m.index ?? 0);
        for (const mm of body.matchAll(/\bmessage\s+(\w+)\s*\{/g)) {
            units.push(makeOmniUnit('interface', `${m[1]}.${mm[1]}`, mm[0], 'public'));
        }
        for (const hm of body.matchAll(/\bhandler\s+(\w+)\s*\(/g)) {
            units.push(makeOmniUnit('function', `${m[1]}.handler_${hm[1]}`, hm[0], 'private'));
        }
    }
    for (const m of source.matchAll(/^channel\s+(\w+)\s*</gm)) {
        units.push(makeOmniUnit('variable', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^fn\s+(\w+)\s*\(/gm)) {
        units.push(makeOmniUnit('function', m[1], m[0], 'public'));
    }
}

// Axiom: Formal verification language
function extractAxiomUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    for (const m of source.matchAll(/^(?:pub\s+)?theorem\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('function', m[1], m[0], m[0].startsWith('pub') ? 'public' : 'public'));
        const body = extractBlock(source, m.index ?? 0);
        for (const pm of body.matchAll(/\b(?:precondition|postcondition|invariant|assertion)\s*:?\s*(.+)/g)) {
            units.push(makeOmniUnit('constant', `${m[1]}.${pm[0].split(':')[0].trim()}`, pm[0], 'private'));
        }
    }
    for (const m of source.matchAll(/^proof\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('function', `proof_${m[1]}`, m[0], 'public'));
    }
    for (const m of source.matchAll(/^type\s+(\w+)/gm)) {
        units.push(makeOmniUnit('type-alias', m[1], m[0], 'public'));
    }
}

// Sylva: ML/Data science language
function extractSylvaUnits(source: string, lines: string[], units: ULIRUnit[], imports: ULIRImport[]): void {
    for (const m of source.matchAll(/^(?:pub\s+)?model\s+(\w+)(?:\s+extends\s+(\w+))?\s*\{/gm)) {
        units.push(makeOmniUnit('class', m[1], m[0], m[0].startsWith('pub') ? 'public' : 'public'));
    }
    for (const m of source.matchAll(/^layer\s+(\w+)(?:\s+:\s+([\w<>, ]+))?\s*\{/gm)) {
        units.push(makeOmniUnit('class', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^pipeline\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('function', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^fn\s+(\w+)\s*\(([^)]*)\)/gm)) {
        units.push(makeOmniUnit('function', m[1], m[0], 'public'));
    }
    for (const m of source.matchAll(/^dataset\s+(\w+)\s*\{/gm)) {
        units.push(makeOmniUnit('variable', m[1], m[0], 'public'));
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function parseTitanParams(raw: string): ULIRParam[] {
    if (!raw.trim()) { return []; }
    return raw.split(',').map((p): ULIRParam | null => {
        const t = p.trim();
        if (t === '&self' || t === 'self' || t === '&mut self') { return null; }
        const m = t.match(/(\w+)\s*:\s*(&?mut?\s*)?(.+)/);
        if (m) {
            return {
                name: m[1],
                type: { ...UNKNOWN_TYPE, name: m[3].trim(), originalSrc: m[3] },
                defaultValue: undefined,
                isVariadic: t.includes('..'),
                isKeyword: false,
                isRef: t.includes('&'),
                isMut: t.includes('mut'),
            };
        }
        return { name: t.replace(/[^a-zA-Z0-9_]/, '') || 'arg', type: ANY_TYPE, defaultValue: undefined, isVariadic: false, isKeyword: false, isRef: false, isMut: false };
    }).filter((p): p is ULIRParam => p !== null && p.name.length > 0);
}

function extractTitanAttrs(source: string, index: number): string[] {
    const pre = source.slice(Math.max(0, index - 200), index);
    return [...pre.matchAll(/#\[([^\]]+)\]/g)].map(m => `#[${m[1]}]`).slice(-3);
}

function extractTitanDoc(source: string, index: number): string {
    const pre = source.slice(Math.max(0, index - 500), index);
    return [...pre.matchAll(/^\/\/\/\s?(.*)$/gm)].slice(-5).map(m => m[1]).join('\n');
}

function extractBlock(source: string, startIdx: number): string {
    let depth = 0;
    let i = startIdx;
    while (i < source.length) {
        if (source[i] === '{') { depth++; }
        if (source[i] === '}') {
            depth--;
            if (depth <= 0) { return source.slice(startIdx, i); }
        }
        i++;
    }
    return source.slice(startIdx, Math.min(source.length, startIdx + 2000));
}

function makeOmniUnit(kind: ULIRUnit['kind'], name: string, src: string, vis: ULIRUnit['visibility']): ULIRUnit {
    return {
        kind, name, visibility: vis,
        signature: { params: [], returns: VOID_TYPE, throws: [] },
        body: [], attributes: [], docComment: '',
        sourceLines: [0, 0],
        isAsync: false, isStatic: false, isAbstract: false, isFinal: false, isOverride: false, isExtern: false,
        generics: [], extends_: [], implements_: [], children: [],
        originalSource: src.slice(0, 200), confidence: 'high',
    };
}

function detectModuleName(source: string, langId: string): string {
    const m = source.match(/^(?:module|namespace|package)\s+([\w.]+)/m);
    if (m) { return m[1].split('.').pop() ?? m[1]; }
    // Vera: component name
    const c = source.match(/component\s+(\w+)/);
    if (c) { return c[1]; }
    // Aether: actor name
    const a = source.match(/actor\s+(\w+)/);
    if (a) { return a[1]; }
    // Sylva: model name
    const s = source.match(/model\s+(\w+)/);
    if (s) { return s[1]; }
    // Axiom: theorem name
    const t = source.match(/theorem\s+(\w+)/);
    if (t) { return t[1]; }
    return 'OmniModule';
}

// ─── Generate ─────────────────────────────────────────────────────────────────

export function generateOmniLanguage(ir: ULIRModule, targetLangId: string, opts: ConversionOptions = DEFAULT_OPTIONS): string {
    switch (targetLangId) {
        case 'titan': return generateTitan(ir, opts);
        case 'vera': return generateVera(ir, opts);
        case 'nexus': return generateNexus(ir, opts);
        case 'helix': return generateHelix(ir, opts);
        case 'aether': return generateAether(ir, opts);
        case 'axiom': return generateAxiom(ir, opts);
        case 'sylva': return generateSylva(ir, opts);
        default: return generateTitan(ir, opts);
    }
}

function generateTitan(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Titan`, `// Source: ${ir.sourceLanguage}`, ''];
    for (const imp of ir.imports) {
        const names = imp.names.length > 0 ? `::{ ${imp.names.join(', ')} }` : '';
        lines.push(`use ${imp.path.replace(/\//g, '::')}${names};`);
    }
    if (ir.imports.length > 0) { lines.push(''); }

    for (const unit of ir.units) {
        const body = translateBody(unit.originalSource ?? '', ir.sourceLanguage, 'titan');
        const vis = unit.visibility === 'public' ? 'pub ' : '';
        // Faithful type text: prefer a known return/declared type, else preserve
        // the source type text, else fall back to Titan's most permissive type.
        const declType =
            unit.signature.returns.name !== 'void' && unit.signature.returns.name !== 'Unknown'
                ? unit.signature.returns.name
                : (unit.signature.returns.originalSrc && unit.signature.returns.originalSrc !== '?'
                    ? unit.signature.returns.originalSrc
                    : 'Any');
        switch (unit.kind) {
            case 'struct':
            case 'record':
            case 'tuple-type':
                lines.push(`pub struct ${unit.name} {\n${body}\n}`);
                break;
            case 'enum':
            case 'union':
            case 'tagged-union':
                // Titan models sum types as enums (tagged unions).
                lines.push(`pub enum ${unit.name} {\n${body}\n}`);
                break;
            case 'trait':
            case 'interface':
            case 'protocol':
            case 'mixin':
                // Titan expresses all abstract contracts as traits.
                lines.push(`pub trait ${unit.name} {\n${body}\n}`);
                break;
            case 'class':
            case 'actor':
                lines.push(`pub actor ${unit.name} {\n${body}\n}`);
                break;
            case 'type-alias':
            case 'newtype':
            case 'opaque-type':
                lines.push(`${vis}type ${unit.name} = ${declType};`);
                break;
            case 'constant':
                lines.push(`${vis}const ${unit.name}: ${declType} = ${constInit(unit)};`);
                break;
            case 'variable':
            case 'field':
            case 'property':
                lines.push(`${vis}let ${unit.isFinal ? '' : 'mut '}${unit.name}: ${declType};`);
                break;
            case 'namespace':
            case 'module-decl':
            case 'package-decl': {
                // Recurse so nested items aren't lost — a namespace with no
                // faithful expansion would otherwise silently drop its children.
                const nested = unit.children.length > 0
                    ? generateTitan({ ...ir, name: unit.name, units: unit.children, imports: [] }, opts)
                        .split('\n').map(l => l ? `    ${l}` : l).join('\n')
                    : body;
                lines.push(`pub mod ${unit.name} {\n${nested}\n}`);
                break;
            }
            case 'macro':
                // Titan has no macro syntax yet; preserve intent explicitly as an
                // annotated fn rather than silently discarding the construct.
                lines.push(`#[macro]\n${vis}fn ${unit.name}() {\n${body}\n}`);
                break;
            case 'theorem':
            case 'proof':
            case 'axiom-decl':
            case 'invariant':
                // Formal units belong to Axiom; when the target is Titan, keep
                // them as a checked assertion fn so the guarantee isn't lost.
                lines.push(`// formal: originally a ${unit.kind} (see Axiom target for full form)\n${vis}fn assert_${unit.name}() {\n${body}\n}`);
                break;
            default: {
                const async_ = unit.isAsync ? 'async ' : '';
                const pStr = unit.signature.params.map(p => `${p.name}: ${p.type.name !== 'Unknown' ? p.type.name : 'Any'}`).join(', ');
                const ret = unit.signature.returns.name !== 'void' && unit.signature.returns.name !== 'Unknown'
                    ? ` -> ${unit.signature.returns.name}` : '';
                lines.push(`${vis}${async_}fn ${unit.name}(${pStr})${ret} {\n${body}\n}`);
            }
        }
        lines.push('');
    }
    return lines.join('\n');
}

/** Best-effort initializer for a constant, preserving the source value when
 *  present rather than fabricating one. */
function constInit(unit: { originalSource?: string }): string {
    const src = unit.originalSource ?? '';
    const eq = src.indexOf('=');
    if (eq >= 0) {
        const rhs = src.slice(eq + 1).replace(/;+\s*$/, '').trim();
        if (rhs) { return rhs; }
    }
    return 'Default::default()';
}

function generateVera(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Vera UI Component`, `// Source: ${ir.sourceLanguage}`, ''];
    const components = ir.units.filter(u => u.kind === 'widget-component' || u.kind === 'class');
    const fns = ir.units.filter(u => u.kind === 'function');
    const consts = ir.units.filter(u => u.kind === 'constant');
    const events = ir.units.filter(u => u.kind === 'widget-event');

    const compName = components[0]?.name ?? ir.name;
    lines.push(`export component ${compName} {`);

    // Props from constants
    for (const c of consts) {
        lines.push(`    prop ${c.name}: String = ""`);
    }

    // State from events
    if (events.length > 0) {
        lines.push('');
        for (const e of events) {
            lines.push(`    state is_${e.name}_active: Bool = false`);
        }
    }

    lines.push('');
    lines.push('    render {');
    lines.push(`        div .${compName.toLowerCase()} {`);
    if (fns.length > 0) {
        lines.push(`            // ${fns.map(f => f.name).join(', ')}`);
    }
    lines.push('        }');
    lines.push('    }');

    // Event handlers
    if (events.length > 0) {
        lines.push('');
        for (const e of events) {
            const body = translateBody(e.originalSource ?? '', ir.sourceLanguage, 'titan', '        ');
            lines.push(`    on_${e.name}() {\n${body}\n    }`);
        }
    }
    lines.push('}');

    // Standalone functions
    for (const fn of fns) {
        const pStr = fn.signature.params.map(p => `${p.name}: String`).join(', ');
        const fnBody = translateBody(fn.originalSource ?? '', ir.sourceLanguage, 'titan');
        lines.push('', `fn ${fn.name}(${pStr}) {`, fnBody || '    return ()', '}');
    }

    return lines.join('\n');
}

function generateNexus(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Nexus Layout`, `// Source: ${ir.sourceLanguage}`, ''];
    const layouts = ir.units.filter(u => u.kind === 'widget-layout');
    const layoutName = layouts[0]?.name ?? ir.name;

    lines.push(`export layout ${layoutName} {`);
    lines.push('    breakpoints {');
    lines.push('        sm: 640px;');
    lines.push('        md: 768px;');
    lines.push('        lg: 1024px;');
    lines.push('        xl: 1280px;');
    lines.push('    }');
    lines.push('');
    lines.push('    container .main {');
    lines.push('        max_width: 1200px;');
    lines.push('        padding: 16px;');
    lines.push('    }');

    for (const unit of ir.units.filter(u => u.kind !== 'widget-layout')) {
        lines.push('');
        lines.push(`    grid .${unit.name.toLowerCase()} {`);
        lines.push('        columns: 12;');
        lines.push('        gap: 16px;');
        lines.push('    }');
    }
    lines.push('}');
    return lines.join('\n');
}

function generateHelix(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Helix Shader`, `// Source: ${ir.sourceLanguage}`, ''];
    lines.push(`pipeline ${ir.name}Pipeline {`);
    lines.push('    inputs {');
    lines.push('        position: Vec4;');
    lines.push('        color: Vec4;');
    lines.push('    }');
    lines.push('    outputs {');
    lines.push('        frag_color: Vec4;');
    lines.push('    }');
    lines.push('    shaders {');
    lines.push('        vertex: vertex_main;');
    lines.push('        fragment: fragment_main;');
    lines.push('    }');
    lines.push('}');
    lines.push('');

    for (const unit of ir.units.filter(u => u.kind === 'function')) {
        const pStr = unit.signature.params.map(p => `${p.name}: Vec4`).join(', ');
        const helixBody = translateBody(unit.originalSource ?? '', ir.sourceLanguage, 'helix', '    ');
        lines.push(`fn ${unit.name}(${pStr}) -> Vec4 {`);
        lines.push(helixBody || '    return vec4(0.0, 0.0, 0.0, 1.0);');
        lines.push('}');
        lines.push('');
    }
    return lines.join('\n');
}

function generateAether(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Aether Actor`, `// Source: ${ir.sourceLanguage}`, ''];
    const actorName = ir.units.find(u => u.kind === 'class')?.name ?? ir.name;
    lines.push(`pub actor ${actorName} {`);

    // State fields from constants
    for (const c of ir.units.filter(u => u.kind === 'constant')) {
        lines.push(`    let ${c.name}: String`);
    }
    lines.push('');

    // Messages
    const msgs = ir.units.filter(u => u.kind === 'interface');
    for (const msg of msgs) {
        lines.push(`    message ${msg.name.split('.').pop()} {`);
        lines.push('        data: String');
        lines.push('    }');
    }
    if (msgs.length === 0) {
        lines.push('    message Process { data: String }');
    }
    lines.push('');

    // Handlers
    for (const msg of msgs) {
        const msgName = msg.name.split('.').pop() ?? 'Process';
        const handlerBody = translateBody(msg.originalSource ?? '', ir.sourceLanguage, 'aether', '        ');
        lines.push(`    handler ${msgName}(msg: ${msgName}) {`);
        lines.push(handlerBody || `        self.log(msg.data)`);
        lines.push('    }');
    }
    if (msgs.length === 0) {
        lines.push('    handler Process(msg: Process) {');
        lines.push('        self.log(msg.data)');
        lines.push('    }');
    }

    lines.push('}');
    return lines.join('\n');
}

function generateAxiom(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Axiom Formal Verification`, `// Source: ${ir.sourceLanguage}`, ''];
    const fns = ir.units.filter(u => u.kind === 'function');

    for (const fn of fns) {
        lines.push(`theorem ${fn.name}_correctness {`);
        lines.push('    preconditions {');
        lines.push(`        // input assumptions for ${fn.name}`);
        lines.push('    }');
        lines.push('    postconditions {');
        lines.push(`        // output guarantees for ${fn.name}`);
        lines.push('    }');
        lines.push('    invariants {');
        lines.push(`        // state invariants for ${fn.name}`);
        lines.push('    }');
        lines.push('}');
        lines.push('');
    }

    if (fns.length === 0) {
        lines.push('theorem module_correctness {');
        lines.push('    preconditions {}');
        lines.push('    postconditions {}');
        lines.push('    invariants {}');
        lines.push('}');
    }
    return lines.join('\n');
}

function generateSylva(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Sylva ML Model`, `// Source: ${ir.sourceLanguage}`, ''];
    const modelName = ir.units.find(u => u.kind === 'class')?.name ?? `${ir.name}Model`;

    lines.push(`pub model ${modelName} {`);
    lines.push('    architecture: [');
    lines.push('        Dense { units: 128, activation: relu }');
    lines.push('        Dropout { rate: 0.2 }');
    lines.push('        Dense { units: 64, activation: relu }');
    lines.push('        Dense { units: 10, activation: softmax }');
    lines.push('    ]');
    lines.push('');
    lines.push('    optimizer: Adam { lr: 0.001 }');
    lines.push('    loss: cross_entropy');
    lines.push('    metrics: [accuracy, f1_score]');
    lines.push('}');
    lines.push('');

    // Pipeline from existing functions
    const fns = ir.units.filter(u => u.kind === 'function');
    if (fns.length > 0) {
        lines.push(`pipeline ${ir.name}Pipeline {`);
        for (const fn of fns) {
            lines.push(`    step ${fn.name} { }`);
        }
        lines.push('}');
    }
    return lines.join('\n');
}
