import * as vscode from 'vscode';
type DesktopItemKind = 'section' | 'subsection' | 'config' | 'command' | 'status' | 'subApp' | 'source' | 'titanSource';
export declare class DesktopItem extends vscode.TreeItem {
    readonly desktopKind: DesktopItemKind;
    readonly resourceUri?: vscode.Uri | undefined;
    readonly commandId?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, desktopKind: DesktopItemKind, resourceUri?: vscode.Uri | undefined, commandId?: string | undefined, description?: string, tooltip?: string);
    private resolveIcon;
}
export declare class DesktopExplorerProvider implements vscode.TreeDataProvider<DesktopItem> {
    private ecosystemRoot;
    private readonly _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<void | DesktopItem | undefined>;
    constructor(ecosystemRoot: vscode.Uri | undefined);
    refresh(newRoot?: vscode.Uri): void;
    getTreeItem(element: DesktopItem): vscode.TreeItem;
    getChildren(element?: DesktopItem): Promise<DesktopItem[]>;
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
//# sourceMappingURL=DesktopExplorer.d.ts.map