// BodyTranslator.ts — Translates raw function bodies between programming languages.
// Produces idiomatic target code from parsed source bodies instead of TODO stubs.
// Uses pattern-based (regex) transforms covering the most common programming constructs.

// ─── Language metadata ────────────────────────────────────────────────────────

type BlockStyle = 'brace' | 'indent' | 'functional';

function blockStyle(lang: string): BlockStyle {
    const INDENT = new Set(['python', 'coffeescript', 'nim', 'elixir', 'haml', 'pug', 'jade', 'yaml']);
    const FUNC   = new Set(['haskell', 'ocaml', 'fsharp', 'elm', 'purescript', 'idris', 'agda', 'coq', 'lean']);
    if (INDENT.has(lang)) { return 'indent'; }
    if (FUNC.has(lang)) { return 'functional'; }
    return 'brace';
}

function commentPfx(lang: string): string {
    const HASH = new Set(['python', 'ruby', 'r', 'julia', 'perl', 'bash', 'sh', 'zsh', 'fish', 'powershell',
                          'elixir', 'nim', 'crystal', 'coffeescript', 'makefile', 'yaml', 'toml']);
    const DASH = new Set(['sql', 'mysql', 'postgresql', 'lua', 'haskell', 'ada', 'vhdl']);
    const PCT  = new Set(['erlang', 'prolog', 'matlab', 'octave', 'tex', 'latex']);
    const SEMI = new Set(['commonlisp', 'scheme', 'racket', 'clojure', 'asm', 'assembly']);
    if (HASH.has(lang)) { return '#'; }
    if (DASH.has(lang)) { return '--'; }
    if (PCT.has(lang))  { return '%'; }
    if (SEMI.has(lang)) { return ';'; }
    return '//';
}

function usesSemicolon(lang: string): boolean {
    const NO_SEMI = new Set(['python', 'ruby', 'coffeescript', 'nim', 'elixir', 'go', 'kotlin',
                             'swift', 'lua', 'r', 'julia', 'haskell', 'ocaml', 'fsharp', 'elm', 'bash',
                             'sh', 'zsh', 'fish', 'makefile', 'yaml', 'toml', 'json']);
    return !NO_SEMI.has(lang);
}

// ─── Boolean / null literal maps ──────────────────────────────────────────────

interface LitMap { T: string; F: string; N: string; and: string; or: string; not: string; }

const LITS: Record<string, LitMap> = {
    python:     { T: 'True',  F: 'False', N: 'None',      and: 'and', or: 'or',  not: 'not ' },
    ruby:       { T: 'true',  F: 'false', N: 'nil',       and: '&&',  or: '||',  not: '!' },
    go:         { T: 'true',  F: 'false', N: 'nil',       and: '&&',  or: '||',  not: '!' },
    rust:       { T: 'true',  F: 'false', N: 'None',      and: '&&',  or: '||',  not: '!' },
    swift:      { T: 'true',  F: 'false', N: 'nil',       and: '&&',  or: '||',  not: '!' },
    kotlin:     { T: 'true',  F: 'false', N: 'null',      and: '&&',  or: '||',  not: '!' },
    scala:      { T: 'true',  F: 'false', N: 'None',      and: '&&',  or: '||',  not: '!' },
    lua:        { T: 'true',  F: 'false', N: 'nil',       and: 'and', or: 'or',  not: 'not ' },
    elixir:     { T: 'true',  F: 'false', N: 'nil',       and: '&&',  or: '||',  not: '!' },
    erlang:     { T: 'true',  F: 'false', N: 'undefined', and: 'andalso', or: 'orelse', not: 'not ' },
    haskell:    { T: 'True',  F: 'False', N: 'Nothing',   and: '&&',  or: '||',  not: 'not ' },
    ocaml:      { T: 'true',  F: 'false', N: 'None',      and: '&&',  or: '||',  not: 'not ' },
    julia:      { T: 'true',  F: 'false', N: 'nothing',   and: '&&',  or: '||',  not: '!' },
    r:          { T: 'TRUE',  F: 'FALSE', N: 'NULL',      and: '&&',  or: '||',  not: '!' },
    perl:       { T: '1',     F: '0',     N: 'undef',     and: 'and', or: 'or',  not: '!' },
    php:        { T: 'true',  F: 'false', N: 'null',      and: '&&',  or: '||',  not: '!' },
};

function lits(lang: string): LitMap {
    return LITS[lang] ?? { T: 'true', F: 'false', N: 'null', and: '&&', or: '||', not: '!' };
}

// ─── Extract body lines from originalSource ───────────────────────────────────

