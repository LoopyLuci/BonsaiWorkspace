// Omni Language Parser — parses Vera, Nexus, and Titan widget definitions
import {
    WidgetIR, WidgetNode, WidgetKind, WidgetEvent, WidgetProp, WidgetStyle,
    ConversionConfidence, makeId, SourceLanguage,
} from '../WidgetIR';

// ─── Vera Parser ──────────────────────────────────────────────────────────────

interface VeraComponent {
    name: string;
    props: WidgetProp[];
    state: Record<string, string>;
    events: WidgetEvent[];
    renderBody: string;
}

function parseVeraProps(propsBlock: string): WidgetProp[] {
    const props: WidgetProp[] = [];
    const lineRx = /(\w+)\s*:\s*([\w<>, ]+)\s*(?:=\s*([^\n,]+))?/g;
    let m: RegExpExecArray | null;
    while ((m = lineRx.exec(propsBlock)) !== null) {
        const name = m[1].trim();
        const type = m[2].trim();
        const value = m[3]?.trim().replace(/^["']|["']$/g, '');
        props.push({ name, type, value, required: !value });
    }
    return props;
}

function parseVeraEvents(src: string): WidgetEvent[] {
    const events: WidgetEvent[] = [];
    // on:click, on:change, on:submit, on:focus, on:blur in render block
    const evRx = /on:(\w+)\s*=\s*\{([^}]+)\}/g;
    let m: RegExpExecArray | null;
    while ((m = evRx.exec(src)) !== null) {
        const evName = 'on' + m[1].charAt(0).toUpperCase() + m[1].slice(1);
        events.push({ name: evName, handler: m[2].trim().slice(0, 200), params: [] });
    }
    // Also detect fn handlers
    const handlerRx = /fn\s+(handle_\w+|on_\w+)\s*\(([^)]*)\)/g;
    while ((m = handlerRx.exec(src)) !== null) {
        const fn = m[1];
        const params = m[2].split(',').map(p => p.trim()).filter(Boolean);
        const evName = fn.replace(/^handle_/, 'on').replace(/_(\w)/g, (_, c) => c.toUpperCase());
        events.push({ name: evName, handler: fn, params });
    }
    return events;
}

