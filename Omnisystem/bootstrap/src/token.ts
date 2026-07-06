// Token definitions for the Titan lexer.

import type { Span } from './diagnostics.ts';

export type TokKind =
  // literals
  | 'int' | 'float' | 'string' | 'char' | 'bool'
  // identifiers & keywords
  | 'ident' | 'keyword' | 'lifetime'
  // punctuation / operators
  | 'op'
  // structural
  | 'eof';

export interface Token {
  kind: TokKind;
  value: string;      // raw lexeme (for strings/chars: the decoded value)
  raw: string;        // exact source text
  span: Span;
}

// Reserved words recognised by the Titan front end. Generics/traits/lifetimes
// are parsed; a runnable subset is interpreted.
export const KEYWORDS = new Set<string>([
  'use', 'pub', 'mod', 'struct', 'enum', 'impl', 'trait', 'fn', 'let', 'mut',
  'const', 'static', 'if', 'else', 'match', 'while', 'for', 'in', 'loop',
  'break', 'continue', 'return', 'self', 'Self', 'true', 'false', 'as', 'where',
  'ref', 'move', 'dyn', 'async', 'await', 'type', 'unsafe', 'extern',
]);

// Multi-character operators, longest first so the lexer is greedy.
export const OPERATORS: string[] = [
  '..=', '...', '<<=', '>>=',
  '->', '=>', '::', '==', '!=', '<=', '>=', '&&', '||', '+=', '-=', '*=', '/=',
  '%=', '&=', '|=', '^=', '<<', '>>', '..',
  '+', '-', '*', '/', '%', '=', '<', '>', '!', '&', '|', '^', '~', '?',
  '.', ',', ';', ':', '(', ')', '{', '}', '[', ']', '@', '#',
];
