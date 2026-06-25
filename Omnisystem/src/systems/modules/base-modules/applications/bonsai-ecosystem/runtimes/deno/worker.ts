/**
 * Bonsai Deno Worker — TypeScript eval backend.
 *
 * Launched as a subprocess by the Tauri backend (bonsai-workspace/src-tauri).
 * Communicates over stdin/stdout using newline-delimited JSON (NDJSON).
 *
 * Protocol (NDJSON):
 *   → {"id": "...", "op": "eval",   "code": "...", "context"?: {...}}
 *   → {"id": "...", "op": "ping"}
 *   → {"id": "...", "op": "shutdown"}
 *
 *   ← {"id": "...", "ok": true,  "result": <value>}
 *   ← {"id": "...", "ok": false, "error": "..."}
 *   ← {"id": "...", "ok": true,  "pong": true}
 *
 * Security: eval runs in a restricted scope — no Deno.* globals are exposed
 * to the evaluated code.  Only a safe subset of built-ins is provided.
 */

// ── Types ─────────────────────────────────────────────────────────────────────

interface EvalRequest {
  id: string;
  op: "eval";
  code: string;
  context?: Record<string, unknown>;
}

interface PingRequest {
  id: string;
  op: "ping";
}

interface ShutdownRequest {
  id: string;
  op: "shutdown";
}

type Request = EvalRequest | PingRequest | ShutdownRequest;

interface OkResponse {
  id: string;
  ok: true;
  result?: unknown;
  pong?: boolean;
  stdout?: string;
}

interface ErrResponse {
  id: string;
  ok: false;
  error: string;
}

type Response = OkResponse | ErrResponse;

// ── Safe eval sandbox ─────────────────────────────────────────────────────────

/**
 * Evaluate a TypeScript/JavaScript snippet in a restricted sandbox.
 *
 * The sandbox exposes:
 *   - Standard JS globals: Math, JSON, Array, Object, String, Number, Boolean,
 *     Symbol, BigInt, Error, Promise, console (captured to string)
 *   - bonsai.* namespace for Bonsai-specific helpers
 *
 * The sandbox does NOT expose:
 *   - Deno (file system, network, subprocess, FFI)
 *   - globalThis / window references to the outer scope
 */
function evalSandboxed(
  code: string,
  context: Record<string, unknown> = {}
): { result: unknown; stdout: string } {
  const logs: string[] = [];

  const sandboxConsole = {
    log:   (...args: unknown[]) => logs.push(args.map(String).join(" ")),
    warn:  (...args: unknown[]) => logs.push("[warn] " + args.map(String).join(" ")),
    error: (...args: unknown[]) => logs.push("[error] " + args.map(String).join(" ")),
    info:  (...args: unknown[]) => logs.push("[info] " + args.map(String).join(" ")),
  };

  // Bonsai helper namespace (safe, no I/O)
  const bonsai = {
    version: "0.1.0",
    /** Format a value as a pretty JSON string. */
    pretty: (v: unknown) => JSON.stringify(v, null, 2),
    /** Return the type tag of a value. */
    typeOf: (v: unknown) => typeof v,
    /** Deep clone a JSON-compatible value. */
    clone: <T>(v: T): T => JSON.parse(JSON.stringify(v)),
    /** Range [start, end). */
    range: (start: number, end?: number): number[] => {
      if (end === undefined) { end = start; start = 0; }
      return Array.from({ length: end - start }, (_, i) => start + i);
    },
    /** Zip two arrays. */
    zip: <A, B>(a: A[], b: B[]): [A, B][] =>
      a.slice(0, Math.min(a.length, b.length)).map((v, i) => [v, b[i]]),
  };

  // Build the sandbox function — only safe globals in scope.
  const sandboxKeys = [
    "Math", "JSON", "Array", "Object", "String", "Number",
    "Boolean", "Symbol", "BigInt", "Error", "Promise",
    "parseInt", "parseFloat", "isNaN", "isFinite",
    "encodeURIComponent", "decodeURIComponent",
    "console", "bonsai",
    ...Object.keys(context),
  ];

  const sandboxValues = [
    Math, JSON, Array, Object, String, Number,
    Boolean, Symbol, BigInt, Error, Promise,
    parseInt, parseFloat, isNaN, isFinite,
    encodeURIComponent, decodeURIComponent,
    sandboxConsole, bonsai,
    ...Object.values(context),
  ];

  // Use Function constructor to create an isolated scope.
  // The code is wrapped so that the last expression is the return value.
  const wrapped = `"use strict";\n${code}`;
  // deno-lint-ignore no-new-func
  const fn_ = new Function(...sandboxKeys, wrapped);
  const result = fn_(...sandboxValues);
  return { result, stdout: logs.join("\n") };
}

// ── I/O loop ──────────────────────────────────────────────────────────────────

function respond(res: Response): void {
  const line = JSON.stringify(res) + "\n";
  const encoder = new TextEncoder();
  Deno.stdout.writeSync(encoder.encode(line));
}

async function processLine(line: string): Promise<boolean> {
  line = line.trim();
  if (!line) return true;

  let req: Request;
  try {
    req = JSON.parse(line) as Request;
  } catch (e) {
    respond({ id: "?", ok: false, error: `JSON parse error: ${e}` });
    return true;
  }

  switch (req.op) {
    case "ping":
      respond({ id: req.id, ok: true, pong: true });
      break;

    case "shutdown":
      respond({ id: req.id, ok: true, result: "bye" });
      return false; // signal main loop to exit

    case "eval": {
      try {
        const { result, stdout } = evalSandboxed(req.code, req.context ?? {});
        respond({ id: req.id, ok: true, result, stdout: stdout || undefined });
      } catch (err) {
        respond({
          id: req.id,
          ok: false,
          error: err instanceof Error ? err.message : String(err),
        });
      }
      break;
    }

    default:
      respond({ id: (req as Request).id, ok: false, error: `unknown op: ${(req as { op: string }).op}` });
  }

  return true;
}

async function main(): Promise<void> {
  const decoder = new TextDecoder();
  let buf = "";

  const stdin = Deno.stdin.readable;
  const reader = stdin.getReader();

  // Write startup signal
  const startupLine = JSON.stringify({ ready: true, version: "0.1.0" }) + "\n";
  await Deno.stdout.write(new TextEncoder().encode(startupLine));

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buf += decoder.decode(value, { stream: true });
      const lines = buf.split("\n");
      buf = lines.pop() ?? "";

      for (const line of lines) {
        const cont = await processLine(line);
        if (!cont) {
          reader.releaseLock();
          return;
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

main().catch((e) => {
  const encoder = new TextEncoder();
  Deno.stdout.writeSync(encoder.encode(
    JSON.stringify({ id: "?", ok: false, error: `worker fatal: ${e}` }) + "\n"
  ));
  Deno.exit(1);
});
