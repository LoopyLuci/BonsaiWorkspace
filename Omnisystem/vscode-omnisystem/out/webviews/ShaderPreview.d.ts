import * as vscode from 'vscode';
export declare class ShaderPreviewPanel {
    static currentPanel: ShaderPreviewPanel | undefined;
    static readonly viewType = "omnisystem.shaderPreview";
    static postMessage(msg: object): void;
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    private _editorChangeDisposable;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    private _post;
    private _loadActiveShader;
    private _analyzeShader;
    private _guessUniformDesc;
    private _handleMessage;
    private _update;
    private _getHtmlForWebview;
    dispose(): void;
}
//# sourceMappingURL=ShaderPreview.d.ts.map