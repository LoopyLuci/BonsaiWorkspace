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
exports.OmniPMExplorerProvider = exports.PackageItem = void 0;
const vscode = __importStar(require("vscode"));
const fs = __importStar(require("fs"));
// ─── TreeItem ─────────────────────────────────────────────────────────────────
class PackageItem extends vscode.TreeItem {
    constructor(label, collapsibleState, pkgKind, pkg, description, tooltip) {
        super(label, collapsibleState);
        this.pkgKind = pkgKind;
        this.pkg = pkg;
        this.description = description;
        if (tooltip) {
            this.tooltip = tooltip;
        }
        this.contextValue = `omnipm.${pkgKind}`;
        this.iconPath = this.resolveIcon(pkgKind, pkg);
    }
    resolveIcon(kind, pkg) {
        if (kind === 'section') {
            return new vscode.ThemeIcon('package');
        }
        if (kind === 'registryAction') {
            return new vscode.ThemeIcon('search');
        }
        if (kind === 'auditResult') {
            return new vscode.ThemeIcon('shield');
        }
        if (kind === 'auditIssue') {
            return new vscode.ThemeIcon('warning');
        }
        // package
        if (pkg?.dev) {
            return new vscode.ThemeIcon('tools');
        }
        return new vscode.ThemeIcon('library');
    }
}
exports.PackageItem = PackageItem;
// ─── Provider ─────────────────────────────────────────────────────────────────
class OmniPMExplorerProvider {
    constructor() {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh() {
        this.cachedPackages = undefined;
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (!element) {
            return this.getRootItems();
        }
        const label = typeof element.label === 'string' ? element.label : '';
        switch (label) {
            case 'Dependencies': return this.getPackageItems(false);
            case 'Dev Dependencies': return this.getPackageItems(true);
            case 'Registry': return this.getRegistryActions();
            case 'Audit': return this.getAuditItems();
            default: return [];
        }
    }
    // ── Root ──────────────────────────────────────────────────────────────────
    async getRootItems() {
        const packages = await this.loadPackages();
        const deps = packages.filter((p) => !p.dev);
        const devDeps = packages.filter((p) => p.dev);
        return [
            new PackageItem('Dependencies', vscode.TreeItemCollapsibleState.Expanded, 'section', undefined, `${deps.length} package${deps.length !== 1 ? 's' : ''}`),
            new PackageItem('Dev Dependencies', vscode.TreeItemCollapsibleState.Collapsed, 'section', undefined, `${devDeps.length} package${devDeps.length !== 1 ? 's' : ''}`),
            new PackageItem('Registry', vscode.TreeItemCollapsibleState.Collapsed, 'section', undefined, 'search & browse'),
            new PackageItem('Audit', vscode.TreeItemCollapsibleState.Collapsed, 'section', undefined, 'security'),
        ];
    }
    // ── Package items ─────────────────────────────────────────────────────────
    async getPackageItems(dev) {
        const packages = await this.loadPackages();
        const filtered = packages.filter((p) => p.dev === dev);
        if (filtered.length === 0) {
            return [
                new PackageItem(dev ? 'No dev dependencies' : 'No dependencies', vscode.TreeItemCollapsibleState.None, 'package', undefined, ''),
            ];
        }
        return filtered.map((pkg) => {
            const item = new PackageItem(pkg.name, vscode.TreeItemCollapsibleState.None, 'package', pkg, pkg.version, `${pkg.name}@${pkg.version}`);
            // Context menu: Update, Remove
            item.contextValue = 'omnipm.package';
            return item;
        });
    }
    // ── Registry actions ──────────────────────────────────────────────────────
    getRegistryActions() {
        const searchItem = new PackageItem('Search Registry', vscode.TreeItemCollapsibleState.None, 'registryAction', undefined, 'find packages');
        searchItem.command = {
            command: 'omnisystem.omnipmSearch',
            title: 'Search OmniPM Registry',
        };
        const installItem = new PackageItem('Install Package', vscode.TreeItemCollapsibleState.None, 'registryAction', undefined, 'omnicc pm install');
        installItem.command = {
            command: 'omnisystem.omnipmInstall',
            title: 'Install OmniPM Package',
        };
        installItem.iconPath = new vscode.ThemeIcon('cloud-download');
        return [searchItem, installItem];
    }
    // ── Audit items ───────────────────────────────────────────────────────────
    async getAuditItems() {
        // In a real implementation, this would run `omnicc pm audit --json`
        const issues = await this.runAudit();
        if (issues.length === 0) {
            const ok = new PackageItem('$(check) No vulnerabilities found', vscode.TreeItemCollapsibleState.None, 'auditResult');
            return [ok];
        }
        return issues.map((issue) => {
            const icons = {
                critical: '$(error)',
                high: '$(warning)',
                medium: '$(info)',
                low: '$(circle-outline)',
            };
            return new PackageItem(`${icons[issue.severity]} ${issue.package}`, vscode.TreeItemCollapsibleState.None, 'auditIssue', undefined, issue.severity, issue.advisory);
        });
    }
    // ── BUILD.omnisystem parser ───────────────────────────────────────────────
    async loadPackages() {
        if (this.cachedPackages !== undefined) {
            return this.cachedPackages;
        }
        this.cachedPackages = [];
        const buildFile = this.findBuildFile();
        if (!buildFile) {
            return this.cachedPackages;
        }
        try {
            const content = fs.readFileSync(buildFile, 'utf8');
            this.cachedPackages = parseBuildOmnisystem(content);
        }
        catch {
            // ignore parse errors
        }
        return this.cachedPackages;
    }
    findBuildFile() {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders) {
            return undefined;
        }
        const roots = [folders[0].uri, vscode.Uri.joinPath(folders[0].uri, 'Omnisystem')];
        const candidates = [];
        for (const root of roots) {
            candidates.push(vscode.Uri.joinPath(root, 'BUILD.omnisystem').fsPath, vscode.Uri.joinPath(root, 'build.omnisystem').fsPath, vscode.Uri.joinPath(root, 'omnisystem.toml').fsPath);
        }
        for (const c of candidates) {
            if (fs.existsSync(c)) {
                return c;
            }
        }
        return undefined;
    }
    async runAudit() {
        // Stub — in a real extension this would shell out to `omnicc pm audit --json`
        return [];
    }
}
exports.OmniPMExplorerProvider = OmniPMExplorerProvider;
// ─── BUILD.omnisystem parser ──────────────────────────────────────────────────
function parseBuildOmnisystem(content) {
    const packages = [];
    // Simple section-based TOML-like parser
    // Handles:
    //   [dependencies]
    //   omni-http = "1.2.0"
    //
    //   [dev-dependencies]
    //   omni-test = "0.3.1"
    let currentSection = null;
    const lines = content.split('\n');
    for (const rawLine of lines) {
        const line = rawLine.trim();
        if (line === '' || line.startsWith('#')) {
            continue;
        }
        const sectionMatch = line.match(/^\[([^\]]+)\]$/);
        if (sectionMatch) {
            const sec = sectionMatch[1].toLowerCase();
            if (sec === 'dependencies' || sec === 'deps') {
                currentSection = 'dependencies';
            }
            else if (sec === 'dev-dependencies' || sec === 'dev-deps') {
                currentSection = 'dev-dependencies';
            }
            else {
                currentSection = null;
            }
            continue;
        }
        if (currentSection === null) {
            continue;
        }
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
//# sourceMappingURL=OmniPMExplorer.js.map