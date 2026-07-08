import * as vscode from 'vscode';
export declare class WidgetGalleryPanel {
    static currentPanel: WidgetGalleryPanel | undefined;
    static readonly viewType = "omnisystem.widgetGallery";
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    private _post;
    static postMessage(msg: object): void;
    private _handleMessage;
    private _getWidgetCode;
    private _update;
    private _getHtml;
    dispose(): void;
}
//# sourceMappingURL=WidgetGallery.d.ts.map