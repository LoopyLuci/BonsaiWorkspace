import * as vscode from 'vscode';
export declare class OmnisystemDashboardPanel {
    static currentPanel: OmnisystemDashboardPanel | undefined;
    static readonly viewType = "omnisystem.desktopDashboard";
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
//# sourceMappingURL=OmnisystemDashboard.d.ts.map