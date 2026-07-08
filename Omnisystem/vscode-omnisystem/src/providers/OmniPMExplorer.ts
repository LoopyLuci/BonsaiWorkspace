import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

// ─── Data structures ──────────────────────────────────────────────────────────

interface OmniPackage {
    name: string;
    version: string;
    dev: boolean;
}

interface AuditIssue {
    package: string;
    severity: 'critical' | 'high' | 'medium' | 'low';
    advisory: string;
}

type PackageItemKind =
    | 'section'
    | 'package'
    | 'registryAction'
    | 'auditResult'
    | 'auditIssue';

// ─── TreeItem ─────────────────────────────────────────────────────────────────

export class PackageItem extends vscode.TreeItem {
    constructor(
        label: string,
        collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly pkgKind: PackageItemKind,
        public readonly pkg?: OmniPackage,
        description?: string,
        tooltip?: string,
    ) {
        super(label, collapsibleState);
        this.description = description;
        if (tooltip) { this.tooltip = tooltip; }
        this.contextValue = `omnipm.${pkgKind}`;
        this.iconPath = this.resolveIcon(pkgKind, pkg);
    }

    private resolveIcon(
        kind: PackageItemKind,
        pkg?: OmniPackage,
    ): vscode.ThemeIcon {
        if (kind === 'section')         { return new vscode.ThemeIcon('package'); }
        if (kind === 'registryAction')  { return new vscode.ThemeIcon('search'); }
        if (kind === 'auditResult')     { return new vscode.ThemeIcon('shield'); }
        if (kind === 'auditIssue')      { return new vscode.ThemeIcon('warning'); }
        // package
        if (pkg?.dev) { return new vscode.ThemeIcon('tools'); }
        return new vscode.ThemeIcon('library');
    }
}

// ─── Provider ─────────────────────────────────────────────────────────────────

