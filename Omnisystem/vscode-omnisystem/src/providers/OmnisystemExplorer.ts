import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

// ─── Item types ───────────────────────────────────────────────────────────────

type ItemKind =
    | 'languageGroup'
    | 'file'
    | 'buildArtifacts'
    | 'artifact'
    | 'testGroup'
    | 'testFile';

export class OmnisystemItem extends vscode.TreeItem {
    constructor(
        label: string,
        collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly kind: ItemKind,
        public readonly resourceUri?: vscode.Uri,
        public readonly language?: string,
        description?: string,
    ) {
        super(label, collapsibleState);
        this.description = description;

        if (kind === 'file' || kind === 'testFile' || kind === 'artifact') {
            this.resourceUri = resourceUri;
            this.command = {
                command: 'omnisystem.openFile',
                title: 'Open File',
                arguments: [resourceUri],
            };
            this.contextValue = `omnisystem.${kind}`;
        } else {
            this.contextValue = `omnisystem.${kind}`;
        }

        this.iconPath = this.resolveIcon(kind, language);
    }

    private resolveIcon(
        kind: ItemKind,
        language?: string,
    ): vscode.ThemeIcon | undefined {
        if (kind === 'languageGroup') {
            const icons: Record<string, string> = {
                titan: 'symbol-module',
                vera:  'symbol-interface',
                helix: 'symbol-color',
                aether:'symbol-event',
                axiom: 'symbol-boolean',
                sylva: 'symbol-misc',
                nexus: 'layout',
            };
            return new vscode.ThemeIcon(icons[language ?? ''] ?? 'folder');
        }
        if (kind === 'buildArtifacts') { return new vscode.ThemeIcon('package'); }
        if (kind === 'testGroup')      { return new vscode.ThemeIcon('beaker'); }
        if (kind === 'testFile')       { return new vscode.ThemeIcon('beaker'); }
        if (kind === 'artifact')       { return new vscode.ThemeIcon('file-binary'); }
        // 'file' — use resource URI for language icon from VS Code
        return undefined;
    }
}

// ─── Language group descriptor ────────────────────────────────────────────────

interface LanguageGroup {
    id: string;
    label: string;
    extension: string;
    badgeKey?: 'theorem' | 'model';
}

const LANGUAGE_GROUPS: LanguageGroup[] = [
    { id: 'titan',  label: 'TITAN',  extension: 'titan' },
    { id: 'vera',   label: 'VERA',   extension: 'vera'  },
    { id: 'helix',  label: 'HELIX',  extension: 'helix' },
    { id: 'aether', label: 'AETHER', extension: 'aether' },
    { id: 'axiom',  label: 'AXIOM',  extension: 'axiom', badgeKey: 'theorem' },
    { id: 'sylva',  label: 'SYLVA',  extension: 'sylva', badgeKey: 'model'   },
    { id: 'nexus',  label: 'NEXUS',  extension: 'nexus' },
];

// ─── Provider ─────────────────────────────────────────────────────────────────