function extractBodyLines(src: string, srcLang: string): string[] {
    const raw = src.split('\n');
    if (raw.length === 0) { return []; }

    if (blockStyle(srcLang) === 'indent') {
        // Python / Ruby etc. — first line is the def/fn, rest is body
        if (raw.length <= 1) { return []; }
        const bodyLines = raw.slice(1);
        // Strip the common leading indent
        const minIndent = bodyLines
            .filter(l => l.trim().length > 0)
            .reduce((min, l) => {
                const m = l.match(/^(\s*)/);
                return Math.min(min, m ? m[1].length : 0);
            }, Infinity);
        if (minIndent === Infinity) { return bodyLines; }
        return bodyLines.map(l => l.slice(minIndent));
    }

    // Brace-style: extract content between the outer { }
    const openIdx = raw.findIndex(l => l.includes('{'));
    if (openIdx < 0) { return raw.slice(1); }

    // Find the matching closing brace at depth 0
    let depth = 0;
    let closeIdx = raw.length - 1;
    for (let i = 0; i < raw.length; i++) {
        for (const ch of raw[i]) {
            if (ch === '{') { depth++; }
            if (ch === '}') {
                depth--;
                if (depth === 0) { closeIdx = i; break; }
            }
        }
        if (depth === 0 && i > openIdx) { closeIdx = i; break; }
    }

    const inner = raw.slice(openIdx + 1, closeIdx);
    // Remove one level of common indent (4 spaces or 1 tab)
    return inner.map(l => l.replace(/^    |\t/, ''));
}

// ─── Variable declaration transforms ─────────────────────────────────────────

function transformVarDecl(line: string, tgtLang: string): string {
    // JS/TS: const/let/var x [: T] = expr;
    const jsConst = line.match(/^(\s*)(?:const|final)\s+(\w+)(?:\s*:\s*[\w<>?,\[\]|]+)?\s*=\s*(.+?);?\s*$/);
    if (jsConst) {
        const [, ind, name, val] = jsConst;
        switch (tgtLang) {
            case 'python':   return `${ind}${name} = ${val}`;
            case 'go':       return `${ind}${name} := ${val}`;
            case 'rust':     return `${ind}let ${name} = ${val};`;
            case 'kotlin':   return `${ind}val ${name} = ${val}`;
            case 'scala':    return `${ind}val ${name} = ${val}`;
            case 'swift':    return `${ind}let ${name} = ${val}`;
            case 'ruby':     return `${ind}${name} = ${val}`;
            case 'lua':      return `${ind}local ${name} = ${val}`;
            case 'java':     return `${ind}final var ${name} = ${val};`;
            case 'csharp':   return `${ind}var ${name} = ${val};`;
            case 'php':      return `${ind}$${name} = ${val};`;
            default:         return line;
        }
    }
    // JS/TS: let/var x [: T] = expr;
    const jsLet = line.match(/^(\s*)(?:let|var)\s+(\w+)(?:\s*:\s*[\w<>?,\[\]|]+)?\s*=\s*(.+?);?\s*$/);
    if (jsLet) {
        const [, ind, name, val] = jsLet;
        switch (tgtLang) {
            case 'python':   return `${ind}${name} = ${val}`;
            case 'go':       return `${ind}${name} := ${val}`;
            case 'rust':     return `${ind}let mut ${name} = ${val};`;
            case 'kotlin':   return `${ind}var ${name} = ${val}`;
            case 'scala':    return `${ind}var ${name} = ${val}`;
            case 'swift':    return `${ind}var ${name} = ${val}`;
            case 'ruby':     return `${ind}${name} = ${val}`;
            case 'lua':      return `${ind}local ${name} = ${val}`;
            case 'java':     return `${ind}var ${name} = ${val};`;
            case 'csharp':   return `${ind}var ${name} = ${val};`;
            case 'php':      return `${ind}$${name} = ${val};`;
            default:         return line;
        }
    }
    // Rust: let [mut] x = expr;
    const rustLet = line.match(/^(\s*)let\s+(mut\s+)?(\w+)\s*=\s*(.+?);?\s*$/);
    if (rustLet) {
        const [, ind,, name, val] = rustLet;
        switch (tgtLang) {
            case 'python':   return `${ind}${name} = ${val}`;
            case 'javascript':
            case 'typescript': return `${ind}let ${name} = ${val};`;
            case 'go':       return `${ind}${name} := ${val}`;
            case 'kotlin':   return `${ind}var ${name} = ${val}`;
            case 'java':     return `${ind}var ${name} = ${val};`;
            case 'swift':    return `${ind}var ${name} = ${val}`;
            case 'ruby':     return `${ind}${name} = ${val}`;
            case 'lua':      return `${ind}local ${name} = ${val}`;
            default:         return line;
        }
    }
    // Go: x := expr
    const goShortDecl = line.match(/^(\s*)(\w+)\s*:=\s*(.+?)\s*$/);
    if (goShortDecl) {
        const [, ind, name, val] = goShortDecl;
        switch (tgtLang) {
            case 'python':   return `${ind}${name} = ${val}`;
            case 'javascript': return `${ind}let ${name} = ${val};`;
            case 'typescript': return `${ind}let ${name} = ${val};`;
            case 'rust':     return `${ind}let mut ${name} = ${val};`;
            case 'kotlin':   return `${ind}var ${name} = ${val}`;
            case 'java':     return `${ind}var ${name} = ${val};`;
            case 'ruby':     return `${ind}${name} = ${val}`;
            case 'swift':    return `${ind}var ${name} = ${val}`;
            case 'lua':      return `${ind}local ${name} = ${val}`;
            default:         return line;
        }
    }
    // Kotlin: val/var x = expr
    const ktDecl = line.match(/^(\s*)(val|var)\s+(\w+)\s*=\s*(.+?)\s*$/);
    if (ktDecl) {
        const [, ind, kw, name, val] = ktDecl;
        switch (tgtLang) {
            case 'python':   return `${ind}${name} = ${val}`;
            case 'javascript': return `${ind}${kw === 'val' ? 'const' : 'let'} ${name} = ${val};`;
            case 'typescript': return `${ind}${kw === 'val' ? 'const' : 'let'} ${name} = ${val};`;
            case 'go':       return `${ind}${name} := ${val}`;
            case 'rust':     return `${ind}let ${kw === 'val' ? '' : 'mut '}${name} = ${val};`;
            case 'java':     return `${ind}var ${name} = ${val};`;
            case 'csharp':   return `${ind}var ${name} = ${val};`;
            case 'ruby':     return `${ind}${name} = ${val}`;
            case 'swift':    return `${ind}${kw} ${name} = ${val}`;
            case 'lua':      return `${ind}local ${name} = ${val}`;
            default:         return line;
        }
    }
    return line;
}

