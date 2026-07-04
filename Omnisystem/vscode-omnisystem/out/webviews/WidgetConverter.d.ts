import * as vscode from 'vscode';
export declare class WidgetConverterPanel {
    static currentPanel: WidgetConverterPanel | undefined;
    static readonly viewType = "omnisystem.widgetConverter";
    static postMessage(msg: object): void;
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    private _handleMessage;
    private _update;
    private _getHtml;
    dispose(): void;
}
//# sourceMappingURL=WidgetConverter.d.ts.map