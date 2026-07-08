// Data-Family Handler — SQL, JSON, YAML, TOML, XML, GraphQL, HCL, Protobuf, CSV, INI, NDJSON
import {
    ULIRModule, ULIRUnit, ULIRImport, ULIRMetadata,
    STRING_TYPE, VOID_TYPE, INT_TYPE, FLOAT_TYPE, BOOL_TYPE, ANY_TYPE, UNKNOWN_TYPE,
    DEFAULT_OPTIONS, ConversionOptions,
} from '../ULIR';
import { getLang } from '../LanguageRegistry';
import { translateBody } from '../BodyTranslator';

// ─── Parse ────────────────────────────────────────────────────────────────────

export function parseDataFamily(source: string, langId: string): ULIRModule {
    const lang = getLang(langId);
    const lines = source.split('\n');
    const units: ULIRUnit[] = [];

    switch (langId) {
        case 'sql': extractSQLUnits(source, units); break;
        case 'graphql': extractGraphQLUnits(source, units); break;
        case 'json': extractJSONUnits(source, units); break;
        case 'yaml': case 'yml': extractYAMLUnits(source, lines, units); break;
        case 'toml': extractTOMLUnits(source, lines, units); break;
        case 'xml': extractXMLUnits(source, units); break;
        case 'protobuf': extractProtobufUnits(source, units); break;
        case 'hcl': extractHCLUnits(source, units); break;
        default: extractGenericDataUnits(source, lines, units); break;
    }

    const meta: ULIRMetadata = {
        sourceLines: lines.length,
        paradigms: lang?.paradigms ?? ['declarative', 'data'],
        typeSystem: lang?.typing ?? 'structural',
        memoryModel: 'none',
        usesAsync: false, usesGenerics: false, usesReflection: false,
        usesMetaprogramming: false, hasTests: false, hasUI: false, hasSideEffects: false,
    };

    return {
        name: detectModuleName(source, langId),
        sourceLanguage: langId,
        sourceFamily: 'data',
        units,
        imports: [],
        exports: units.map(u => u.name),
        docComment: '',
        metadata: meta,
        confidence: units.length > 0 ? 'high' : 'medium',
        notes: [],
    };
}

function extractSQLUnits(source: string, units: ULIRUnit[]): void {
    // Tables
    for (const m of source.matchAll(/CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)\s*\(([^;]+)\)/gi)) {
        const fields = m[2].split(',').map(f => f.trim()).filter(Boolean);
        units.push(makeDataUnit('table', m[1], fields, m[0]));
    }
    // Views
    for (const m of source.matchAll(/CREATE\s+(?:OR\s+REPLACE\s+)?VIEW\s+(\w+)\s+AS\s+([^;]+)/gi)) {
        units.push(makeDataUnit('variable', m[1], [], m[0]));
    }
    // Procedures/functions
    for (const m of source.matchAll(/CREATE\s+(?:OR\s+REPLACE\s+)?(?:PROCEDURE|FUNCTION)\s+(\w+)\s*\(([^)]*)\)/gi)) {
        units.push(makeDataUnit('function', m[1], [], m[0]));
    }
    // INSERT templates
    for (const m of source.matchAll(/INSERT\s+INTO\s+(\w+)\s*\(([^)]+)\)/gi)) {
        units.push(makeDataUnit('variable', `insert_${m[1]}`, m[2].split(',').map(s => s.trim()), m[0]));
    }
}

function extractGraphQLUnits(source: string, units: ULIRUnit[]): void {
    // Types, inputs, enums, unions, interfaces
    for (const m of source.matchAll(/(?:type|input|enum|union|interface|scalar)\s+(\w+)(?:\s+(?:implements|on)\s+([\w\s|&]+))?\s*\{([^}]*)\}/g)) {
        const kind = source.slice(m.index ?? 0, (m.index ?? 0) + 10).match(/(type|input|enum|union|interface|scalar)/)?.[1];
        const fields = m[3].split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('#'));
        units.push(makeDataUnit(kind === 'enum' ? 'enum' : 'interface', m[1], fields, m[0]));
    }
    // Queries, Mutations, Subscriptions
    for (const m of source.matchAll(/(?:query|mutation|subscription)\s+(\w+)\s*(?:\([^)]*\))?\s*\{/g)) {
        units.push(makeDataUnit('function', m[1], [], m[0]));
    }
}