// ─── I/O transforms ───────────────────────────────────────────────────────────

function transformIO(line: string, srcLang: string, tgtLang: string): string {
    const ind = (line.match(/^(\s*)/) ?? ['', ''])[1];
    const body = line.trimStart();

    // console.log / console.error / console.warn → target
    if (['javascript', 'typescript', 'jsx', 'tsx'].includes(srcLang)) {
        const m = body.match(/^console\.(log|error|warn|info|debug)\((.+)\);?$/);
        if (m) {
            const args = m[2];
            switch (tgtLang) {
                case 'python':  return `${ind}print(${args})`;
                case 'go':      return `${ind}fmt.Println(${args})`;
                case 'rust':    return `${ind}println!("{:?}", ${args});`;
                case 'java':    return `${ind}System.out.println(${args});`;
                case 'kotlin':  return `${ind}println(${args})`;
                case 'swift':   return `${ind}print(${args})`;
                case 'dart':    return `${ind}print(${args});`;
                case 'ruby':    return `${ind}puts ${args}`;
                case 'lua':     return `${ind}print(${args})`;
                case 'php':     return `${ind}echo ${args};`;
                case 'csharp':  return `${ind}Console.WriteLine(${args});`;
                case 'scala':   return `${ind}println(${args})`;
            }
        }
    }

    // print(x) [Python] → target
    if (srcLang === 'python') {
        const m = body.match(/^print\((.+)\)$/);
        if (m) {
            const args = m[1];
            switch (tgtLang) {
                case 'javascript':
                case 'typescript': return `${ind}console.log(${args});`;
                case 'go':         return `${ind}fmt.Println(${args})`;
                case 'rust':       return `${ind}println!("{:?}", ${args});`;
                case 'java':       return `${ind}System.out.println(${args});`;
                case 'kotlin':     return `${ind}println(${args})`;
                case 'swift':      return `${ind}print(${args})`;
                case 'dart':       return `${ind}print(${args});`;
                case 'ruby':       return `${ind}puts ${args}`;
                case 'lua':        return `${ind}print(${args})`;
                case 'php':        return `${ind}echo ${args};`;
                case 'csharp':     return `${ind}Console.WriteLine(${args});`;
                case 'scala':      return `${ind}println(${args})`;
            }
        }
    }

    // System.out.println [Java/Kotlin] → target
    if (['java', 'kotlin', 'scala'].includes(srcLang)) {
        const m = body.match(/^System\.out\.println\((.+)\);?$/) ?? body.match(/^println\((.+)\)$/);
        if (m) {
            const args = m[1];
            switch (tgtLang) {
                case 'javascript':
                case 'typescript': return `${ind}console.log(${args});`;
                case 'python':     return `${ind}print(${args})`;
                case 'go':         return `${ind}fmt.Println(${args})`;
                case 'rust':       return `${ind}println!("{:?}", ${args});`;
                case 'swift':      return `${ind}print(${args})`;
                case 'ruby':       return `${ind}puts ${args}`;
                case 'lua':        return `${ind}print(${args})`;
                case 'csharp':     return `${ind}Console.WriteLine(${args});`;
            }
        }
    }

    // fmt.Println [Go] → target
    if (srcLang === 'go') {
        const m = body.match(/^fmt\.(Println|Printf|Print)\((.+)\);?$/);
        if (m) {
            const args = m[2];
            switch (tgtLang) {
                case 'javascript':
                case 'typescript': return `${ind}console.log(${args});`;
                case 'python':     return `${ind}print(${args})`;
                case 'rust':       return `${ind}println!("{:?}", ${args});`;
                case 'java':       return `${ind}System.out.println(${args});`;
                case 'kotlin':     return `${ind}println(${args})`;
                case 'swift':      return `${ind}print(${args})`;
                case 'ruby':       return `${ind}puts ${args}`;
                case 'csharp':     return `${ind}Console.WriteLine(${args});`;
            }
        }
    }

    // println! [Rust] → target
    if (srcLang === 'rust') {
        const m = body.match(/^println!\((.+)\);?$/);
        if (m) {
            const args = m[1];
            switch (tgtLang) {
                case 'javascript':
                case 'typescript': return `${ind}console.log(${args});`;
                case 'python':     return `${ind}print(${args})`;
                case 'go':         return `${ind}fmt.Println(${args})`;
                case 'java':       return `${ind}System.out.println(${args});`;
                case 'kotlin':     return `${ind}println(${args})`;
                case 'swift':      return `${ind}print(${args})`;
                case 'ruby':       return `${ind}puts ${args}`;
                case 'csharp':     return `${ind}Console.WriteLine(${args});`;
            }
        }
    }

    // puts [Ruby] → target
    if (srcLang === 'ruby') {
        const m = body.match(/^puts\s+(.+)$/);
        if (m) {
            const args = m[1];
            switch (tgtLang) {
                case 'javascript':
                case 'typescript': return `${ind}console.log(${args});`;
                case 'python':     return `${ind}print(${args})`;
                case 'go':         return `${ind}fmt.Println(${args})`;
                case 'java':       return `${ind}System.out.println(${args});`;
                case 'kotlin':     return `${ind}println(${args})`;
                case 'rust':       return `${ind}println!("{:?}", ${args});`;
                case 'csharp':     return `${ind}Console.WriteLine(${args});`;
            }
        }
    }

    // Console.WriteLine [C#] → target
    if (srcLang === 'csharp') {
        const m = body.match(/^Console\.WriteLine\((.+)\);?$/);
        if (m) {
            const args = m[1];
            switch (tgtLang) {
                case 'javascript':
                case 'typescript': return `${ind}console.log(${args});`;
                case 'python':     return `${ind}print(${args})`;
                case 'go':         return `${ind}fmt.Println(${args})`;
                case 'java':       return `${ind}System.out.println(${args});`;
                case 'kotlin':     return `${ind}println(${args})`;
                case 'rust':       return `${ind}println!("{:?}", ${args});`;
                case 'ruby':       return `${ind}puts ${args}`;
            }
        }
    }

    return line;
}

