#!/usr/bin/env node
// OmniCC — the Omnisystem compiler driver (bootstrap seed).
//
// Runs directly under Node 24 (TypeScript type-stripping, no build step):
//     node src/cli.ts run  program.titan
//     node src/cli.ts tokens program.titan
//     node src/cli.ts ast   program.titan
//     node src/cli.ts check program.titan
//     node src/cli.ts test  [dir]
//     node src/cli.ts build program.titan   (parse+check, reports readiness)
//
// Designed for individuals: zero install, zero config, clear diagnostics.

import { readFileSync, readdirSync, statSync } from 'fs';
import { join, extname, basename } from 'path';
import { OmniError, setColor } from './diagnostics.ts';
import { Lexer } from './lexer.ts';
import { parse } from './parser.ts';
import { link } from './linker.ts';
import { Interpreter } from './interpreter.ts';

const VERSION = '0.1.0';

const BOLD = '\x1b[1m', DIM = '\x1b[2m', RESET = '\x1b[0m';
const GREEN = '\x1b[32m', RED = '\x1b[31m', CYAN = '\x1b[36m';
const useColor = process.stdout.isTTY === true && !process.env.NO_COLOR;
function paint(code: string, s: string): string { return useColor ? code + s + RESET : s; }

function banner(): void {
  process.stdout.write(
    paint(BOLD + CYAN, `OmniCC ${VERSION}`) + paint(DIM, ` — Omnisystem bootstrap compiler (Titan)\n`),
  );
}

function usage(): void {
  banner();
  process.stdout.write(`
${paint(BOLD, 'Usage')}: omnicc <command> [args]

${paint(BOLD, 'Commands')}:
  run <file>       Compile and execute a Titan program
  check <file>     Parse + resolve without running (fast feedback)
  build <file>     Check and report build readiness
  tokens <file>    Dump the token stream
  ast <file>       Dump the parsed AST as JSON
  test [dir]       Run every *.titan in dir (default: tests/) and check //@ expect
  version          Print version
  help             Show this help
`);
}

function readSource(file: string): string {
  try {
    return readFileSync(file, 'utf8');
  } catch {
    process.stderr.write(paint(RED, `error`) + `: cannot read file '${file}'\n`);
    process.exit(2);
  }
}

function reportError(e: unknown): void {
  if (e instanceof OmniError) {
    process.stderr.write('\n' + e.render() + '\n');
  } else {
    process.stderr.write(paint(RED, 'internal error') + `: ${(e as Error).message}\n${(e as Error).stack ?? ''}\n`);
  }
}

function cmdTokens(file: string): number {
  const src = readSource(file);
  try {
    const toks = new Lexer(src, file).tokenize();
    for (const t of toks) {
      const loc = `${t.span.start.line}:${t.span.start.col}`;
      process.stdout.write(`${paint(DIM, loc.padEnd(7))} ${paint(CYAN, t.kind.padEnd(8))} ${JSON.stringify(t.value)}\n`);
    }
    return 0;
  } catch (e) { reportError(e); return 1; }
}

function cmdAst(file: string): number {
  const src = readSource(file);
  try {
    const prog = parse(src, file);
    process.stdout.write(JSON.stringify(prog.items, (k, v) => (k === 'span' ? undefined : v), 2) + '\n');
    return 0;
  } catch (e) { reportError(e); return 1; }
}

function cmdCheck(file: string, quiet = false): number {
  try {
    const linked = link(file);
    const intr = new Interpreter(linked.file, linked.source);
    intr.register({ items: linked.items, file: linked.file, source: linked.source });
    if (!quiet) {
      const structs = intr.structs.size, enums = intr.enums.size, fns = intr.fns.size;
      let methods = 0;
      for (const t of intr.methods.values()) methods += t.size;
      process.stdout.write(paint(GREEN, '✓ check passed') +
        paint(DIM, `  (${structs} structs, ${enums} enums, ${fns} fns, ${methods} methods)\n`));
    }
    return 0;
  } catch (e) { reportError(e); return 1; }
}

function cmdRun(file: string): number {
  try {
    const linked = link(file);
    const intr = new Interpreter(linked.file, linked.source);
    intr.register({ items: linked.items, file: linked.file, source: linked.source });
    const res = intr.runMain();
    process.stdout.write(res.stdout);
    return res.exitCode;
  } catch (e) { reportError(e); return 1; }
}