function extractJSONUnits(source: string, units: ULIRUnit[]): void {
    try {
        // We can't use JSON.parse safely without try/catch; extract top-level keys
        const keys = [...source.matchAll(/"(\w+)"\s*:/g)].map(m => m[1]).slice(0, 50);
        const uniqueKeys = [...new Set(keys)];
        if (uniqueKeys.length > 0) {
            units.push(makeDataUnit('variable', 'root', uniqueKeys, source.slice(0, 200)));
        }
    } catch { /* malformed JSON */ }
}

function extractYAMLUnits(source: string, lines: string[], units: ULIRUnit[]): void {
    // Top-level keys
    const topKeys: string[] = [];
    for (const line of lines) {
        const m = line.match(/^([a-zA-Z_][\w-]*)\s*:/);
        if (m && !line.startsWith(' ') && !line.startsWith('\t')) {
            topKeys.push(m[1]);
        }
    }
    if (topKeys.length > 0) {
        units.push(makeDataUnit('variable', 'document', topKeys, source.slice(0, 200)));
    }
    // YAML anchors as named constants
    for (const m of source.matchAll(/&(\w+)\s/g)) {
        units.push(makeDataUnit('constant', m[1], [], m[0]));
    }
}

function extractTOMLUnits(source: string, lines: string[], units: ULIRUnit[]): void {
    // Sections [section] and [[array-section]]
    for (const m of source.matchAll(/^\[{1,2}([\w.]+)\]{1,2}/gm)) {
        units.push(makeDataUnit('variable', m[1].replace(/\./g, '_'), [], m[0]));
    }
}

function extractXMLUnits(source: string, units: ULIRUnit[]): void {
    // Root element + major child elements
    for (const m of source.matchAll(/<(\w[\w:-]*)(?:\s[^>]*)?>(?!\s*<\/)/g)) {
        if (!['?xml', '!DOCTYPE', '!--'].includes(m[1])) {
            units.push(makeDataUnit('variable', m[1], [], m[0]));
        }
    }
}

function extractProtobufUnits(source: string, units: ULIRUnit[]): void {
    for (const m of source.matchAll(/message\s+(\w+)\s*\{([^}]*)\}/g)) {
        const fields = m[2].split(';').map(f => f.trim()).filter(Boolean);
        units.push(makeDataUnit('interface', m[1], fields, m[0]));
    }
    for (const m of source.matchAll(/enum\s+(\w+)\s*\{/g)) {
        units.push(makeDataUnit('enum', m[1], [], m[0]));
    }
    for (const m of source.matchAll(/(?:rpc|service)\s+(\w+)/g)) {
        units.push(makeDataUnit('function', m[1], [], m[0]));
    }
}

function extractHCLUnits(source: string, units: ULIRUnit[]): void {
    // Terraform resources, data sources, variables, locals
    for (const m of source.matchAll(/^(?:resource|data|module)\s+"([\w_]+)"\s+"([\w_]+)"/gm)) {
        units.push(makeDataUnit('variable', `${m[1]}_${m[2]}`, [], m[0]));
    }
    for (const m of source.matchAll(/^variable\s+"([\w_]+)"/gm)) {
        units.push(makeDataUnit('constant', m[1], [], m[0]));
    }
    for (const m of source.matchAll(/^output\s+"([\w_]+)"/gm)) {
        units.push(makeDataUnit('variable', `output_${m[1]}`, [], m[0]));
    }
}

function extractGenericDataUnits(source: string, lines: string[], units: ULIRUnit[]): void {
    // INI sections
    for (const m of source.matchAll(/^\[([^\]]+)\]/gm)) {
        units.push(makeDataUnit('variable', m[1], [], m[0]));
    }
}