// ─── Literal substitution ─────────────────────────────────────────────────────

function applyLiterals(line: string, srcLang: string, tgtLang: string): string {
    const src = lits(srcLang);
    const tgt = lits(tgtLang);

    // Only transform if source and target literals differ
    if (src.T !== tgt.T) {
        // Use word-boundary replacement to avoid partial matches
        line = line.replace(/\btrue\b/g, tgt.T).replace(/\bTrue\b/g, tgt.T);
    }
    if (src.F !== tgt.F) {
        line = line.replace(/\bfalse\b/g, tgt.F).replace(/\bFalse\b/g, tgt.F);
    }
    // Null/nil/None — replace all variants with target
    if (tgt.N !== 'null') {
        line = line.replace(/\bnull\b/g, tgt.N);
    }
    if (tgt.N !== 'nil') {
        line = line.replace(/\bnil\b/g, tgt.N);
    }
    if (tgt.N !== 'None') {
        line = line.replace(/\bNone\b/g, tgt.N);
    }
    if (tgt.N !== 'nothing') {
        line = line.replace(/\bnothing\b/g, tgt.N);
    }
    if (tgt.N !== 'undefined') {
        line = line.replace(/\bundefined\b/g, tgt.N);
    }

    return line;
}

// ─── Comparison / equality operators ─────────────────────────────────────────

function transformOperators(line: string, srcLang: string, tgtLang: string): string {
    // Strict equality in JS/TS → == in other languages
    if (['javascript', 'typescript'].includes(srcLang) &&
        !['javascript', 'typescript'].includes(tgtLang)) {
        line = line.replace(/===/g, '==').replace(/!==/g, '!=');
    }
    // Python not-equals uses !=; most others too — already compatible
    return line;
}

// ─── String operation transforms ─────────────────────────────────────────────

