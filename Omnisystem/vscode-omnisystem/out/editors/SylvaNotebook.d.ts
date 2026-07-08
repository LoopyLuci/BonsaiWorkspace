import * as vscode from 'vscode';
export declare class SylvaNotebookSerializer implements vscode.NotebookSerializer {
    deserializeNotebook(content: Uint8Array): Promise<vscode.NotebookData>;
    serializeNotebook(data: vscode.NotebookData): Promise<Uint8Array>;
}
export declare class SylvaNotebookKernel {
    readonly id = "sylva-kernel";
    readonly label = "SYLVA ML Kernel";
    readonly supportedLanguages: string[];
    private _executionOrder;
    private readonly _controller;
    constructor();
    dispose(): void;
    private _execute;
    private _executeCell;
    private _runSylva;
    /** Extract numeric loss values from lines like "LOSS: 0.432" */
    private _extractLoss;
    /** Extract everything between MODEL: ... END_MODEL */
    private _extractModel;
    /** Remove LOSS:/MODEL: marker lines from plain text */
    private _stripSpecialMarkers;
    /** Render a simple ASCII bar chart of loss values */
    private _renderLossChart;
}
//# sourceMappingURL=SylvaNotebook.d.ts.map