function makeDataUnit(kind: ULIRUnit['kind'], name: string, fields: string[], src: string): ULIRUnit {
    return {
        kind,
        name: sanitizeName(name),
        visibility: 'public',
        signature: { params: [], returns: VOID_TYPE, throws: [] },
        body: [],
        attributes: fields.length > 0 ? [`fields: ${fields.slice(0, 5).join(', ')}${fields.length > 5 ? ', ...' : ''}`] : [],
        docComment: '',
        sourceLines: [0, 0],
        isAsync: false, isStatic: false, isAbstract: false,
        isFinal: false, isOverride: false, isExtern: false,
        generics: [], extends_: [], implements_: [], children: [],
        originalSource: src.slice(0, 300),
        confidence: 'high',
    };
}

function sanitizeName(name: string): string {
    return name.replace(/[^a-zA-Z0-9_]/g, '_').replace(/^(\d)/, '_$1') || 'unnamed';
}

function detectModuleName(source: string, langId: string): string {
    if (langId === 'graphql') {
        const m = source.match(/schema\s*\{/) ? 'Schema' : 'GraphQL';
        return m;
    }
    if (langId === 'sql') {
        const m = source.match(/(?:DATABASE|SCHEMA)\s+(\w+)/i);
        if (m) { return m[1]; }
    }
    if (langId === 'hcl') {
        const m = source.match(/terraform\s*\{\s*(?:[^}]*\n)*\s*required_providers/);
        return m ? 'TerraformConfig' : 'Infrastructure';
    }
    return langId.toUpperCase();
}

// ─── Generate ─────────────────────────────────────────────────────────────────

export function generateDataFamily(ir: ULIRModule, targetLangId: string, opts: ConversionOptions = DEFAULT_OPTIONS): string {
    switch (targetLangId) {
        case 'sql': return generateSQL(ir, opts);
        case 'graphql': return generateGraphQL(ir, opts);
        case 'json': return generateJSON(ir, opts);
        case 'yaml': return generateYAML(ir, opts);
        case 'toml': return generateTOML(ir, opts);
        case 'xml': return generateXML(ir, opts);
        case 'protobuf': return generateProtobuf(ir, opts);
        case 'hcl': return generateHCL(ir, opts);
        default: return generateJSON(ir, opts);
    }
}

function generateSQL(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`-- ${ir.name} — Converted to SQL`, `-- Source: ${ir.sourceLanguage}`, ''];
    for (const unit of ir.units) {
        if (unit.kind === 'interface' || unit.kind === 'class' || unit.kind === 'struct' || unit.kind === 'table' as any) {
            const fields = unit.attributes.length > 0
                ? unit.attributes[0].replace('fields: ', '').split(', ').map(f => `    ${sanitizeName(f)} TEXT`).join(',\n')
                : '    id BIGSERIAL PRIMARY KEY';
            lines.push(`CREATE TABLE IF NOT EXISTS ${unit.name} (\n${fields}\n);`);
        } else if (unit.kind === 'function') {
            lines.push(`-- Procedure: ${unit.name}`);
            lines.push(`CREATE OR REPLACE PROCEDURE ${unit.name}()`);
            lines.push(`LANGUAGE plpgsql AS $$\nBEGIN\n${translateBody(unit.originalSource ?? '', ir.sourceLanguage, 'sql', '  ')}\nEND;\n$$;`);
        } else {
            lines.push(`-- ${unit.name}: ${unit.kind}`);
        }
        lines.push('');
    }
    return lines.join('\n');
}

function generateGraphQL(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`# ${ir.name} — Converted to GraphQL`, `# Source: ${ir.sourceLanguage}`, ''];
    for (const unit of ir.units) {
        const gqlKind = unit.kind === 'enum' ? 'enum' : unit.kind === 'function' ? 'type' : 'type';
        const fields = unit.attributes.length > 0
            ? unit.attributes[0].replace('fields: ', '').split(', ').map(f => `  ${sanitizeName(f)}: String`).join('\n')
            : '  id: ID!';
        lines.push(`${gqlKind} ${unit.name} {\n${fields}\n}`);
        lines.push('');
    }
    return lines.join('\n');
}

