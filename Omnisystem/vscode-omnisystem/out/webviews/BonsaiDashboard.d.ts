import * as vscode from 'vscode';
export declare class BonsaiDashboardPanel {
    static currentPanel: BonsaiDashboardPanel | undefined;
    static readonly viewType = "omnisystem.bonsaiDashboard";
    static postMessage(msg: object): void;
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    private _buildProcess;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    private _post;
    private _log;
    private _run;
    private _handleMessage;
    private _update;
    private _getHtmlForWebview;
    dispose(): void;
}
//# sourceMappingURL=BonsaiDashboard.d.ts.map