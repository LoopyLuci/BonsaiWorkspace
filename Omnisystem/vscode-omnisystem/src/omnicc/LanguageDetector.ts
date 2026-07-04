// OmniCC Language Detector — multi-heuristic auto-detection engine
// Scores every candidate language and returns the best match with confidence.

import { LanguageDef, LanguageFamily } from './ULIR';
import { allLanguages, getLangByExtension, getLang } from './LanguageRegistry';

export interface DetectionResult {
    langId: string;
    name: string;
    family: LanguageFamily;
    confidence: number;  // 0–100
    signals: string[];   // why we chose this
}

interface ScoredLang {
    lang: LanguageDef;
    score: number;
    signals: string[];
}

// ─── Primary detection ────────────────────────────────────────────────────────

export function detectLanguage(source: string, filenameHint?: string, langHint?: string): DetectionResult {
    if (langHint && langHint !== 'auto') {
        const lang = getLang(langHint);
        if (lang) {
            return { langId: lang.id, name: lang.name, family: lang.family, confidence: 100, signals: ['explicit hint'] };
        }
    }

    const scored: ScoredLang[] = [];

    // 1. Extension match (very strong signal)
    if (filenameHint) {
        const ext = extractExtension(filenameHint);
        const byExt = getLangByExtension(ext);
        if (byExt) {
            scored.push({ lang: byExt, score: 55, signals: [`file extension ${ext}`] });
        }
        // Special filenames
        const base = filenameHint.split('/').pop()?.toLowerCase() ?? '';
        if (base === 'makefile' || base === 'gnumakefile') {
            const make = getLang('makefile');
            if (make) { scored.push({ lang: make, score: 90, signals: ['filename is Makefile'] }); }
        }
        if (base === 'dockerfile') {
            scored.push({ lang: { id: 'docker', name: 'Dockerfile', aliases: [], extensions: ['Dockerfile'], family: 'shell', paradigms: ['declarative'], typing: 'none', memory: 'none', year: 2013, popularity: 8, color: '#0db7ed', description: 'Docker container image definition', keywords: ['FROM','RUN','CMD','COPY','EXPOSE','ENV'], comment: { line: '#' }, indentStyle: 'none', features: [] }, score: 90, signals: ['filename is Dockerfile'] });
        }
    }

    // 2. Shebang line (very strong)
    const shebangMatch = detectShebang(source);
    if (shebangMatch) { scored.push(shebangMatch); }

    // 3. Keyword / pattern heuristics for all known languages
    for (const lang of allLanguages()) {
        const s = scoreByKeywords(source, lang);
        if (s.score > 0) { scored.push(s); }
    }

    // 4. Structural patterns (indentation, brace style, etc.)
    const structural = scoreStructural(source);
    for (const s of structural) { scored.push(s); }

    // 5. Combine scores per language
    const combined = new Map<string, ScoredLang>();
    for (const s of scored) {
        const existing = combined.get(s.lang.id);
        if (existing) {
            existing.score += s.score;
            existing.signals.push(...s.signals);
        } else {
            combined.set(s.lang.id, { ...s, signals: [...s.signals] });
        }
    }

    // 6. Sort and return best
    const results = [...combined.values()].sort((a, b) => b.score - a.score);
    if (results.length === 0) {
        return { langId: 'unknown', name: 'Unknown', family: 'unknown', confidence: 0, signals: ['no patterns matched'] };
    }

    const best = results[0];
    const confidence = Math.min(100, Math.round((best.score / 120) * 100));
    return {
        langId: best.lang.id,
        name: best.lang.name,
        family: best.lang.family,
        confidence,
        signals: [...new Set(best.signals)].slice(0, 5),
    };
}

// ─── Shebang detection ────────────────────────────────────────────────────────

const SHEBANG_MAP: Array<[RegExp, string]> = [
    [/python3?/, 'python'], [/ruby/, 'ruby'], [/node/, 'javascript'],
    [/bash/, 'bash'], [/sh\b/, 'bash'], [/zsh/, 'zsh'], [/fish/, 'fish'],
    [/perl/, 'perl'], [/lua/, 'lua'], [/php/, 'php'],
    [/Rscript/, 'r'], [/julia/, 'julia'], [/deno/, 'typescript'],
    [/bun/, 'javascript'], [/elixir/, 'elixir'], [/tclsh/, 'tcl'],
];

