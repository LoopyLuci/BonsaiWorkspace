import * as vscode from 'vscode';
export declare class BuildEditorProvider implements vscode.CustomTextEditorProvider {
    private readonly _context;
    static readonly viewType = "omnisystem.buildEditor";
    static register(context: vscode.ExtensionContext): vscode.Disposable;
    constructor(_context: vscode.ExtensionContext);
    resolveCustomTextEditor(document: vscode.TextDocument, webviewPanel: vscode.WebviewPanel, _token: vscode.CancellationToken): Promise<void>;
    private _parse;
    private _parseList;
    private _serialize;
    private _applyEdits;
    private _buildHtml;
}
//# sourceMappingURL=BuildEditor.d.ts.map