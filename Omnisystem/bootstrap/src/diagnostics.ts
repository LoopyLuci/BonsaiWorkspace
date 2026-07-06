// Diagnostics — source spans and richly formatted compiler errors.
//
// Every token and AST node carries a Span so the compiler can point at the
// exact source location. OmniError renders a caret-underlined snippet the way a
// modern, individual-friendly toolchain should: clear, colored, actionable.

export interface Pos {
  offset: number; // 0-based byte offset into the source
  line: number;   // 1-based
  col: number;    // 1-based
}

export interface Span {
  start: Pos;
  end: Pos;
}

export function span(start: Pos, end: Pos): Span {
  return { start, end };
}

export type Phase = 'lex' | 'parse' | 'resolve' | 'type' | 'runtime' | 'link';

const RESET = '\x1b[0m';
const BOLD = '\x1b[1m';
const DIM = '\x1b[2m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';

let COLOR = process.stdout.isTTY === true && !process.env.NO_COLOR;
export function setColor(on: boolean): void {
  COLOR = on;
}
function c(code: string, s: string): string {
  return COLOR ? code + s + RESET : s;
}

export class OmniError extends Error {
  phase: Phase;
  span: Span | null;
  file: string;
  source: string;
  help: string | null;

  constructor(
    phase: Phase,
    message: string,
    span: Span | null,
    file: string,
    source: string,
    help: string | null = null,
  ) {
    super(message);
    this.name = 'OmniError';
    this.phase = phase;
    this.span = span;
    this.file = file;
    this.source = source;
    this.help = help;
  }

  /** Render a GCC/rustc-style diagnostic with a source snippet and carets. */
  render(): string {
    const head = c(BOLD + RED, `error[${this.phase}]`) + c(BOLD, `: ${this.message}`);
    if (!this.span) {
      return `${head}\n  ${c(DIM, '-->')} ${this.file}`;
    }
    const { start, end } = this.span;
    const loc = `${this.file}:${start.line}:${start.col}`;
    const lines = this.source.split(/\r?\n/);
    const lineText = lines[start.line - 1] ?? '';
    const gutter = String(start.line);
    const pad = ' '.repeat(gutter.length);

    const underlineLen =
      end.line === start.line ? Math.max(1, end.col - start.col) : Math.max(1, lineText.length - start.col + 1);
    const caret = ' '.repeat(Math.max(0, start.col - 1)) + c(RED, '^'.repeat(underlineLen));

    let out = '';
    out += `${head}\n`;
    out += ` ${pad}${c(DIM + CYAN, '-->')} ${loc}\n`;
    out += ` ${pad}${c(DIM, '|')}\n`;
    out += ` ${c(DIM, gutter)} ${c(DIM, '|')} ${lineText}\n`;
    out += ` ${pad}${c(DIM, '|')} ${caret}\n`;
    if (this.help) {
      out += ` ${pad}${c(DIM, '=')} ${c(BOLD + YELLOW, 'help')}: ${this.help}\n`;
    }
    return out;
  }
}
