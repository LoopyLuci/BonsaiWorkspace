import * as vscode from 'vscode';
export declare class OmniCCDashboardPanel {
    static readonly viewType = "omnisystem.omniCC";
    static currentPanel: OmniCCDashboardPanel | undefined;
    private readonly _panel;
    private readonly _engine;
    private _disposables;
    private _history;
    private _projectFolder;
    static createOrShow(extensionUri: vscode.Uri): void;
    static postMessage(msg: unknown): void;
    private constructor();
    private _handleMessage;
    private _post;
    private _scanFolder;
    private _addToHistory;
    dispose(): void;
    private _buildHtml;
}
//# sourceMappingURL=OmniCCDashboard.d.ts.map