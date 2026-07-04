import * as vscode from 'vscode';
export declare class MlStudioPanel {
    static currentPanel: MlStudioPanel | undefined;
    static readonly viewType = "omnisystem.mlStudio";
    static postMessage(msg: object): void;
    private readonly _panel;
    private readonly _extensionUri;
    private _disposables;
    private _trainProcess;
    private _trainingInterval;
    static createOrShow(extensionUri: vscode.Uri): void;
    private constructor();
    private _post;
    private _stopTraining;
    private _handleMessage;
    private _fakeConfusionMatrix;
    private _update;
    private _getHtmlForWebview;
    dispose(): void;
}
//# sourceMappingURL=MlStudio.d.ts.map