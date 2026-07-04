import * as vscode from 'vscode';
import { spawn } from 'child_process';

// ── Types ────────────────────────────────────────────────────────────────────

interface SerializedCell {
  kind: 'code' | 'markdown';
  value: string;
  language: string;
  outputs: SerializedOutput[];
  metadata?: Record<string, unknown>;
}

interface SerializedOutput {
  mime: string;
  data: string;   // base64 for binary, plain text otherwise
}

interface SerializedNotebook {
  version: number;
  cells: SerializedCell[];
}

// ── Serializer ───────────────────────────────────────────────────────────────

export class SylvaNotebookSerializer implements vscode.NotebookSerializer {

  async deserializeNotebook(content: Uint8Array): Promise<vscode.NotebookData> {
    let nb: SerializedNotebook = { version: 1, cells: [] };

    try {
      nb = JSON.parse(Buffer.from(content).toString('utf-8')) as SerializedNotebook;
    } catch {
      // Return empty notebook on parse error
    }

    const cells = nb.cells.map(raw => {
      const kind =
        raw.kind === 'markdown'
          ? vscode.NotebookCellKind.Markup
          : vscode.NotebookCellKind.Code;

      const outputs: vscode.NotebookCellOutput[] = raw.outputs
        .filter(o => o.mime && o.data)
        .map(o =>
          new vscode.NotebookCellOutput([
            vscode.NotebookCellOutputItem.text(o.data, o.mime)
          ])
        );

      const cell = new vscode.NotebookCellData(kind, raw.value, raw.language ?? 'sylva');
      cell.outputs = outputs;
      cell.metadata = raw.metadata ?? {};
      return cell;
    });

    const data = new vscode.NotebookData(cells);
    data.metadata = { version: nb.version };
    return data;
  }

  async serializeNotebook(data: vscode.NotebookData): Promise<Uint8Array> {
    const cells: SerializedCell[] = data.cells.map(cell => {
      const outputs: SerializedOutput[] = (cell.outputs ?? []).flatMap(out =>
        out.items.map(item => ({
          mime: item.mime,
          data: Buffer.from(item.data).toString('utf-8')
        }))
      );

      return {
        kind: cell.kind === vscode.NotebookCellKind.Markup ? 'markdown' : 'code',
        value: cell.value,
        language: cell.languageId ?? 'sylva',
        outputs,
        metadata: cell.metadata
      };
    });

    const nb: SerializedNotebook = {
      version: (data.metadata?.['version'] as number | undefined) ?? 1,
      cells
    };

    return Buffer.from(JSON.stringify(nb, null, 2), 'utf-8');
  }
}

// ── Kernel ───────────────────────────────────────────────────────────────────

export class SylvaNotebookKernel {
  readonly id = 'sylva-kernel';
  readonly label = 'SYLVA ML Kernel';
  readonly supportedLanguages = ['sylva'];

  private _executionOrder = 0;
  private readonly _controller: vscode.NotebookController;

  constructor() {
    this._controller = vscode.notebooks.createNotebookController(
      this.id,
      'sylva-notebook',
      this.label
    );
    this._controller.supportedLanguages = this.supportedLanguages;
    this._controller.supportsExecutionOrder = true;
    this._controller.description = 'Execute SYLVA ML code via omnicc';
    this._controller.executeHandler = this._execute.bind(this);
  }

  dispose(): void {
    this._controller.dispose();
  }

  // ── Execution ────────────────────────────────────────────────────────────────

  private async _execute(
    cells: vscode.NotebookCell[],
    _notebook: vscode.NotebookDocument,
    _controller: vscode.NotebookController
  ): Promise<void> {
    for (const cell of cells) {
      await this._executeCell(cell);
    }
  }

  private async _executeCell(cell: vscode.NotebookCell): Promise<void> {
    const execution = this._controller.createNotebookCellExecution(cell);
    execution.executionOrder = ++this._executionOrder;
    execution.start(Date.now());
    execution.clearOutput();

    const source = cell.document.getText();

    try {
      const result = await this._runSylva(source, execution);
      execution.end(result.success, Date.now());
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      await execution.replaceOutput([
        new vscode.NotebookCellOutput([
          vscode.NotebookCellOutputItem.error({ name: 'Error', message: msg })
        ])
      ]);
      execution.end(false, Date.now());
    }
  }

