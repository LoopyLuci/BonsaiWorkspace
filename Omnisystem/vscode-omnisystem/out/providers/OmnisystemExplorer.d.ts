import * as vscode from 'vscode';
type ItemKind = 'languageGroup' | 'file' | 'buildArtifacts' | 'artifact' | 'testGroup' | 'testFile';
export declare class OmnisystemItem extends vscode.TreeItem {
    readonly kind: ItemKind;
    readonly resourceUri?: vscode.Uri | undefined;
    readonly language?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, kind: ItemKind, resourceUri?: vscode.Uri | undefined, language?: string | undefined, description?: string);
    private resolveIcon;
}
export declare class OmnisystemExplorerProvider implements vscode.TreeDataProvider<OmnisystemItem> {
    private readonly _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<void | OmnisystemItem | undefined>;
    refresh(): void;
    getTreeItem(element: OmnisystemItem): vscode.TreeItem;
    getChildren(element?: OmnisystemItem): Promise<OmnisystemItem[]>;
    private getRootItems;
    private getLanguageFiles;
    private getBuildArtifacts;
    private getTestFiles;
    private findFiles;
    private workspaceBuildDir;
    private relativeDir;
    private countPattern;
}
export {};
//# sourceMappingURL=OmnisystemExplorer.d.ts.map