function transformStringOps(line: string, srcLang: string, tgtLang: string): string {
    // .length → len(x) for Python
    if (tgtLang === 'python') {
        // x.length → len(x)  — handles simple cases like `x.length`, `arr.length`
        line = line.replace(/\b(\w+)\.length\b/g, (_, v) => `len(${v})`);
        // .toLowerCase() → .lower()
        line = line.replace(/\.toLowerCase\(\)/g, '.lower()');
        // .toUpperCase() → .upper()
        line = line.replace(/\.toUpperCase\(\)/g, '.upper()');
        // .includes(x) → x in variable (we can't easily restructure this, leave as comment)
        line = line.replace(/\.includes\((.+?)\)/g, (_, arg) => `.__contains__(${arg})`);
        // .startsWith(x) → .startswith(x)
        line = line.replace(/\.startsWith\(/g, '.startswith(');
        // .endsWith(x) → .endswith(x)
        line = line.replace(/\.endsWith\(/g, '.endswith(');
        // .indexOf(x) → .find(x) (or .index(x) — find doesn't throw)
        line = line.replace(/\.indexOf\(/g, '.find(');
        // .trim() → .strip()
        line = line.replace(/\.trim\(\)/g, '.strip()');
        // .split(x) → .split(x) — compatible
        // .join(sep) → sep.join(arr) — too complex to restructure, leave as-is
        // .push(x) → .append(x)
        line = line.replace(/\.push\(/g, '.append(');
        // .pop() → .pop()  — compatible
        // Template literals `${x}` → f"..."
        line = line.replace(/`([^`]*)`/g, (_, inner) => {
            const converted = inner.replace(/\$\{([^}]+)\}/g, '{$1}');
            return `f"${converted}"`;
        });
    }

    if (tgtLang === 'rust') {
        line = line.replace(/\b(\w+)\.length\b/g, (_, v) => `${v}.len()`);
        line = line.replace(/\.toLowerCase\(\)/g, '.to_lowercase()');
        line = line.replace(/\.toUpperCase\(\)/g, '.to_uppercase()');
        line = line.replace(/\.startsWith\(/g, '.starts_with(');
        line = line.replace(/\.endsWith\(/g, '.ends_with(');
        line = line.replace(/\.includes\(/g, '.contains(');
        line = line.replace(/\.indexOf\(/g, '.find(');
        line = line.replace(/\.trim\(\)/g, '.trim()');
        line = line.replace(/\.push\(/g, '.push(');
        // Template literals → format!
        line = line.replace(/`([^`]*)`/g, (_, inner) => {
            const fmtStr = inner.replace(/\$\{([^}]+)\}/g, '{}');
            const args = [];
            const re = /\$\{([^}]+)\}/g;
            let m: RegExpExecArray | null;
            const orig = inner;
            while ((m = re.exec(orig)) !== null) { args.push(m[1]); }
            return `format!("${fmtStr}"${args.length ? ', ' + args.join(', ') : ''})`;
        });
    }

    if (tgtLang === 'go') {
        line = line.replace(/\b(\w+)\.length\b/g, (_, v) => `len(${v})`);
        line = line.replace(/\.toLowerCase\(\)/g, ''); // strings.ToLower() needs restructuring
        line = line.replace(/\.toUpperCase\(\)/g, '');
        line = line.replace(/\.startsWith\(/g, '; strings.HasPrefix(');
        line = line.replace(/\.endsWith\(/g, '; strings.HasSuffix(');
        line = line.replace(/\.includes\(/g, '; strings.Contains(');
        line = line.replace(/\.trim\(\)/g, '; strings.TrimSpace(');
        // Template literals → fmt.Sprintf
        line = line.replace(/`([^`]*)`/g, (_, inner) => {
            const fmtStr = inner.replace(/\$\{([^}]+)\}/g, '%v');
            const args = [];
            const re = /\$\{([^}]+)\}/g;
            let m: RegExpExecArray | null;
            while ((m = re.exec(inner)) !== null) { args.push(m[1]); }
            return `fmt.Sprintf("${fmtStr}"${args.length ? ', ' + args.join(', ') : ''})`;
        });
    }

    if (['java', 'csharp', 'kotlin', 'scala'].includes(tgtLang)) {
        // Template literals → String.format or interpolation
        if (tgtLang === 'kotlin' || tgtLang === 'scala') {
            // Kotlin supports string interpolation with ${}
            line = line.replace(/`([^`]*)`/g, (_, inner) => {
                const converted = inner.replace(/\$\{([^}]+)\}/g, '${$1}');
                return `"${converted}"`;
            });
        } else if (tgtLang === 'java') {
            line = line.replace(/`([^`]*)`/g, (_, inner) => {
                const fmtStr = inner.replace(/\$\{([^}]+)\}/g, '%s');
                const args = [];
                const re = /\$\{([^}]+)\}/g;
                let m: RegExpExecArray | null;
                while ((m = re.exec(inner)) !== null) { args.push(m[1]); }
                return `String.format("${fmtStr}"${args.length ? ', ' + args.join(', ') : ''})`;
            });
        } else if (tgtLang === 'csharp') {
            line = line.replace(/`([^`]*)`/g, (_, inner) => {
                const converted = inner.replace(/\$\{([^}]+)\}/g, '{$1}');
                return `$"${converted}"`;
            });
        }
    }

    return line;
}

// ─── Comment style conversion ─────────────────────────────────────────────────

function transformComment(line: string, srcLang: string, tgtLang: string): string {
    const srcPfx = commentPfx(srcLang);
    const tgtPfx = commentPfx(tgtLang);
    if (srcPfx === tgtPfx) { return line; }

    const ind = (line.match(/^(\s*)/) ?? ['', ''])[1];
    const body = line.trimStart();

    // Single-line comment
    if (body.startsWith(srcPfx)) {
        const content = body.slice(srcPfx.length);
        return `${ind}${tgtPfx}${content}`;
    }
    // C-style /* ... */ → target comment style
    if (body.startsWith('/*') && body.endsWith('*/')) {
        const content = body.slice(2, -2).trim();
        return `${ind}${tgtPfx} ${content}`;
    }

    return line;
}

// ─── Return statement / semicolon handling ────────────────────────────────────

function transformReturn(line: string, tgtLang: string): string {
    const m = line.match(/^(\s*)return\s+(.*?);?\s*$/);
    if (!m) { return line; }
    const [, ind, val] = m;
    if (usesSemicolon(tgtLang)) {
        return `${ind}return ${val};`;
    }
    return `${ind}return ${val}`;
}

function ensureSemicolon(line: string, tgtLang: string): string {
    if (!usesSemicolon(tgtLang)) {
        return line.replace(/;\s*$/, '');
    }
    const trimmed = line.trimEnd();
    if (trimmed.length === 0) { return line; }
    const last = trimmed[trimmed.length - 1];
    // Don't add ; if line already ends with ; { } ( ) or is a control flow keyword line
    if ([';', '{', '}', '(', ')', ',', ':'].includes(last)) { return line; }
    // Don't add ; to comment lines
    const body = line.trimStart();
    if (body.startsWith('//') || body.startsWith('#') || body.startsWith('--') || body.startsWith('*')) {
        return line;
    }
    return trimmed + ';';
}

// ─── Control flow conversion ──────────────────────────────────────────────────

function transformControlFlow(line: string, srcLang: string, tgtLang: string): string {
    const srcStyle = blockStyle(srcLang);
    const tgtStyle = blockStyle(tgtLang);
    const ind = (line.match(/^(\s*)/) ?? ['', ''])[1];
    const body = line.trimStart();

    // Brace-style → Python (indent style)
    if (srcStyle === 'brace' && tgtStyle === 'indent') {
        // if (cond) { → if cond:
        let m = body.match(/^if\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}if ${m[1]}:`; }
        // } else if (cond) { → elif cond:
        m = body.match(/^\}\s*else\s+if\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}elif ${m[1]}:`; }
        // } else { → else:
        m = body.match(/^\}\s*else\s*\{?\s*$/);
        if (m) { return `${ind}else:`; }
        // for (const x of arr) { → for x in arr:
        m = body.match(/^for\s*\(\s*(?:const|let|var)\s+(\w+)\s+of\s+(\w+)\s*\)\s*\{?\s*$/);
        if (m) { return `${ind}for ${m[1]} in ${m[2]}:`; }
        // for (const x in obj) { → for x in obj:
        m = body.match(/^for\s*\(\s*(?:const|let|var)\s+(\w+)\s+in\s+(\w+)\s*\)\s*\{?\s*$/);
        if (m) { return `${ind}for ${m[1]} in ${m[2]}:`; }
        // for (let i = 0; i < n; i++) { → for i in range(n):
        m = body.match(/^for\s*\(\s*(?:let|var|int)\s+(\w+)\s*=\s*(\d+)\s*;\s*\1\s*<\s*(\w+|\d+)\s*;\s*\1\+\+\s*\)\s*\{?\s*$/);
        if (m) { return m[2] === '0' ? `${ind}for ${m[1]} in range(${m[3]}):` : `${ind}for ${m[1]} in range(${m[2]}, ${m[3]}):`; }
        // while (cond) { → while cond:
        m = body.match(/^while\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}while ${m[1]}:`; }
        // switch (x) { → (leave as comment — Python has no switch)
        m = body.match(/^switch\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}# match ${m[1]}:`; }
        // case x: → case x:
        m = body.match(/^case\s+(.+):\s*$/);
        if (m) { return `${ind}case ${m[1]}:`; }
        // try { → try:
        m = body.match(/^try\s*\{?\s*$/);
        if (m) { return `${ind}try:`; }
        // } catch (e) { → except Exception as e:
        m = body.match(/^\}\s*catch\s*\((\w+)\s*(?::\s*\w+)?\)\s*\{?\s*$/);
        if (m) { return `${ind}except Exception as ${m[1]}:`; }
        // } finally { → finally:
        m = body.match(/^\}\s*finally\s*\{?\s*$/);
        if (m) { return `${ind}finally:`; }
        // Standalone } → skip (handled in convertBracesToIndent)
        if (body === '}' || body === '};') { return ''; }
    }

    // Python (indent) → brace-style
    if (srcStyle === 'indent' && tgtStyle === 'brace') {
        // if x: → if (x) {
        let m = body.match(/^if\s+(.+):$/);
        if (m) { return `${ind}if (${m[1]}) {`; }
        // elif x: → } else if (x) {
        m = body.match(/^elif\s+(.+):$/);
        if (m) { return `${ind}} else if (${m[1]}) {`; }
        // else: → } else {
        m = body.match(/^else:$/);
        if (m) { return `${ind}} else {`; }
        // for x in range(n): → for (let x = 0; x < n; x++) {
        m = body.match(/^for\s+(\w+)\s+in\s+range\((\d+)\):$/);
        if (m) { return `${ind}for (let ${m[1]} = 0; ${m[1]} < ${m[2]}; ${m[1]}++) {`; }
        // for x in range(a, b): → for (let x = a; x < b; x++) {
        m = body.match(/^for\s+(\w+)\s+in\s+range\((\w+|\d+),\s*(\w+|\d+)\):$/);
        if (m) { return `${ind}for (let ${m[1]} = ${m[2]}; ${m[1]} < ${m[3]}; ${m[1]}++) {`; }
        // for x in arr: → for (const x of arr) {
        m = body.match(/^for\s+(\w+)\s+in\s+(\w+):$/);
        if (m) { return `${ind}for (const ${m[1]} of ${m[2]}) {`; }
        // while x: → while (x) {
        m = body.match(/^while\s+(.+):$/);
        if (m) { return `${ind}while (${m[1]}) {`; }
        // try: → try {
        m = body.match(/^try:$/);
        if (m) { return `${ind}try {`; }
        // except Exception as e: → } catch (e) {
        m = body.match(/^except\s+(?:\w+\s+as\s+)?(\w+):$/);
        if (m) { return `${ind}} catch (${m[1]}) {`; }
        m = body.match(/^except:$/);
        if (m) { return `${ind}} catch (e) {`; }
        // finally: → } finally {
        m = body.match(/^finally:$/);
        if (m) { return `${ind}} finally {`; }
    }

    // Brace → Rust
    if (srcStyle === 'brace' && tgtLang === 'rust') {
        let m = body.match(/^if\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}if ${m[1]} {`; }
        m = body.match(/^\}\s*else\s+if\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}} else if ${m[1]} {`; }
        m = body.match(/^for\s*\(\s*(?:const|let|var)\s+(\w+)\s+of\s+(\w+)\s*\)\s*\{?\s*$/);
        if (m) { return `${ind}for ${m[1]} in ${m[2]}.iter() {`; }
        m = body.match(/^for\s*\(\s*(?:let|var|int)\s+(\w+)\s*=\s*(\d+)\s*;\s*\1\s*<\s*(\w+|\d+)\s*;\s*\1\+\+\s*\)\s*\{?\s*$/);
        if (m) { return m[2] === '0' ? `${ind}for ${m[1]} in 0..${m[3]} {` : `${ind}for ${m[1]} in ${m[2]}..${m[3]} {`; }
        m = body.match(/^while\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}while ${m[1]} {`; }
    }

    // Brace → Go
    if (srcStyle === 'brace' && tgtLang === 'go') {
        let m = body.match(/^if\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}if ${m[1]} {`; }
        m = body.match(/^\}\s*else\s+if\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}} else if ${m[1]} {`; }
        m = body.match(/^for\s*\(\s*(?:const|let|var)\s+(\w+)\s+of\s+(\w+)\s*\)\s*\{?\s*$/);
        if (m) { return `${ind}for _, ${m[1]} := range ${m[2]} {`; }
        m = body.match(/^for\s*\(\s*(?:let|var|int)\s+(\w+)\s*=\s*(\d+)\s*;\s*\1\s*<\s*(\w+|\d+)\s*;\s*\1\+\+\s*\)\s*\{?\s*$/);
        if (m) { return `${ind}for ${m[1]} := ${m[2]}; ${m[1]} < ${m[3]}; ${m[1]}++ {`; }
        m = body.match(/^while\s*\((.+)\)\s*\{?\s*$/);
        if (m) { return `${ind}for ${m[1]} {`; }
    }

    return line;
}

// ─── Block structure: brace → indent ─────────────────────────────────────────

function convertBracesToIndent(lines: string[]): string[] {
    const out: string[] = [];
    for (const line of lines) {
        const body = line.trimStart();
        // Skip standalone closing braces
        if (body === '}' || body === '};' || body === '})' || body === '});') {
            continue;
        }
        // Remove trailing { from lines (they became : in transformControlFlow)
        out.push(line.replace(/\s*\{\s*$/, ''));
    }
    return out.filter(l => l !== undefined);
}

// ─── Block structure: indent → brace ─────────────────────────────────────────

function convertIndentToBraces(lines: string[], tgtLang: string): string[] {
    const out: string[] = [];
    const indents: number[] = [0];

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.trim() === '') { out.push(''); continue; }

        const currentIndent = (line.match(/^(\s*)/) ?? ['', ''])[1].length;
        const lastIndent = indents[indents.length - 1] ?? 0;

        // Dedent: close blocks
        while (currentIndent < lastIndent && indents.length > 1) {
            indents.pop();
            const closeIndent = ' '.repeat(indents[indents.length - 1] ?? 0);
            out.push(`${closeIndent}}`);
        }

        // Indent: block opened on previous line (previous line ended with ':')
        if (currentIndent > lastIndent) {
            indents.push(currentIndent);
        }

        out.push(line);
    }

    // Close remaining blocks
    while (indents.length > 1) {
        indents.pop();
        const closeIndent = ' '.repeat(indents[indents.length - 1] ?? 0);
        out.push(`${closeIndent}}`);
    }

    return out;
}

// ─── Per-line transform ────────────────────────────────────────────────────────

function transformLine(line: string, srcLang: string, tgtLang: string): string {
    if (line.trim() === '') { return line; }

    // 1. Comment style
    const commentPfxSrc = commentPfx(srcLang);
    const lineBody = line.trimStart();
    const isComment = lineBody.startsWith(commentPfxSrc) ||
                      lineBody.startsWith('//') || lineBody.startsWith('/*') ||
                      lineBody.startsWith('#') || lineBody.startsWith('--');
    if (isComment) {
        return transformComment(line, srcLang, tgtLang);
    }

    // 2. Return statement
    if (/^\s*return\s/.test(line)) {
        line = transformReturn(line, tgtLang);
    }

    // 3. Control flow
    line = transformControlFlow(line, srcLang, tgtLang);
    if (line === '') { return ''; }  // empty = removed closing brace

    // 4. Variable declarations
    line = transformVarDecl(line, tgtLang);

    // 5. I/O transforms
    line = transformIO(line, srcLang, tgtLang);

    // 6. String operations
    line = transformStringOps(line, srcLang, tgtLang);

    // 7. Boolean/null literals
    line = applyLiterals(line, srcLang, tgtLang);

    // 8. Operator normalization
    line = transformOperators(line, srcLang, tgtLang);

    // 9. Semicolons — handled separately per block style

    return line;
}

// ─── Main export ──────────────────────────────────────────────────────────────

function emptyBody(tgtLang: string, baseIndent: string): string {
    const style = blockStyle(tgtLang);
    if (style === 'indent') { return `${baseIndent}pass`; }
    if (style === 'functional') {
        if (tgtLang === 'haskell') { return `${baseIndent}undefined`; }
        if (tgtLang === 'erlang') { return `${baseIndent}ok`; }
        if (tgtLang === 'elixir') { return `${baseIndent}:ok`; }
        if (tgtLang === 'clojure') { return `${baseIndent}nil`; }
        return `${baseIndent}()`; // OCaml, F#, etc.
    }
    return ''; // brace-style: empty body `{}` is valid
}

export function translateBody(
    originalSource: string,
    srcLang: string,
    tgtLang: string,
    baseIndent: string = '    ',
): string {
    if (!originalSource || originalSource.trim() === '') {
        return emptyBody(tgtLang, baseIndent);
    }

    // Same language: return body as-is (just strip the signature)
    if (srcLang === tgtLang) {
        const bodyLines = extractBodyLines(originalSource, srcLang);
        if (bodyLines.length === 0) {
            return emptyBody(tgtLang, baseIndent);
        }
        return bodyLines.map(l => `${baseIndent}${l}`).join('\n');
    }

    // Extract body lines
    let bodyLines = extractBodyLines(originalSource, srcLang);
    if (bodyLines.length === 0) {
        return emptyBody(tgtLang, baseIndent);
    }

    const srcStyle = blockStyle(srcLang);
    const tgtStyle = blockStyle(tgtLang);

    // Transform each line
    let transformed = bodyLines.map(l => transformLine(l, srcLang, tgtLang));

    // Block structure conversion
    if (srcStyle === 'brace' && tgtStyle === 'indent') {
        transformed = convertBracesToIndent(transformed);
    } else if (srcStyle === 'indent' && tgtStyle === 'brace') {
        transformed = convertIndentToBraces(transformed, tgtLang);
    }

    // Remove empty-string entries (removed closing braces) but keep blank lines
    const filtered = transformed.filter(l => l !== undefined && l !== null);

    // Add semicolons to statement lines (for brace-style targets)
    const result = filtered.map(l => {
        if (l.trim() === '') { return l; }
        // Don't add semicolons to control flow lines, braces, or comments
        const body = l.trimStart();
        const isCtrl = /^(if|else|for|while|do|switch|try|catch|finally|case|default)\b/.test(body) ||
                       body.startsWith('//') || body.startsWith('#') || body.startsWith('--') ||
                       body.startsWith('/*') || body.startsWith('*') ||
                       body.endsWith('{') || body === '}' || body === '};';
        if (!isCtrl && tgtStyle === 'brace') {
            return ensureSemicolon(l, tgtLang);
        }
        return l;
    });

    // Re-indent with base indent
    const indented = result.map(l => l === '' ? '' : `${baseIndent}${l}`);

    if (indented.length === 0) {
        return emptyBody(tgtLang, baseIndent);
    }

    return indented.join('\n');
}

// ─── Named re-export for convenience ─────────────────────────────────────────

export { blockStyle, commentPfx, usesSemicolon, lits };
