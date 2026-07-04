import * as vscode from 'vscode';
interface OmniPackage {
    name: string;
    version: string;
    dev: boolean;
}
type PackageItemKind = 'section' | 'package' | 'registryAction' | 'auditResult' | 'auditIssue';
export declare class PackageItem extends vscode.TreeItem {
    readonly pkgKind: PackageItemKind;
    readonly pkg?: OmniPackage | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, pkgKind: PackageItemKind, pkg?: OmniPackage | undefined, description?: string, tooltip?: string);
    private resolveIcon;
}
export declare class OmniPMExplorerProvider implements vscode.TreeDataProvider<PackageItem> {
    private readonly _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<void | PackageItem | undefined>;
    private cachedPackages;
    refresh(): void;
    getTreeItem(element: PackageItem): vscode.TreeItem;
    getChildren(element?: PackageItem): Promise<PackageItem[]>;
    private getRootItems;
    private getPackageItems;
    private getRegistryActions;
    private getAuditItems;
    private loadPackages;
    private findBuildFile;
    private runAudit;
}
export {};
//# sourceMappingURL=OmniPMExplorer.d.ts.map