function inferKindFromRender(renderBody: string): WidgetKind {
    if (/<button/i.test(renderBody))                                   { return 'button'; }
    if (/<input/i.test(renderBody))                                    { return 'input'; }
    if (/<textarea/i.test(renderBody))                                 { return 'textarea'; }
    if (/<select/i.test(renderBody))                                   { return 'select'; }
    if (/class=["'][^"']*checkbox/i.test(renderBody))                  { return 'checkbox'; }
    if (/class=["'][^"']*toggle|role=["']switch/i.test(renderBody))    { return 'toggle'; }
    if (/<slider|class=["'][^"']*slider/i.test(renderBody))            { return 'slider'; }
    if (/class=["'][^"']*modal/i.test(renderBody))                     { return 'modal'; }
    if (/role=["']tablist/i.test(renderBody))                          { return 'tabgroup'; }
    if (/role=["']tab['"]/i.test(renderBody))                          { return 'tab'; }
    if (/class=["'][^"']*card/i.test(renderBody))                      { return 'card'; }
    if (/class=["'][^"']*badge/i.test(renderBody))                     { return 'badge'; }
    if (/class=["'][^"']*progress/i.test(renderBody))                  { return 'progress'; }
    if (/class=["'][^"']*spinner|class=["'][^"']*loader/i.test(renderBody)) { return 'spinner'; }
    if (/<nav|class=["'][^"']*nav/i.test(renderBody))                  { return 'navbar'; }
    if (/<ul|<ol|class=["'][^"']*list/i.test(renderBody))             { return 'list'; }
    if (/<table/i.test(renderBody))                                    { return 'table'; }
    if (/<form/i.test(renderBody))                                     { return 'form'; }
    if (/class=["'][^"']*alert/i.test(renderBody))                     { return 'alert'; }
    if (/class=["'][^"']*toast/i.test(renderBody))                     { return 'toast'; }
    return 'container';
}

function extractBlock(src: string, keyword: string): string {
    const rx = new RegExp(`\\b${keyword}\\s*\\{`, 'i');
    const start = src.search(rx);
    if (start < 0) { return ''; }
    let depth = 0;
    let i = src.indexOf('{', start);
    const begin = i + 1;
    while (i < src.length) {
        if (src[i] === '{') { depth++; }
        else if (src[i] === '}') {
            depth--;
            if (depth === 0) { return src.slice(begin, i); }
        }
        i++;
    }
    return '';
}

function parseVera(src: string): VeraComponent | null {
    const compRx = /component\s+(\w+)\s*\{/;
    const m = src.match(compRx);
    if (!m) { return null; }
    const name = m[1];

    const propsBlock = extractBlock(src, 'props');
    const renderBlock = extractBlock(src, 'render');
    const props = parseVeraProps(propsBlock);
    const events = parseVeraEvents(src);
    const state: Record<string, string> = {};
    const stateBlock = extractBlock(src, 'state');
    const stateRx = /(\w+)\s*:\s*[\w<>, ]+\s*=\s*([^\n]+)/g;
    let sm: RegExpExecArray | null;
    while ((sm = stateRx.exec(stateBlock)) !== null) {
        state[sm[1]] = sm[2].trim();
    }

    return { name, props, state, events, renderBody: renderBlock };
}

// ─── Nexus Parser ─────────────────────────────────────────────────────────────

interface NexusLayout {
    name: string;
    breakpoints: Record<string, string>;
    rules: string[];
    children: string[];
}

function parseNexus(src: string): NexusLayout | null {
    const layoutRx = /layout\s+(\w+)\s*\{/;
    const m = src.match(layoutRx);
    if (!m) { return null; }
    const name = m[1];

    const breakpointBlock = extractBlock(src, 'breakpoints');
    const breakpoints: Record<string, string> = {};
    const bpRx = /(\w+)\s*:\s*(\d+px)/g;
    let bm: RegExpExecArray | null;
    while ((bm = bpRx.exec(breakpointBlock)) !== null) {
        breakpoints[bm[1]] = bm[2];
    }

    // Extract flex/grid rules
    const rules: string[] = [];
    const ruleRx = /\b(flex|grid|column|row|gap-\d+|align-\w+|justify-\w+|p-\d+|m-\d+)\b/g;
    let rm: RegExpExecArray | null;
    while ((rm = ruleRx.exec(src)) !== null) {
        if (!rules.includes(rm[1])) { rules.push(rm[1]); }
    }

    return { name, breakpoints, rules, children: [] };
}

// ─── Titan Parser ─────────────────────────────────────────────────────────────

interface TitanStruct {
    name: string;
    fields: WidgetProp[];
    functions: Array<{ name: string; params: string[]; body: string }>;
}

function parseTitan(src: string): TitanStruct | null {
    const structRx = /pub\s+struct\s+(\w+)\s*\{([^}]+)\}/s;
    const m = src.match(structRx);
    if (!m) {
        // Try fn-only
        const fnRx = /pub\s+fn\s+(\w+)\s*\(/;
        const fm = src.match(fnRx);
        if (!fm) { return null; }
        return { name: fm[1], fields: [], functions: [] };
    }

    const name = m[1];
    const fieldBlock = m[2];
    const fields: WidgetProp[] = [];
    const fieldRx = /(\w+)\s*:\s*([\w<>&' ]+)(?:,|$)/g;
    let fm: RegExpExecArray | null;
    while ((fm = fieldRx.exec(fieldBlock)) !== null) {
        fields.push({ name: fm[1].trim(), type: fm[2].trim() });
    }

    const functions: Array<{ name: string; params: string[]; body: string }> = [];
    const fnRx = /pub\s+fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*[\w<>&' ]+)?\s*\{/g;
    let fnm: RegExpExecArray | null;
    while ((fnm = fnRx.exec(src)) !== null) {
        const params = fnm[2].split(',').map(p => p.trim()).filter(Boolean);
        functions.push({ name: fnm[1], params, body: '' });
    }

    return { name, fields, functions };
}

// ─── Main export ──────────────────────────────────────────────────────────────

export function parseOmni(source: string, lang: SourceLanguage): WidgetIR {
    const src = source.trim();
    const notes: string[] = [];

    if (lang === 'vera') {
        const comp = parseVera(src);
        if (comp) {
            const kind = inferKindFromRender(comp.renderBody);
            notes.push(`Vera component: "${comp.name}"`);
            if (comp.props.length > 0) { notes.push(`${comp.props.length} props extracted`); }
            if (comp.events.length > 0) { notes.push(`${comp.events.length} event handlers`); }

            const node: WidgetNode = {
                id: makeId(comp.name),
                kind,
                name: comp.name,
                props: comp.props,
                events: comp.events,
                raw: src.slice(0, 300),
            };

            return {
                name: comp.name,
                rootWidget: node,
                sourceLanguage: 'vera',
                confidence: 'high',
                notes,
            };
        }
    }

    if (lang === 'nexus') {
        const layout = parseNexus(src);
        if (layout) {
            notes.push(`Nexus layout: "${layout.name}"`);
            notes.push(`Breakpoints: ${Object.keys(layout.breakpoints).join(', ') || 'none'}`);
            notes.push(`Layout rules: ${layout.rules.join(', ') || 'none'}`);

            const node: WidgetNode = {
                id: makeId(layout.name),
                kind: 'panel',
                name: layout.name,
                meta: {
                    breakpoints: JSON.stringify(layout.breakpoints),
                    rules: layout.rules.join(' '),
                },
            };

            return {
                name: layout.name,
                rootWidget: node,
                sourceLanguage: 'nexus',
                confidence: 'high',
                notes,
            };
        }
    }

    if (lang === 'titan') {
        const struct = parseTitan(src);
        if (struct) {
            notes.push(`Titan struct/module: "${struct.name}"`);
            if (struct.fields.length > 0) { notes.push(`${struct.fields.length} fields`); }
            if (struct.functions.length > 0) { notes.push(`${struct.functions.length} functions`); }

            // Infer widget kind from struct name
            const kindMap: [RegExp, WidgetKind][] = [
                [/Button/i, 'button'], [/Input/i, 'input'], [/Toggle/i, 'toggle'],
                [/Modal/i, 'modal'], [/Card/i, 'card'], [/Panel/i, 'panel'],
                [/List/i, 'list'], [/Table/i, 'table'], [/Form/i, 'form'],
                [/Badge/i, 'badge'], [/Toast/i, 'toast'], [/Alert/i, 'alert'],
                [/Nav/i, 'navbar'], [/Slider/i, 'slider'], [/Progress/i, 'progress'],
            ];
            let kind: WidgetKind = 'container';
            for (const [rx, k] of kindMap) {
                if (rx.test(struct.name)) { kind = k; break; }
            }

            const node: WidgetNode = {
                id: makeId(struct.name),
                kind,
                name: struct.name,
                props: struct.fields,
                events: struct.functions.map(f => ({
                    name: 'on' + f.name.charAt(0).toUpperCase() + f.name.slice(1),
                    handler: f.name,
                    params: f.params,
                })),
            };

            return {
                name: struct.name,
                rootWidget: node,
                sourceLanguage: 'titan',
                confidence: kind !== 'container' ? 'high' : 'medium',
                notes,
            };
        }
    }

    return {
        name: 'OmniWidget',
        rootWidget: { id: 'omni_widget', kind: 'unknown', name: 'OmniWidget' },
        sourceLanguage: lang,
        confidence: 'low',
        notes: [`Could not parse ${lang.toUpperCase()} source`],
    };
}
