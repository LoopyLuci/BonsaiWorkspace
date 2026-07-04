"use strict";
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
exports.SylvaNotebookKernel = exports.SylvaNotebookSerializer = void 0;
const vscode = __importStar(require("vscode"));
const child_process_1 = require("child_process");
// ── Serializer ───────────────────────────────────────────────────────────────
class SylvaNotebookSerializer {
    async deserializeNotebook(content) {
        let nb = { version: 1, cells: [] };
        try {
            nb = JSON.parse(Buffer.from(content).toString('utf-8'));
        }
        catch {
            // Return empty notebook on parse error
        }
        const cells = nb.cells.map(raw => {
            const kind = raw.kind === 'markdown'
                ? vscode.NotebookCellKind.Markup
                : vscode.NotebookCellKind.Code;
            const outputs = raw.outputs
                .filter(o => o.mime && o.data)
                .map(o => new vscode.NotebookCellOutput([
                vscode.NotebookCellOutputItem.text(o.data, o.mime)
            ]));
            const cell = new vscode.NotebookCellData(kind, raw.value, raw.language ?? 'sylva');
            cell.outputs = outputs;
            cell.metadata = raw.metadata ?? {};
            return cell;
        });
        const data = new vscode.NotebookData(cells);
        data.metadata = { version: nb.version };
        return data;
    }
    async serializeNotebook(data) {
        const cells = data.cells.map(cell => {
            const outputs = (cell.outputs ?? []).flatMap(out => out.items.map(item => ({
                mime: item.mime,
                data: Buffer.from(item.data).toString('utf-8')
            })));
            return {
                kind: cell.kind === vscode.NotebookCellKind.Markup ? 'markdown' : 'code',
                value: cell.value,
                language: cell.languageId ?? 'sylva',
                outputs,
                metadata: cell.metadata
            };
        });
        const nb = {
            version: data.metadata?.['version'] ?? 1,
            cells
        };
        return Buffer.from(JSON.stringify(nb, null, 2), 'utf-8');
    }
}
exports.SylvaNotebookSerializer = SylvaNotebookSerializer;
// ── Kernel ───────────────────────────────────────────────────────────────────
class SylvaNotebookKernel {
    constructor() {
        this.id = 'sylva-kernel';
        this.label = 'SYLVA ML Kernel';
        this.supportedLanguages = ['sylva'];
        this._executionOrder = 0;
        this._controller = vscode.notebooks.createNotebookController(this.id, 'sylva-notebook', this.label);
        this._controller.supportedLanguages = this.supportedLanguages;
        this._controller.supportsExecutionOrder = true;
        this._controller.description = 'Execute SYLVA ML code via omnicc';
        this._controller.executeHandler = this._execute.bind(this);
    }
    dispose() {
        this._controller.dispose();
    }
    // ── Execution ────────────────────────────────────────────────────────────────
    async _execute(cells, _notebook, _controller) {
        for (const cell of cells) {
            await this._executeCell(cell);
        }
    }
    async _executeCell(cell) {
        const execution = this._controller.createNotebookCellExecution(cell);
        execution.executionOrder = ++this._executionOrder;
        execution.start(Date.now());
        execution.clearOutput();
        const source = cell.document.getText();
        try {
            const result = await this._runSylva(source, execution);
            execution.end(result.success, Date.now());
        }
        catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            await execution.replaceOutput([
                new vscode.NotebookCellOutput([
                    vscode.NotebookCellOutputItem.error({ name: 'Error', message: msg })
                ])
            ]);
            execution.end(false, Date.now());
        }
    }
    async _runSylva(source, execution) {
        return new Promise(resolve => {
            const omniccPath = vscode.workspace
                .getConfiguration('omnisystem')
                .get('omniccPath', 'omnicc');
            const proc = (0, child_process_1.spawn)(omniccPath, ['eval', '--language', 'sylva'], {
                stdio: ['pipe', 'pipe', 'pipe'],
                windowsHide: true
            });
            const stdoutChunks = [];
            const stderrChunks = [];
            proc.stdout.on('data', (chunk) => stdoutChunks.push(chunk.toString()));
            proc.stderr.on('data', (chunk) => stderrChunks.push(chunk.toString()));
            proc.stdin.write(source);
            proc.stdin.end();
            proc.on('error', (err) => {
                void execution.replaceOutput([
                    new vscode.NotebookCellOutput([
                        vscode.NotebookCellOutputItem.error({ name: 'SpawnError', message: err.message })
                    ])
                ]);
                resolve({ success: false });
            });
            proc.on('close', async (code) => {
                const stdout = stdoutChunks.join('');
                const stderr = stderrChunks.join('');
                const outputs = [];
                if (stderr) {
                    outputs.push(new vscode.NotebookCellOutput([
                        vscode.NotebookCellOutputItem.stderr(stderr)
                    ]));
                }
                if (stdout) {
                    // Detect special markers and render accordingly
                    const lossLines = this._extractLoss(stdout);
                    const modelSummary = this._extractModel(stdout);
                    const plainText = this._stripSpecialMarkers(stdout);
                    if (lossLines.length > 0) {
                        const chart = this._renderLossChart(lossLines);
                        outputs.push(new vscode.NotebookCellOutput([
                            vscode.NotebookCellOutputItem.text(chart, 'text/plain')
                        ]));
                    }
                    if (modelSummary) {
                        outputs.push(new vscode.NotebookCellOutput([
                            vscode.NotebookCellOutputItem.text(modelSummary, 'text/plain')
                        ]));
                    }
                    if (plainText.trim()) {
                        outputs.push(new vscode.NotebookCellOutput([
                            vscode.NotebookCellOutputItem.stdout(plainText)
                        ]));
                    }
                }
                if (outputs.length === 0 && code !== 0) {
                    outputs.push(new vscode.NotebookCellOutput([
                        vscode.NotebookCellOutputItem.error({
                            name: 'ExecutionError',
                            message: `Process exited with code ${code ?? 1}`
                        })
                    ]));
                }
                await execution.replaceOutput(outputs);
                resolve({ success: code === 0 });
            });
        });
    }
    // ── Output parsing ───────────────────────────────────────────────────────────
    /** Extract numeric loss values from lines like "LOSS: 0.432" */
    _extractLoss(output) {
        const pattern = /^LOSS:\s*([\d.]+)/gim;
        const values = [];
        let m;
        while ((m = pattern.exec(output)) !== null) {
            const v = parseFloat(m[1]);
            if (!isNaN(v))
                values.push(v);
        }
        return values;
    }
    /** Extract everything between MODEL: ... END_MODEL */
    _extractModel(output) {
        const match = /MODEL:([\s\S]*?)END_MODEL/im.exec(output);
        return match ? match[1].trim() : undefined;
    }
    /** Remove LOSS:/MODEL: marker lines from plain text */
    _stripSpecialMarkers(output) {
        return output
            .replace(/^LOSS:\s*[\d.]+\s*$/gim, '')
            .replace(/^MODEL:[\s\S]*?END_MODEL\s*$/gim, '')
            .trim();
    }
    /** Render a simple ASCII bar chart of loss values */
    _renderLossChart(values) {
        if (values.length === 0)
            return '';
        const maxVal = Math.max(...values);
        const minVal = Math.min(...values);
        const range = maxVal - minVal || 1;
        const chartHeight = 8;
        const chartWidth = Math.min(values.length, 60);
        // Sample values to fit chartWidth
        const sampled = [];
        for (let i = 0; i < chartWidth; i++) {
            const idx = Math.floor((i / chartWidth) * values.length);
            sampled.push(values[idx]);
        }
        const rows = [];
        rows.push(`Training Loss (${values.length} steps, min=${minVal.toFixed(4)}, max=${maxVal.toFixed(4)})`);
        rows.push('─'.repeat(chartWidth + 8));
        for (let row = chartHeight; row >= 0; row--) {
            const threshold = minVal + (row / chartHeight) * range;
            const label = row % 2 === 0 ? (threshold).toFixed(3).padStart(7) : '       ';
            const line = sampled.map(v => (v >= threshold ? '█' : ' ')).join('');
            rows.push(`${label} │${line}`);
        }
        rows.push('        └' + '─'.repeat(chartWidth));
        rows.push(`         ${'0'.padEnd(Math.floor(chartWidth / 2))}${values.length - 1}`);
        return rows.join('\n');
    }
}
exports.SylvaNotebookKernel = SylvaNotebookKernel;
//# sourceMappingURL=SylvaNotebook.js.map