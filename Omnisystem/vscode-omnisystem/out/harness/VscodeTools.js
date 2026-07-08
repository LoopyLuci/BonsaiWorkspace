"use strict";
// VscodeTools — the bridge that lets an OmniHarness agent actually control VS Code
// and work on the user's project: read/write/edit files, list & search, run
// commands, inspect diagnostics and the active selection.
//
// Every mutating or executing tool passes through an async approval gate so the
// user stays in control (unless an agent is explicitly set to auto-approve).
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.VscodeTools = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const child_process_1 = require("child_process");
const MAX_READ_BYTES = 200000;
const MAX_SEARCH_HITS = 60;
const MAX_CMD_OUTPUT = 30000;
class VscodeTools {
    constructor(approve) {
        this.approve = approve;
    }
    /** Static catalog used to build the model's system prompt and the UI. */
    static catalog() {
        return [
            { name: 'read_file', mutating: false, description: 'Read a file from the workspace. Returns its text content.',
                params: [{ name: 'path', type: 'string', description: 'Workspace-relative or absolute path', required: true }] },
            { name: 'list_dir', mutating: false, description: 'List the entries of a directory in the workspace.',
                params: [{ name: 'path', type: 'string', description: 'Directory path (default: workspace root)', required: false }] },
            { name: 'search', mutating: false, description: 'Search file contents for a substring/regex. Returns matching path:line: text.',
                params: [
                    { name: 'query', type: 'string', description: 'Text or regex to search for', required: true },
                    { name: 'glob', type: 'string', description: 'Optional include glob, e.g. **/*.ts', required: false },
                ] },
            { name: 'write_file', mutating: true, description: 'Create or overwrite a file with the given content.',
                params: [
                    { name: 'path', type: 'string', description: 'File path to write', required: true },
                    { name: 'content', type: 'string', description: 'Full file content', required: true },
                ] },
            { name: 'edit_file', mutating: true, description: 'Replace the first exact occurrence of old_text with new_text in a file.',
                params: [
                    { name: 'path', type: 'string', description: 'File to edit', required: true },
                    { name: 'old_text', type: 'string', description: 'Exact text to find (include enough context to be unique)', required: true },
                    { name: 'new_text', type: 'string', description: 'Replacement text', required: true },
                ] },
            { name: 'run_command', mutating: true, description: 'Run a shell command in the workspace root and return its output.',
                params: [{ name: 'command', type: 'string', description: 'The command line to execute', required: true }] },
            { name: 'open_file', mutating: false, description: 'Open a file in the editor (optionally at a line).',
                params: [
                    { name: 'path', type: 'string', description: 'File to open', required: true },
                    { name: 'line', type: 'number', description: 'Line number to reveal (1-based)', required: false },
                ] },
            { name: 'get_diagnostics', mutating: false, description: 'Get compiler/linter problems for a file or the whole workspace.',
                params: [{ name: 'path', type: 'string', description: 'Optional file path; omit for all', required: false }] },
            { name: 'get_selection', mutating: false, description: 'Get the active editor file path and the currently selected text.',
                params: [] },
        ];
    }
    /**
     * Convert the tool catalog to OpenAI/Anthropic-style function schemas for
     * native function calling. Optionally restrict to an allow-list of names.
     */
    static toFunctionSchemas(allowed) {
        const wildcard = !allowed || (allowed.length === 1 && allowed[0] === '*');
        const set = wildcard ? null : new Set(allowed);
        return VscodeTools.catalog()
            .filter((t) => !set || set.has(t.name))
            .map((t) => {
            const properties = {};
            const required = [];
            for (const p of t.params) {
                properties[p.name] = { type: p.type, description: p.description };
                if (p.required) {
                    required.push(p.name);
                }
            }
            return {
                name: t.name,
                description: t.description,
                parameters: { type: 'object', properties, required },
            };
        });
    }
    // ── Path helpers ─────────────────────────────────────────────────────────
    root() {
        return vscode.workspace.workspaceFolders?.[0]?.uri;
    }
    resolve(p) {
        if (path.isAbsolute(p)) {
            return vscode.Uri.file(p);
        }
        const root = this.root();
        if (!root) {
            return vscode.Uri.file(p);
        }
        return vscode.Uri.joinPath(root, p);
    }
    rel(uri) {
        const root = this.root();
        if (!root) {
            return uri.fsPath;
        }
        return path.relative(root.fsPath, uri.fsPath) || path.basename(uri.fsPath);
    }
    // ── Dispatch ─────────────────────────────────────────────────────────────
    async execute(name, args) {
        try {
            switch (name) {
                case 'read_file': return await this.readFile(args);
                case 'list_dir': return await this.listDir(args);
                case 'search': return await this.search(args);
                case 'write_file': return await this.writeFile(args);
                case 'edit_file': return await this.editFile(args);
                case 'run_command': return await this.runCommand(args);
                case 'open_file': return await this.openFile(args);
                case 'get_diagnostics': return await this.getDiagnostics(args);
                case 'get_selection': return await this.getSelection();
                default:
                    return { ok: false, summary: `Unknown tool: ${name}`, error: `No such tool "${name}".` };
            }
        }
        catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            return { ok: false, summary: `${name} failed`, error: msg };
        }
    }
    // ── Read-only tools ──────────────────────────────────────────────────────
    async readFile(args) {
        const p = String(args.path ?? '');
        if (!p) {
            return { ok: false, summary: 'read_file: missing path', error: 'path is required' };
        }
        const uri = this.resolve(p);
        const bytes = await vscode.workspace.fs.readFile(uri);
        if (bytes.byteLength > MAX_READ_BYTES) {
            const text = new TextDecoder().decode(bytes.slice(0, MAX_READ_BYTES));
            return { ok: true, summary: `Read ${this.rel(uri)} (truncated)`, content: text + '\n… [truncated]' };
        }
        const text = new TextDecoder().decode(bytes);
        return { ok: true, summary: `Read ${this.rel(uri)} (${bytes.byteLength} bytes)`, content: text };
    }
    async listDir(args) {
        const p = args.path ? String(args.path) : '.';
        const uri = this.resolve(p);
        const entries = await vscode.workspace.fs.readDirectory(uri);
        const lines = entries
            .sort((a, b) => (b[1] - a[1]) || a[0].localeCompare(b[0]))
            .map(([n, t]) => (t === vscode.FileType.Directory ? `${n}/` : n));
        return { ok: true, summary: `Listed ${this.rel(uri)} (${entries.length} entries)`, content: lines.join('\n') };
    }
    async search(args) {
        const query = String(args.query ?? '');
        if (!query) {
            return { ok: false, summary: 'search: missing query', error: 'query is required' };
        }
        const glob = args.glob ? String(args.glob) : '**/*';
        let re;
        try {
            re = new RegExp(query, 'i');
        }
        catch {
            re = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i');
        }
        const files = await vscode.workspace.findFiles(glob, '**/{node_modules,.git,out,target,dist}/**', 400);
        const hits = [];
        for (const file of files) {
            if (hits.length >= MAX_SEARCH_HITS) {
                break;
            }
            let text;
            try {
                const bytes = await vscode.workspace.fs.readFile(file);
                if (bytes.byteLength > MAX_READ_BYTES) {
                    continue;
                }
                text = new TextDecoder().decode(bytes);
            }
            catch {
                continue;
            }
            const lines = text.split('\n');
            for (let i = 0; i < lines.length; i++) {
                if (re.test(lines[i])) {
                    hits.push(`${this.rel(file)}:${i + 1}: ${lines[i].trim().slice(0, 200)}`);
                    if (hits.length >= MAX_SEARCH_HITS) {
                        break;
                    }
                }
            }
        }
        return {
            ok: true,
            summary: `search "${query}" — ${hits.length} hit(s)`,
            content: hits.length ? hits.join('\n') : '(no matches)',
        };
    }
    async openFile(args) {
        const p = String(args.path ?? '');
        if (!p) {
            return { ok: false, summary: 'open_file: missing path', error: 'path is required' };
        }
        const uri = this.resolve(p);
        const doc = await vscode.workspace.openTextDocument(uri);
        const editor = await vscode.window.showTextDocument(doc, { preview: false });
        if (args.line !== undefined) {
            const line = Math.max(0, Number(args.line) - 1);
            const pos = new vscode.Position(line, 0);
            editor.selection = new vscode.Selection(pos, pos);
            editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
        }
        return { ok: true, summary: `Opened ${this.rel(uri)}`, content: `Opened ${this.rel(uri)}` };
    }
    async getDiagnostics(args) {
        const sev = (s) => s === vscode.DiagnosticSeverity.Error ? 'error'
            : s === vscode.DiagnosticSeverity.Warning ? 'warning'
                : s === vscode.DiagnosticSeverity.Information ? 'info' : 'hint';
        const format = (uri, diags) => diags.map((d) => `${this.rel(uri)}:${d.range.start.line + 1}:${d.range.start.character + 1}: ${sev(d.severity)}: ${d.message}`);
        let lines = [];
        if (args.path) {
            const uri = this.resolve(String(args.path));
            lines = format(uri, vscode.languages.getDiagnostics(uri));
        }
        else {
            for (const [uri, diags] of vscode.languages.getDiagnostics()) {
                if (diags.length) {
                    lines.push(...format(uri, diags));
                }
                if (lines.length > 300) {
                    break;
                }
            }
        }
        return { ok: true, summary: `${lines.length} problem(s)`, content: lines.length ? lines.join('\n') : '(no problems)' };
    }
    async getSelection() {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            return { ok: true, summary: 'No active editor', content: '(no active editor)' };
        }
        const sel = editor.selection;
        const text = editor.document.getText(sel.isEmpty ? undefined : sel);
        const p = this.rel(editor.document.uri);
        const where = sel.isEmpty ? '(whole file)' : `lines ${sel.start.line + 1}-${sel.end.line + 1}`;
        return {
            ok: true,
            summary: `Selection in ${p} ${where}`,
            content: `File: ${p}\n${where}\n\n${text.slice(0, MAX_READ_BYTES)}`,
        };
    }
    // ── Mutating tools (approval-gated) ──────────────────────────────────────
    async writeFile(args) {
        const p = String(args.path ?? '');
        const content = String(args.content ?? '');
        if (!p) {
            return { ok: false, summary: 'write_file: missing path', error: 'path is required' };
        }
        const uri = this.resolve(p);
        let existed = true;
        let beforeContent = null;
        try {
            const bytes = await vscode.workspace.fs.readFile(uri);
            beforeContent = new TextDecoder().decode(bytes);
        }
        catch {
            existed = false;
        }
        const preview = `${existed ? 'Overwrite' : 'Create'} ${this.rel(uri)} (${content.length} chars)`;
        const diff = { relPath: this.rel(uri), before: beforeContent ?? '', after: content };
        if (!(await this.approve('write_file', args, preview, diff))) {
            return { ok: false, summary: 'write_file rejected', error: 'User rejected the write.' };
        }
        await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(content));
        return {
            ok: true,
            summary: `${existed ? 'Wrote' : 'Created'} ${this.rel(uri)}`,
            content: `${existed ? 'Wrote' : 'Created'} ${this.rel(uri)} (${content.length} chars)`,
            diff,
            checkpoint: { fsPath: uri.fsPath, before: beforeContent },
        };
    }
    async editFile(args) {
        const p = String(args.path ?? '');
        const oldText = String(args.old_text ?? '');
        const newText = String(args.new_text ?? '');
        if (!p || !oldText) {
            return { ok: false, summary: 'edit_file: missing args', error: 'path and old_text are required' };
        }
        const uri = this.resolve(p);
        const doc = await vscode.workspace.openTextDocument(uri);
        const full = doc.getText();
        const idx = full.indexOf(oldText);
        if (idx === -1) {
            return { ok: false, summary: 'edit_file: text not found', error: `old_text not found in ${this.rel(uri)}.` };
        }
        if (full.indexOf(oldText, idx + 1) !== -1) {
            return { ok: false, summary: 'edit_file: text not unique', error: 'old_text matches multiple locations; add more context to make it unique.' };
        }
        const startPos = doc.positionAt(idx);
        const endPos = doc.positionAt(idx + oldText.length);
        const preview = `Edit ${this.rel(uri)} @ line ${startPos.line + 1} (-${oldText.length}/+${newText.length} chars)`;
        const diff = { relPath: this.rel(uri), before: oldText, after: newText };
        if (!(await this.approve('edit_file', args, preview, diff))) {
            return { ok: false, summary: 'edit_file rejected', error: 'User rejected the edit.' };
        }
        const edit = new vscode.WorkspaceEdit();
        edit.replace(uri, new vscode.Range(startPos, endPos), newText);
        const applied = await vscode.workspace.applyEdit(edit);
        if (!applied) {
            return { ok: false, summary: 'edit_file failed', error: 'applyEdit returned false' };
        }
        await doc.save();
        return {
            ok: true,
            summary: `Edited ${this.rel(uri)} @ line ${startPos.line + 1}`,
            content: `Applied edit to ${this.rel(uri)}.`,
            diff,
            checkpoint: { fsPath: uri.fsPath, before: full },
        };
    }
    async runCommand(args) {
        const command = String(args.command ?? '');
        if (!command) {
            return { ok: false, summary: 'run_command: missing command', error: 'command is required' };
        }
        if (!(await this.approve('run_command', args, `Run: ${command}`))) {
            return { ok: false, summary: 'run_command rejected', error: 'User rejected the command.' };
        }
        const cwd = this.root()?.fsPath ?? process.cwd();
        return new Promise((resolve) => {
            const child = (0, child_process_1.spawn)(command, { cwd, shell: true });
            let out = '';
            let err = '';
            const cap = (buf, isErr) => {
                if (isErr) {
                    err += buf.toString();
                }
                else {
                    out += buf.toString();
                }
                if (out.length + err.length > MAX_CMD_OUTPUT) {
                    child.kill();
                }
            };
            child.stdout.on('data', (d) => cap(d, false));
            child.stderr.on('data', (d) => cap(d, true));
            const timer = setTimeout(() => child.kill(), 120000);
            child.on('close', (code) => {
                clearTimeout(timer);
                const combined = (out + (err ? `\n[stderr]\n${err}` : '')).slice(0, MAX_CMD_OUTPUT);
                resolve({
                    ok: code === 0,
                    summary: `$ ${command.slice(0, 60)} → exit ${code}`,
                    content: `exit code: ${code}\n\n${combined || '(no output)'}`,
                    error: code === 0 ? undefined : `Command exited with code ${code}`,
                });
            });
            child.on('error', (e) => {
                clearTimeout(timer);
                resolve({ ok: false, summary: `run_command failed`, error: e.message });
            });
        });
    }
}
exports.VscodeTools = VscodeTools;
//# sourceMappingURL=VscodeTools.js.map