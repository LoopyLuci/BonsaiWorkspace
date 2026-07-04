import * as vscode from 'vscode';
declare global {
    interface Window {
        post: (cmd: string, extra?: Record<string, unknown>) => void;
        notify: (title: string, msg: string, icon?: string) => void;
        openApp: (appId: string) => void;
    }
}
export declare class OmniOSDesktopPanel {
    static currentPanel: OmniOSDesktopPanel | undefined;
    static readonly viewType = "omnisystem.omniOsDesktop";
    static _extensionContext: vscode.ExtensionContext | undefined;
    static onThemeChange: ((themeId: string) => void) | undefined;
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    private _activeProcs;
    private _lastProc;
    private _runtime;
    private _pty;
    private _lastDiagCount;
    static createOrShow(extensionUri: vscode.Uri, ctx?: vscode.ExtensionContext): void;
    static postMessage(data: unknown): void;
    private constructor();
    private _update;
    private _handleMessage;
    dispose(): void;
    private _getHtml;
}
//# sourceMappingURL=OmniOSDesktop.d.ts.map