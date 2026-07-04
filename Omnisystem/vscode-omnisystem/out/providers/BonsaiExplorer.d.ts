import * as vscode from 'vscode';
type BonsaiItemKind = 'section' | 'subsection' | 'config' | 'command' | 'status' | 'subApp' | 'source' | 'titanSource';
export declare class BonsaiItem extends vscode.TreeItem {
    readonly bonsaiKind: BonsaiItemKind;
    readonly resourceUri?: vscode.Uri | undefined;
    readonly commandId?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, bonsaiKind: BonsaiItemKind, resourceUri?: vscode.Uri | undefined, commandId?: string | undefined, description?: string, tooltip?: string);
    private resolveIcon;
}
export declare class BonsaiExplorerProvider implements vscode.TreeDataProvider<BonsaiItem> {
    private ecosystemRoot;
    private readonly _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<void | BonsaiItem | undefined>;
    constructor(ecosystemRoot: vscode.Uri | undefined);
    refresh(newRoot?: vscode.Uri): void;
    getTreeItem(element: BonsaiItem): vscode.TreeItem;
    getChildren(element?: BonsaiItem): Promise<BonsaiItem[]>;
    private eco;
    private ecoPath;
    private exists;
    private found;
    private locCount;
    private titanItem;
    private cmdItem;
    private sectionItem;
    private getRootSections;
    /** Live reachability check against the orchestrator's health endpoint. */
    private harnessStatusLabel;
    private getHarnessChildren;
    private getControlPanelChildren;
    private getNotificationsChildren;
    private getSystemTrayChildren;
    private getInitChildren;
    private getWorkspaceChildren;
    private getBuddyChildren;
    private getBrowserExtChildren;
    private getLauncherChildren;
    private getRuntimeChildren;
    private getFileAssocChildren;
    private getThemeChildren;
    private getInstallerChildren;
    private getIntegrationChildren;
    private adbStatus;
}
export {};
//# sourceMappingURL=BonsaiExplorer.d.ts.map