export class OmniPMExplorerProvider
    implements vscode.TreeDataProvider<PackageItem>
{
    private readonly _onDidChangeTreeData =
        new vscode.EventEmitter<PackageItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    private cachedPackages: OmniPackage[] | undefined;

    refresh(): void {
        this.cachedPackages = undefined;
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: PackageItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: PackageItem): Promise<PackageItem[]> {
        if (!element) {
            return this.getRootItems();
        }

        const label = typeof element.label === 'string' ? element.label : '';

        switch (label) {
            case 'Dependencies':     return this.getPackageItems(false);
            case 'Dev Dependencies': return this.getPackageItems(true);
            case 'Registry':         return this.getRegistryActions();
            case 'Audit':            return this.getAuditItems();
            default: return [];
        }
    }

    // ── Root ──────────────────────────────────────────────────────────────────

    private async getRootItems(): Promise<PackageItem[]> {
        const packages = await this.loadPackages();
        const deps    = packages.filter((p) => !p.dev);
        const devDeps = packages.filter((p) =>  p.dev);

        return [
            new PackageItem(
                'Dependencies',
                vscode.TreeItemCollapsibleState.Expanded,
                'section',
                undefined,
                `${deps.length} package${deps.length !== 1 ? 's' : ''}`,
            ),
            new PackageItem(
                'Dev Dependencies',
                vscode.TreeItemCollapsibleState.Collapsed,
                'section',
                undefined,
                `${devDeps.length} package${devDeps.length !== 1 ? 's' : ''}`,
            ),
            new PackageItem(
                'Registry',
                vscode.TreeItemCollapsibleState.Collapsed,
                'section',
                undefined,
                'search & browse',
            ),
            new PackageItem(
                'Audit',
                vscode.TreeItemCollapsibleState.Collapsed,
                'section',
                undefined,
                'security',
            ),
        ];
    }

    // ── Package items ─────────────────────────────────────────────────────────

    private async getPackageItems(dev: boolean): Promise<PackageItem[]> {
        const packages = await this.loadPackages();
        const filtered = packages.filter((p) => p.dev === dev);

        if (filtered.length === 0) {
            return [
                new PackageItem(
                    dev ? 'No dev dependencies' : 'No dependencies',
                    vscode.TreeItemCollapsibleState.None,
                    'package',
                    undefined,
                    '',
                ),
            ];
        }

        return filtered.map((pkg) => {
            const item = new PackageItem(
                pkg.name,
                vscode.TreeItemCollapsibleState.None,
                'package',
                pkg,
                pkg.version,
                `${pkg.name}@${pkg.version}`,
            );
            // Context menu: Update, Remove
            item.contextValue = 'omnipm.package';
            return item;
        });
    }

    // ── Registry actions ──────────────────────────────────────────────────────

    private getRegistryActions(): PackageItem[] {
        const searchItem = new PackageItem(
            'Search Registry',
            vscode.TreeItemCollapsibleState.None,
            'registryAction',
            undefined,
            'find packages',
        );
        searchItem.command = {
            command: 'omnisystem.omnipmSearch',
            title: 'Search OmniPM Registry',
        };

        const installItem = new PackageItem(
            'Install Package',
            vscode.TreeItemCollapsibleState.None,
            'registryAction',
            undefined,
            'omnicc pm install',
        );
        installItem.command = {
            command: 'omnisystem.omnipmInstall',
            title: 'Install OmniPM Package',
        };
        installItem.iconPath = new vscode.ThemeIcon('cloud-download');

        return [searchItem, installItem];
    }

    // ── Audit items ───────────────────────────────────────────────────────────

    private async getAuditItems(): Promise<PackageItem[]> {
        // In a real implementation, this would run `omnicc pm audit --json`
        const issues = await this.runAudit();

        if (issues.length === 0) {
            const ok = new PackageItem(
                '$(check) No vulnerabilities found',
                vscode.TreeItemCollapsibleState.None,
                'auditResult',
            );
            return [ok];
        }

        return issues.map((issue) => {
            const icons: Record<AuditIssue['severity'], string> = {
                critical: '$(error)',
                high:     '$(warning)',
                medium:   '$(info)',
                low:      '$(circle-outline)',
            };
            return new PackageItem(
                `${icons[issue.severity]} ${issue.package}`,
                vscode.TreeItemCollapsibleState.None,
                'auditIssue',
                undefined,
                issue.severity,
                issue.advisory,
            );
        });
    }

    // ── BUILD.omnisystem parser ───────────────────────────────────────────────

    private async loadPackages(): Promise<OmniPackage[]> {
        if (this.cachedPackages !== undefined) {
            return this.cachedPackages;
        }

        this.cachedPackages = [];

        const buildFile = this.findBuildFile();
        if (!buildFile) { return this.cachedPackages; }

        try {
            const content = fs.readFileSync(buildFile, 'utf8');
            this.cachedPackages = parseBuildOmnisystem(content);
        } catch {
            // ignore parse errors
        }

        return this.cachedPackages;
    }

    private findBuildFile(): string | undefined {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) { return undefined; }

        const roots = [folders[0].uri, vscode.Uri.joinPath(folders[0].uri, 'Omnisystem')];
        const candidates: string[] = [];
        for (const root of roots) {
            candidates.push(
                vscode.Uri.joinPath(root, 'BUILD.omnisystem').fsPath,
                vscode.Uri.joinPath(root, 'build.omnisystem').fsPath,
                vscode.Uri.joinPath(root, 'omnisystem.toml').fsPath,
            );
        }

        for (const c of candidates) {
            if (fs.existsSync(c)) { return c; }
        }
        return undefined;
    }

    private async runAudit(): Promise<AuditIssue[]> {
        // Stub — in a real extension this would shell out to `omnicc pm audit --json`
        return [];
    }
}

// ─── BUILD.omnisystem parser ──────────────────────────────────────────────────

function parseBuildOmnisystem(content: string): OmniPackage[] {
    const packages: OmniPackage[] = [];

    // Simple section-based TOML-like parser
    // Handles:
    //   [dependencies]
    //   omni-http = "1.2.0"
    //
    //   [dev-dependencies]
    //   omni-test = "0.3.1"

    let currentSection: 'dependencies' | 'dev-dependencies' | null = null;
    const lines = content.split('\n');

    for (const rawLine of lines) {
        const line = rawLine.trim();

        if (line === '' || line.startsWith('#')) { continue; }

        const sectionMatch = line.match(/^\[([^\]]+)\]$/);
        if (sectionMatch) {
            const sec = sectionMatch[1].toLowerCase();
            if (sec === 'dependencies' || sec === 'deps') {
                currentSection = 'dependencies';
            } else if (sec === 'dev-dependencies' || sec === 'dev-deps') {
                currentSection = 'dev-dependencies';
            } else {
                currentSection = null;
            }
            continue;
        }

        if (currentSection === null) { continue; }

        // name = "version" or name = { version = "1.0" }
        const kvMatch = line.match(/^([\w\-]+)\s*=\s*"([^"]+)"/);
        if (kvMatch) {
            packages.push({
                name: kvMatch[1],
                version: kvMatch[2],
                dev: currentSection === 'dev-dependencies',
            });
            continue;
        }

        // name = { version = "1.0", ... }
        const objMatch = line.match(/^([\w\-]+)\s*=\s*\{[^}]*version\s*=\s*"([^"]+)"/);
        if (objMatch) {
            packages.push({
                name: objMatch[1],
                version: objMatch[2],
                dev: currentSection === 'dev-dependencies',
            });
        }
    }

    return packages;
}