  private async _runSylva(
    source: string,
    execution: vscode.NotebookCellExecution
  ): Promise<{ success: boolean }> {
    return new Promise(resolve => {
      const omniccPath = vscode.workspace
        .getConfiguration('omnisystem')
        .get<string>('omniccPath', 'omnicc');

      const proc = spawn(omniccPath, ['eval', '--language', 'sylva'], {
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true
      });

      const stdoutChunks: string[] = [];
      const stderrChunks: string[] = [];

      proc.stdout!.on('data', (chunk: Buffer) => stdoutChunks.push(chunk.toString()));
      proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk.toString()));

      proc.stdin!.write(source);
      proc.stdin!.end();

      proc.on('error', (err: Error) => {
        void execution.replaceOutput([
          new vscode.NotebookCellOutput([
            vscode.NotebookCellOutputItem.error({ name: 'SpawnError', message: err.message })
          ])
        ]);
        resolve({ success: false });
      });

      proc.on('close', async (code: number | null) => {
        const stdout = stdoutChunks.join('');
        const stderr = stderrChunks.join('');
        const outputs: vscode.NotebookCellOutput[] = [];

        if (stderr) {
          outputs.push(
            new vscode.NotebookCellOutput([
              vscode.NotebookCellOutputItem.stderr(stderr)
            ])
          );
        }

        if (stdout) {
          // Detect special markers and render accordingly
          const lossLines = this._extractLoss(stdout);
          const modelSummary = this._extractModel(stdout);
          const plainText = this._stripSpecialMarkers(stdout);

          if (lossLines.length > 0) {
            const chart = this._renderLossChart(lossLines);
            outputs.push(
              new vscode.NotebookCellOutput([
                vscode.NotebookCellOutputItem.text(chart, 'text/plain')
              ])
            );
          }

          if (modelSummary) {
            outputs.push(
              new vscode.NotebookCellOutput([
                vscode.NotebookCellOutputItem.text(modelSummary, 'text/plain')
              ])
            );
          }

          if (plainText.trim()) {
            outputs.push(
              new vscode.NotebookCellOutput([
                vscode.NotebookCellOutputItem.stdout(plainText)
              ])
            );
          }
        }

        if (outputs.length === 0 && code !== 0) {
          outputs.push(
            new vscode.NotebookCellOutput([
              vscode.NotebookCellOutputItem.error({
                name: 'ExecutionError',
                message: `Process exited with code ${code ?? 1}`
              })
            ])
          );
        }

        await execution.replaceOutput(outputs);
        resolve({ success: code === 0 });
      });
    });
  }

  // ── Output parsing ───────────────────────────────────────────────────────────

  /** Extract numeric loss values from lines like "LOSS: 0.432" */
  private _extractLoss(output: string): number[] {
    const pattern = /^LOSS:\s*([\d.]+)/gim;
    const values: number[] = [];
    let m: RegExpExecArray | null;
    while ((m = pattern.exec(output)) !== null) {
      const v = parseFloat(m[1]);
      if (!isNaN(v)) values.push(v);
    }
    return values;
  }

  /** Extract everything between MODEL: ... END_MODEL */
  private _extractModel(output: string): string | undefined {
    const match = /MODEL:([\s\S]*?)END_MODEL/im.exec(output);
    return match ? match[1].trim() : undefined;
  }

  /** Remove LOSS:/MODEL: marker lines from plain text */
  private _stripSpecialMarkers(output: string): string {
    return output
      .replace(/^LOSS:\s*[\d.]+\s*$/gim, '')
      .replace(/^MODEL:[\s\S]*?END_MODEL\s*$/gim, '')
      .trim();
  }

  /** Render a simple ASCII bar chart of loss values */
  private _renderLossChart(values: number[]): string {
    if (values.length === 0) return '';

    const maxVal = Math.max(...values);
    const minVal = Math.min(...values);
    const range = maxVal - minVal || 1;
    const chartHeight = 8;
    const chartWidth = Math.min(values.length, 60);

    // Sample values to fit chartWidth
    const sampled: number[] = [];
    for (let i = 0; i < chartWidth; i++) {
      const idx = Math.floor((i / chartWidth) * values.length);
      sampled.push(values[idx]);
    }

    const rows: string[] = [];
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