export class OmnisystemExplorerProvider
    implements vscode.TreeDataProvider<OmnisystemItem>
{
    private readonly _onDidChangeTreeData =
        new vscode.EventEmitter<OmnisystemItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: OmnisystemItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: OmnisystemItem): Promise<OmnisystemItem[]> {
        if (!element) {
            return this.getRootItems();
        }

        switch (element.kind) {
            case 'languageGroup':
                return this.getLanguageFiles(element.language!);
            case 'buildArtifacts':
                return this.getBuildArtifacts();
            case 'testGroup':
                return this.getTestFiles();
            default:
                return [];
        }
    }

    private async getRootItems(): Promise<OmnisystemItem[]> {
        const items: OmnisystemItem[] = [];

        // Launch button — always first
        const launch = new OmnisystemItem('Launch OmniOS Desktop', vscode.TreeItemCollapsibleState.None, 'file');
        launch.description = 'Open interactive desktop';
        launch.iconPath = new vscode.ThemeIcon('rocket');
        launch.command = { command: 'omnisystem.omniOsBoot', title: 'Launch OmniOS Desktop' };
        items.push(launch);

        for (const group of LANGUAGE_GROUPS) {
            const files = await this.findFiles(`**/*.${group.extension}`);
            if (files.length === 0) { continue; }

            let description = `${files.length} file${files.length !== 1 ? 's' : ''}`;

            if (group.badgeKey === 'theorem') {
                const count = await this.countPattern(files, /\btheorem\b/g);
                description += ` · ${count} theorem${count !== 1 ? 's' : ''}`;
            } else if (group.badgeKey === 'model') {
                const count = await this.countPattern(files, /\bmodel\b/g);
                description += ` · ${count} model${count !== 1 ? 's' : ''}`;
            }

            items.push(
                new OmnisystemItem(
                    group.label,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'languageGroup',
                    undefined,
                    group.id,
                    description,
                )
            );
        }

        // Build Artifacts
        const buildDir = this.workspaceBuildDir();
        if (buildDir && fs.existsSync(buildDir)) {
            items.push(
                new OmnisystemItem(
                    'Build Artifacts',
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'buildArtifacts',
                )
            );
        }

        // Tests
        const testFiles = await this.findFiles('**/*{test,spec}*.{titan,vera,helix,aether,axiom,sylva,nexus}');
        if (testFiles.length > 0) {
            items.push(
                new OmnisystemItem(
                    'Tests',
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'testGroup',
                    undefined,
                    undefined,
                    `${testFiles.length} file${testFiles.length !== 1 ? 's' : ''}`,
                )
            );
        }

        return items;
    }

    private async getLanguageFiles(language: string): Promise<OmnisystemItem[]> {
        const ext = LANGUAGE_GROUPS.find((g) => g.id === language)?.extension ?? language;
        const files = await this.findFiles(`**/*.${ext}`);

        return files
            .sort((a, b) => a.fsPath.localeCompare(b.fsPath))
            .map((uri) => {
                const filename = path.basename(uri.fsPath);
                const relDir = this.relativeDir(uri);
                return new OmnisystemItem(
                    filename,
                    vscode.TreeItemCollapsibleState.None,
                    'file',
                    uri,
                    language,
                    relDir || undefined,
                );
            });
    }

    private async getBuildArtifacts(): Promise<OmnisystemItem[]> {
        const buildDir = this.workspaceBuildDir();
        if (!buildDir || !fs.existsSync(buildDir)) { return []; }

        try {
            const entries = fs.readdirSync(buildDir, { withFileTypes: true });
            return entries
                .filter((e) => e.isFile())
                .map((e) => {
                    const uri = vscode.Uri.file(path.join(buildDir, e.name));
                    return new OmnisystemItem(
                        e.name,
                        vscode.TreeItemCollapsibleState.None,
                        'artifact',
                        uri,
                    );
                });
        } catch {
            return [];
        }
    }

    private async getTestFiles(): Promise<OmnisystemItem[]> {
        const files = await this.findFiles(
            '**/*{test,spec}*.{titan,vera,helix,aether,axiom,sylva,nexus}'
        );
        return files
            .sort((a, b) => a.fsPath.localeCompare(b.fsPath))
            .map((uri) => {
                const filename = path.basename(uri.fsPath);
                return new OmnisystemItem(
                    filename,
                    vscode.TreeItemCollapsibleState.None,
                    'testFile',
                    uri,
                );
            });
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    private async findFiles(pattern: string): Promise<vscode.Uri[]> {
        try {
            return await vscode.workspace.findFiles(pattern, '**/node_modules/**');
        } catch {
            return [];
        }
    }

    private workspaceBuildDir(): string | undefined {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) { return undefined; }
        return vscode.Uri.joinPath(folders[0].uri, 'build').fsPath;
    }

    private relativeDir(uri: vscode.Uri): string {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) { return ''; }
        const root = folders[0].uri.fsPath;
        const rel = path.relative(root, path.dirname(uri.fsPath));
        return rel === '.' ? '' : rel;
    }

    private async countPattern(files: vscode.Uri[], pattern: RegExp): Promise<number> {
        let count = 0;
        for (const uri of files) {
            try {
                const content = fs.readFileSync(uri.fsPath, 'utf8');
                const matches = content.match(pattern);
                count += matches?.length ?? 0;
            } catch {
                // ignore unreadable files
            }
        }
        return count;
    }
}
