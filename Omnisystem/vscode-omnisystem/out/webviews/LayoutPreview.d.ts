import * as vscode from 'vscode';
export declare class LayoutPreviewPanel {
    static currentPanel: LayoutPreviewPanel | undefined;
    static readonly viewType = "omnisystem.layoutPreview";
    static postMessage(msg: object): void;
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    private _post;
    private _loadActiveLayout;
    private _analyzeLayout;
    private _handleMessage;
    private _update;
    private _getHtmlForWebview;
    dispose(): void;
}
//# sourceMappingURL=LayoutPreview.d.ts.map