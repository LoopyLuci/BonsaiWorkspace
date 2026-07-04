"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.OmnisystemExplorerProvider = exports.OmnisystemItem = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
class OmnisystemItem extends vscode.TreeItem {
    constructor(label, collapsibleState, kind, resourceUri, language, description) {
        super(label, collapsibleState);
        this.kind = kind;
        this.resourceUri = resourceUri;
        this.language = language;
        this.description = description;
        if (kind === 'file' || kind === 'testFile' || kind === 'artifact') {
            this.resourceUri = resourceUri;
            this.command = {
                command: 'omnisystem.openFile',
                title: 'Open File',
                arguments: [resourceUri],
            };
            this.contextValue = `omnisystem.${kind}`;
        }
        else {
            this.contextValue = `omnisystem.${kind}`;
        }
        this.iconPath = this.resolveIcon(kind, language);
    }
    resolveIcon(kind, language) {
        if (kind === 'languageGroup') {
            const icons = {
                titan: 'symbol-module',
                vera: 'symbol-interface',
                helix: 'symbol-color',
                aether: 'symbol-event',
                axiom: 'symbol-boolean',
                sylva: 'symbol-misc',
                nexus: 'layout',
            };
            return new vscode.ThemeIcon(icons[language ?? ''] ?? 'folder');
        }
        if (kind === 'buildArtifacts') {
            return new vscode.ThemeIcon('package');
        }
        if (kind === 'testGroup') {
            return new vscode.ThemeIcon('beaker');
        }
        if (kind === 'testFile') {
            return new vscode.ThemeIcon('beaker');
        }
        if (kind === 'artifact') {
            return new vscode.ThemeIcon('file-binary');
        }
        // 'file' — use resource URI for language icon from VS Code
        return undefined;
    }
}
exports.OmnisystemItem = OmnisystemItem;
const LANGUAGE_GROUPS = [
    { id: 'titan', label: 'TITAN', extension: 'titan' },
    { id: 'vera', label: 'VERA', extension: 'vera' },
    { id: 'helix', label: 'HELIX', extension: 'helix' },
    { id: 'aether', label: 'AETHER', extension: 'aether' },
    { id: 'axiom', label: 'AXIOM', extension: 'axiom', badgeKey: 'theorem' },
    { id: 'sylva', label: 'SYLVA', extension: 'sylva', badgeKey: 'model' },
    { id: 'nexus', label: 'NEXUS', extension: 'nexus' },
];
// ─── Provider ─────────────────────────────────────────────────────────────────
class OmnisystemExplorerProvider {
    constructor() {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (!element) {
            return this.getRootItems();
        }
        switch (element.kind) {
            case 'languageGroup':
                return this.getLanguageFiles(element.language);
            case 'buildArtifacts':
                return this.getBuildArtifacts();
            case 'testGroup':
                return this.getTestFiles();
            default:
                return [];
        }
    }
    async getRootItems() {
        const items = [];
        // Launch button — always first
        const launch = new OmnisystemItem('Launch OmniOS Desktop', vscode.TreeItemCollapsibleState.None, 'file');
        launch.description = 'Open interactive desktop';
        launch.iconPath = new vscode.ThemeIcon('rocket');
        launch.command = { command: 'omnisystem.omniOsBoot', title: 'Launch OmniOS Desktop' };
        items.push(launch);
        for (const group of LANGUAGE_GROUPS) {
            const files = await this.findFiles(`**/*.${group.extension}`);
            if (files.length === 0) {
                continue;
            }
            let description = `${files.length} file${files.length !== 1 ? 's' : ''}`;
            if (group.badgeKey === 'theorem') {
                const count = await this.countPattern(files, /\btheorem\b/g);
                description += ` · ${count} theorem${count !== 1 ? 's' : ''}`;
            }
            else if (group.badgeKey === 'model') {
                const count = await this.countPattern(files, /\bmodel\b/g);
                description += ` · ${count} model${count !== 1 ? 's' : ''}`;
            }
            items.push(new OmnisystemItem(group.label, vscode.TreeItemCollapsibleState.Collapsed, 'languageGroup', undefined, group.id, description));
        }
        // Build Artifacts
        const buildDir = this.workspaceBuildDir();
        if (buildDir && fs.existsSync(buildDir)) {
            items.push(new OmnisystemItem('Build Artifacts', vscode.TreeItemCollapsibleState.Collapsed, 'buildArtifacts'));
        }
        // Tests
        const testFiles = await this.findFiles('**/*{test,spec}*.{titan,vera,helix,aether,axiom,sylva,nexus}');
        if (testFiles.length > 0) {
            items.push(new OmnisystemItem('Tests', vscode.TreeItemCollapsibleState.Collapsed, 'testGroup', undefined, undefined, `${testFiles.length} file${testFiles.length !== 1 ? 's' : ''}`));
        }
        return items;
    }
    async getLanguageFiles(language) {
        const ext = LANGUAGE_GROUPS.find((g) => g.id === language)?.extension ?? language;
        const files = await this.findFiles(`**/*.${ext}`);
        return files
            .sort((a, b) => a.fsPath.localeCompare(b.fsPath))
            .map((uri) => {
            const filename = path.basename(uri.fsPath);
            const relDir = this.relativeDir(uri);
            return new OmnisystemItem(filename, vscode.TreeItemCollapsibleState.None, 'file', uri, language, relDir || undefined);
        });
    }
    async getBuildArtifacts() {
        const buildDir = this.workspaceBuildDir();
        if (!buildDir || !fs.existsSync(buildDir)) {
            return [];
        }
        try {
            const entries = fs.readdirSync(buildDir, { withFileTypes: true });
            return entries
                .filter((e) => e.isFile())
                .map((e) => {
                const uri = vscode.Uri.file(path.join(buildDir, e.name));
                return new OmnisystemItem(e.name, vscode.TreeItemCollapsibleState.None, 'artifact', uri);
            });
        }
        catch {
            return [];
        }
    }
    async getTestFiles() {
        const files = await this.findFiles('**/*{test,spec}*.{titan,vera,helix,aether,axiom,sylva,nexus}');
        return files
            .sort((a, b) => a.fsPath.localeCompare(b.fsPath))
            .map((uri) => {
            const filename = path.basename(uri.fsPath);
            return new OmnisystemItem(filename, vscode.TreeItemCollapsibleState.None, 'testFile', uri);
        });
    }
    // ── helpers ───────────────────────────────────────────────────────────────
    async findFiles(pattern) {
        try {
            return await vscode.workspace.findFiles(pattern, '**/node_modules/**');
        }
        catch {
            return [];
        }
    }
    workspaceBuildDir() {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) {
            return undefined;
        }
        return vscode.Uri.joinPath(folders[0].uri, 'build').fsPath;
    }
    relativeDir(uri) {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) {
            return '';
        }
        const root = folders[0].uri.fsPath;
        const rel = path.relative(root, path.dirname(uri.fsPath));
        return rel === '.' ? '' : rel;
    }
    async countPattern(files, pattern) {
        let count = 0;
        for (const uri of files) {
            try {
                const content = fs.readFileSync(uri.fsPath, 'utf8');
                const matches = content.match(pattern);
                count += matches?.length ?? 0;
            }
            catch {
                // ignore unreadable files
            }
        }
        return count;
    }
}
exports.OmnisystemExplorerProvider = OmnisystemExplorerProvider;
//# sourceMappingURL=OmnisystemExplorer.js.map