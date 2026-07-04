import * as vscode from 'vscode';
type SystemItemKind = 'category' | 'systemFile' | 'tool';
export declare class SystemItem extends vscode.TreeItem {
    readonly systemKind: SystemItemKind;
    readonly resourceUri?: vscode.Uri | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, systemKind: SystemItemKind, resourceUri?: vscode.Uri | undefined, description?: string, tooltip?: string, iconName?: string);
}
export declare class OmniOSExplorerProvider implements vscode.TreeDataProvider<SystemItem> {
    private workspaceRoot;
    private readonly _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<void | SystemItem | undefined>;
    constructor(workspaceRoot: vscode.Uri | undefined);
    refresh(newRoot?: vscode.Uri): void;
    getTreeItem(element: SystemItem): vscode.TreeItem;
    getChildren(element?: SystemItem): Promise<SystemItem[]>;
    private getCategoryItems;
    private getSystemItems;
    private resolveSystemUri;
    private countFoundSystems;
    private countLines;
}
export {};
//# sourceMappingURL=OmniOSExplorer.d.ts.map