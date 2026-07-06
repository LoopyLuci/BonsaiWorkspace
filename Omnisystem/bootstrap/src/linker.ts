// OmniLinker — module resolution for the bootstrap. Titan programs `use` names
// from `omnisystem::` (the builtin prelude, always available) and may split
// code across multiple files. The linker loads a root file, follows local
// `mod name;` / sibling `.titan` files, and merges them into one Program whose
// items are registered together — the interpreter then resolves cross-module
// references by name (a single flat namespace for the runnable subset).

import { readFileSync, existsSync } from 'fs';
import { dirname, join, basename } from 'path';
import type * as A from './ast.ts';
import { parse } from './parser.ts';

export interface LinkedProgram {
  items: A.Item[];
  file: string;
  source: string;               // source of the root file (for diagnostics)
  sources: Map<string, string>; // file -> source, for multi-file diagnostics
}

export function link(rootFile: string): LinkedProgram {
  const sources = new Map<string, string>();
  const items: A.Item[] = [];
  const seen = new Set<string>();

  const loadFile = (file: string): void => {
    const abs = file;
    if (seen.has(abs)) return;
    seen.add(abs);
    const src = readFileSync(abs, 'utf8');
    sources.set(abs, src);
    const prog = parse(src, abs);
    for (const item of prog.items) {
      items.push(item);
      // resolve `mod name;` to a sibling file name.titan or name/mod.titan
      if (item.kind === 'mod' && item.items.length === 0) {
        const dir = dirname(abs);
        const candidates = [
          join(dir, item.name + '.titan'),
          join(dir, item.name, 'mod.titan'),
        ];
        for (const c of candidates) {
          if (existsSync(c)) { loadFile(c); break; }
        }
      }
    }
  };

  loadFile(rootFile);
  const rootSrc = sources.get(rootFile) ?? '';
  return { items, file: rootFile, source: rootSrc, sources };
}