function detectShebang(src: string): ScoredLang | null {
    const first = src.slice(0, 80);
    if (!first.startsWith('#!')) { return null; }
    for (const [pattern, langId] of SHEBANG_MAP) {
        if (pattern.test(first)) {
            const lang = getLang(langId);
            if (lang) { return { lang, score: 70, signals: [`shebang: ${first.slice(2, 40).trim()}`] }; }
        }
    }
    return null;
}

// ─── Keyword scoring ──────────────────────────────────────────────────────────

function scoreByKeywords(src: string, lang: LanguageDef): ScoredLang {
    let score = 0;
    const signals: string[] = [];
    const sample = src.slice(0, 8000); // score on first 8KB for speed

    for (const kw of lang.keywords) {
        if (sample.includes(kw)) {
            score += 8;
            if (signals.length < 3) { signals.push(`keyword: ${kw}`); }
        }
    }
    return { lang, score, signals };
}

// ─── Language-specific pattern scoring ───────────────────────────────────────

function scoreStructural(src: string): ScoredLang[] {
    const results: ScoredLang[] = [];
    const sample = src.slice(0, 4000);

    // TypeScript-specific patterns (must come before JS)
    if (/:\s*(string|number|boolean|void|never|unknown|any)\b/.test(sample) ||
        /\binterface\s+\w+/.test(sample) ||
        /\btype\s+\w+\s*=/.test(sample) ||
        /<\w+>/.test(sample)) {
        const ts = getLang('typescript');
        if (ts) { results.push({ lang: ts, score: 20, signals: ['TypeScript type annotations'] }); }
    }

    // JSX/React
    if (/<[A-Z]\w+[\s/>]/.test(sample) && /import.*React|useState|useEffect/.test(sample)) {
        const jsx = getLang('jsx');
        if (jsx) { results.push({ lang: jsx, score: 30, signals: ['JSX component syntax'] }); }
    }

    // Vue SFC
    if (/<template>/.test(sample) && /<script/.test(sample) && /<style/.test(sample)) {
        const vue = getLang('vue');
        if (vue) { results.push({ lang: vue, score: 60, signals: ['Vue single-file component'] }); }
    }

    // Svelte
    if (/<script>/.test(sample) && /\$:/.test(sample)) {
        const svelte = getLang('svelte');
        if (svelte) { results.push({ lang: svelte, score: 40, signals: ['Svelte reactive syntax'] }); }
    }

    // Rust: lifetime annotations
    if (/'[a-z]\b/.test(sample) && /fn\s+\w+/.test(sample) && /\bimpl\b/.test(sample)) {
        const rust = getLang('rust');
        if (rust) { results.push({ lang: rust, score: 25, signals: ['Rust lifetime annotations'] }); }
    }

    // Haskell: type signatures
    if (/\w+\s*::\s*/.test(sample) && /where/.test(sample) && /\bdo\b/.test(sample)) {
        const hs = getLang('haskell');
        if (hs) { results.push({ lang: hs, score: 25, signals: ['Haskell type signatures'] }); }
    }

    // OCaml/F#: let...in pattern
    if (/\blet\s+\w+\s*=/.test(sample) && /\bin\b/.test(sample) && /\bmatch\b/.test(sample)) {
        const ocaml = getLang('ocaml');
        if (ocaml) { results.push({ lang: ocaml, score: 15, signals: ['ML let-in pattern'] }); }
    }

    // Lisp family: many parentheses
    if ((sample.match(/\(/g) ?? []).length > 20 && /\(def/.test(sample)) {
        const cl = getLang('commonlisp');
        if (cl) { results.push({ lang: cl, score: 20, signals: ['Lisp S-expression density'] }); }
    }

    // Clojure
    if (/\(ns\s+\w/.test(sample) || /\(defn\s+/.test(sample)) {
        const clj = getLang('clojure');
        if (clj) { results.push({ lang: clj, score: 35, signals: ['Clojure ns/defn'] }); }
    }

    // SQL
    if (/\bSELECT\b.+\bFROM\b/i.test(sample) || /\bCREATE\s+TABLE\b/i.test(sample)) {
        const sql = getLang('sql');
        if (sql) { results.push({ lang: sql, score: 40, signals: ['SQL SELECT/FROM'] }); }
    }

    // YAML: indented key-value
    if (/^[a-z_]+:\s/m.test(sample) && /---/.test(sample)) {
        const yaml = getLang('yaml');
        if (yaml) { results.push({ lang: yaml, score: 20, signals: ['YAML structure'] }); }
    }

    // JSON
    if (/^\s*\{/.test(sample.trim()) && /"\w+"\s*:/.test(sample)) {
        const json = getLang('json');
        if (json) { results.push({ lang: json, score: 25, signals: ['JSON object structure'] }); }
    }

    // Prolog
    if (/:-/.test(sample) && /\.\s*$/.test(sample)) {
        const prolog = getLang('prolog');
        if (prolog) { results.push({ lang: prolog, score: 20, signals: ['Prolog :- rules'] }); }
    }

    // Assembly
    if (/\b(mov|push|pop|call|ret|jmp|lea)\b/i.test(sample) && /\bsection\b/i.test(sample)) {
        const asm = getLang('asm-x86');
        if (asm) { results.push({ lang: asm, score: 30, signals: ['x86 mnemonics'] }); }
    }

    // COBOL
    if (/IDENTIFICATION DIVISION/.test(sample) || /PROCEDURE DIVISION/.test(sample)) {
        const cobol = getLang('cobol');
        if (cobol) { results.push({ lang: cobol, score: 60, signals: ['COBOL DIVISION'] }); }
    }

    // Terraform/HCL
    if (/\bresource\s+"[\w_]+"/.test(sample) || /\bterraform\s*\{/.test(sample)) {
        const hcl = getLang('hcl');
        if (hcl) { results.push({ lang: hcl, score: 40, signals: ['Terraform resource block'] }); }
    }

    // GraphQL
    if (/\btype\s+\w+\s*\{/.test(sample) && /\bQuery\b/.test(sample)) {
        const gql = getLang('graphql');
        if (gql) { results.push({ lang: gql, score: 30, signals: ['GraphQL type definition'] }); }
    }

    // Omni languages
    if (/\bcomponent\s+\w+\s*\{/.test(sample)) {
        const vera = getLang('vera');
        if (vera) { results.push({ lang: vera, score: 45, signals: ['Vera component syntax'] }); }
    }
    if (/\blayout\s+\w+\s*\{/.test(sample)) {
        const nexus = getLang('nexus');
        if (nexus) { results.push({ lang: nexus, score: 45, signals: ['Nexus layout syntax'] }); }
    }
    if (/\bactor\s+\w+\s*\{/.test(sample)) {
        const aether = getLang('aether');
        if (aether) { results.push({ lang: aether, score: 45, signals: ['Aether actor syntax'] }); }
    }
    if (/\btheorem\s+\w+\s*\{/.test(sample)) {
        const axiom = getLang('axiom');
        if (axiom) { results.push({ lang: axiom, score: 45, signals: ['Axiom theorem syntax'] }); }
    }
    if (/\bpipeline\s+\w+\s*\{/.test(sample) && /\bshader\b/.test(sample)) {
        const helix = getLang('helix');
        if (helix) { results.push({ lang: helix, score: 45, signals: ['Helix shader/pipeline syntax'] }); }
    }

    return results;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function extractExtension(filename: string): string {
    const parts = filename.split('.');
    if (parts.length > 1) { return '.' + parts[parts.length - 1].toLowerCase(); }
    return '';
}

// ─── Batch detection for project mode ────────────────────────────────────────

export function detectLanguageBatch(files: Array<{ path: string; content: string }>): Map<string, DetectionResult> {
    const results = new Map<string, DetectionResult>();
    for (const file of files) {
        results.set(file.path, detectLanguage(file.content, file.path));
    }
    return results;
}
