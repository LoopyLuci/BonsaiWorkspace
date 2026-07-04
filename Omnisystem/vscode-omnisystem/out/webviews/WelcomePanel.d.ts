import * as vscode from 'vscode';
export declare class WelcomePanel {
    static currentPanel: WelcomePanel | undefined;
    static readonly viewType = "omnisystem.welcome";
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    static postMessage(msg: object): void;
    private _handleMessage;
    private _update;
    private _getHtml;
    dispose(): void;
}
//# sourceMappingURL=WelcomePanel.d.ts.map