function cmdBuild(file: string): number {
  banner();
  const t0 = Date.now();
  const rc = cmdCheck(file, true);
  const ms = Date.now() - t0;
  if (rc === 0) {
    process.stdout.write(paint(GREEN, `✓ build ready`) + paint(DIM, `  ${basename(file)} compiled in ${ms}ms\n`));
    process.stdout.write(paint(DIM, `  run it: `) + `omnicc run ${file}\n`);
  }
  return rc;
}

// Each test file may declare an expectation as a comment:
//   //@ expect-stdout: <exact single line>
//   //@ expect-exit: <code>
//   //@ expect-error            (compile/runtime error is the pass condition)
function cmdTest(dir: string): number {
  let files: string[] = [];
  try {
    files = readdirSync(dir).filter((f) => extname(f) === '.titan').map((f) => join(dir, f)).sort();
  } catch {
    process.stderr.write(paint(RED, `error`) + `: no test directory '${dir}'\n`);
    return 2;
  }
  let pass = 0, fail = 0;
  for (const file of files) {
    const src = readFileSync(file, 'utf8');
    const wantErr = /\/\/@\s*expect-error/.test(src);
    const mOut = src.match(/\/\/@\s*expect-stdout:\s?(.*)/);
    const mExit = src.match(/\/\/@\s*expect-exit:\s*(-?\d+)/);
    const expectStdout = mOut ? mOut.slice(1).join('\n') : null;
    const expectExit = mExit ? Number(mExit[1]) : null;

    let stdout = '', exit = 0, errored = false, errMsg = '';
    try {
      const linked = link(file);
      const intr = new Interpreter(linked.file, linked.source);
      intr.register({ items: linked.items, file: linked.file, source: linked.source });
      const res = intr.runMain();
      stdout = res.stdout; exit = res.exitCode;
    } catch (e) {
      errored = true;
      errMsg = e instanceof OmniError ? e.message : (e as Error).message;
    }

    const problems: string[] = [];
    if (wantErr && !errored) problems.push('expected an error but program succeeded');
    if (!wantErr && errored) problems.push(`unexpected error: ${errMsg}`);
    if (expectStdout !== null) {
      const got = stdout.replace(/\n$/, '');
      if (got !== expectStdout) problems.push(`stdout mismatch\n     expected: ${JSON.stringify(expectStdout)}\n     got:      ${JSON.stringify(got)}`);
    }
    if (expectExit !== null && exit !== expectExit) problems.push(`exit ${exit}, expected ${expectExit}`);

    if (problems.length === 0) {
      pass++;
      process.stdout.write(paint(GREEN, '  ✓ ') + basename(file) + '\n');
    } else {
      fail++;
      process.stdout.write(paint(RED, '  ✗ ') + basename(file) + '\n');
      for (const p of problems) process.stdout.write(paint(DIM, '      ' + p.replace(/\n/g, '\n      ')) + '\n');
    }
  }
  process.stdout.write('\n' + (fail === 0
    ? paint(BOLD + GREEN, `${pass}/${pass} passed`)
    : paint(BOLD + RED, `${pass} passed, ${fail} failed`)) + '\n');
  return fail === 0 ? 0 : 1;
}

function main(argv: string[]): number {
  const [cmd, ...rest] = argv;
  switch (cmd) {
    case 'run': return rest[0] ? cmdRun(rest[0]) : (usage(), 2);
    case 'check': return rest[0] ? cmdCheck(rest[0]) : (usage(), 2);
    case 'build': return rest[0] ? cmdBuild(rest[0]) : (usage(), 2);
    case 'tokens': return rest[0] ? cmdTokens(rest[0]) : (usage(), 2);
    case 'ast': return rest[0] ? cmdAst(rest[0]) : (usage(), 2);
    case 'test': return cmdTest(rest[0] ?? 'tests');
    case 'version': case '--version': case '-v': banner(); return 0;
    case 'help': case '--help': case '-h': case undefined: usage(); return 0;
    default:
      process.stderr.write(paint(RED, `error`) + `: unknown command '${cmd}'\n`);
      usage();
      return 2;
  }
}

process.exit(main(process.argv.slice(2)));