function generateJSON(ir: ULIRModule, opts: ConversionOptions): string {
    const obj: Record<string, unknown> = {
        $schema: `omnisystem://converted/${ir.sourceLanguage}`,
        name: ir.name,
        source: ir.sourceLanguage,
        units: ir.units.map(u => ({
            kind: u.kind,
            name: u.name,
            fields: u.attributes[0]?.replace('fields: ', '').split(', ') ?? [],
        })),
    };
    return JSON.stringify(obj, null, 2);
}

function generateYAML(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`# ${ir.name} — Converted to YAML`, `# Source: ${ir.sourceLanguage}`, '', `name: ${ir.name}`, `source: ${ir.sourceLanguage}`, 'units:'];
    for (const unit of ir.units) {
        lines.push(`  - kind: ${unit.kind}`);
        lines.push(`    name: ${unit.name}`);
        if (unit.attributes.length > 0) {
            const fields = unit.attributes[0].replace('fields: ', '').split(', ');
            lines.push(`    fields:`);
            for (const f of fields) { lines.push(`      - ${f}`); }
        }
    }
    return lines.join('\n');
}

function generateTOML(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`# ${ir.name} — Converted to TOML`, `# Source: ${ir.sourceLanguage}`, '', `name = "${ir.name}"`, `source = "${ir.sourceLanguage}"`, ''];
    for (const unit of ir.units) {
        lines.push(`[${unit.name}]`);
        lines.push(`kind = "${unit.kind}"`);
        if (unit.attributes.length > 0) {
            const fields = unit.attributes[0].replace('fields: ', '').split(', ');
            lines.push(`fields = [${fields.map(f => `"${f}"`).join(', ')}]`);
        }
        lines.push('');
    }
    return lines.join('\n');
}

function generateXML(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = ['<?xml version="1.0" encoding="UTF-8"?>', `<!-- ${ir.name} — Converted to XML -->`, `<module name="${ir.name}" source="${ir.sourceLanguage}">`];
    for (const unit of ir.units) {
        lines.push(`  <${unit.kind} name="${unit.name}">`);
        if (unit.attributes.length > 0) {
            const fields = unit.attributes[0].replace('fields: ', '').split(', ');
            for (const f of fields) { lines.push(`    <field name="${f}" type="string"/>`); }
        }
        lines.push(`  </${unit.kind}>`);
    }
    lines.push('</module>');
    return lines.join('\n');
}

function generateProtobuf(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`// ${ir.name} — Converted to Protobuf`, `// Source: ${ir.sourceLanguage}`, '', 'syntax = "proto3";', `package ${ir.name.toLowerCase()};`, ''];
    for (const unit of ir.units) {
        if (unit.kind === 'enum') {
            lines.push(`enum ${unit.name} {`);
            lines.push(`  ${unit.name.toUpperCase()}_UNSPECIFIED = 0;`);
            lines.push('}');
        } else if (unit.kind === 'function') {
            lines.push(`service ${unit.name}Service {`);
            lines.push(`  rpc ${unit.name}(${unit.name}Request) returns (${unit.name}Response);`);
            lines.push('}');
        } else {
            const fields = unit.attributes[0]?.replace('fields: ', '').split(', ') ?? ['id'];
            lines.push(`message ${unit.name} {`);
            fields.forEach((f, i) => lines.push(`  string ${sanitizeName(f)} = ${i + 1};`));
            lines.push('}');
        }
        lines.push('');
    }
    return lines.join('\n');
}

function generateHCL(ir: ULIRModule, opts: ConversionOptions): string {
    const lines = [`# ${ir.name} — Converted to HCL/Terraform`, `# Source: ${ir.sourceLanguage}`, ''];
    for (const unit of ir.units) {
        lines.push(`resource "omnisystem_${unit.kind}" "${unit.name}" {`);
        lines.push(`  name = "${unit.name}"`);
        if (unit.attributes.length > 0) {
            const fields = unit.attributes[0].replace('fields: ', '').split(', ');
            for (const f of fields.slice(0, 5)) { lines.push(`  # ${sanitizeName(f)} = ""`); }
        }
        lines.push('}');
        lines.push('');
    }
    return lines.join('\n');
}
