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
exports.DesktopExplorerProvider = exports.DesktopItem = void 0;
const vscode = __importStar(require("vscode"));
const fs = __importStar(require("fs"));
class DesktopItem extends vscode.TreeItem {
    constructor(label, collapsibleState, desktopKind, resourceUri, commandId, description, tooltip) {
        super(label, collapsibleState);
        this.desktopKind = desktopKind;
        this.resourceUri = resourceUri;
        this.commandId = commandId;
        this.description = description;
        if (tooltip) {
            this.tooltip = tooltip;
        }
        this.contextValue = `desktop.${desktopKind}`;
        if (commandId) {
            this.command = { command: commandId, title: label, arguments: [resourceUri] };
        }
        else if (resourceUri && (desktopKind === 'config' || desktopKind === 'source' || desktopKind === 'titanSource')) {
            this.command = {
                command: 'omnisystem.openFile',
                title: 'Open File',
                arguments: [resourceUri],
            };
        }
        this.iconPath = this.resolveIcon();
    }
    resolveIcon() {
        switch (this.desktopKind) {
            case 'section': return new vscode.ThemeIcon('folder');
            case 'subsection': return new vscode.ThemeIcon('folder-opened');
            case 'config': return new vscode.ThemeIcon('settings-gear');
            case 'command': return new vscode.ThemeIcon('play');
            case 'status': return new vscode.ThemeIcon('circle-outline');
            case 'subApp': return new vscode.ThemeIcon('extensions');
            case 'source': return new vscode.ThemeIcon('file-code');
            case 'titanSource': return new vscode.ThemeIcon('symbol-module');
            default: return new vscode.ThemeIcon('circle-outline');
        }
    }
}
exports.DesktopItem = DesktopItem;
// ─── Provider ─────────────────────────────────────────────────────────────────
class DesktopExplorerProvider {
    constructor(ecosystemRoot) {
        this.ecosystemRoot = ecosystemRoot;
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh(newRoot) {
        if (newRoot) {
            this.ecosystemRoot = newRoot;
        }
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) { return element; }
    async getChildren(element) {
        if (!element) {
            return this.getRootSections();
        }
        const label = typeof element.label === 'string' ? element.label : '';
        switch (label) {
            // Infrastructure
            case 'Control Panel': return this.getControlPanelChildren();
            case 'Notification System': return this.getNotificationsChildren();
            case 'System Tray': return this.getSystemTrayChildren();
            case 'Master Initialization': return this.getInitChildren();
            // Applications
            case 'OmniHarness AI': return this.getHarnessChildren();
            case 'Workspace IDE': return this.getWorkspaceChildren();
            case 'Buddy AI (Android)': return this.getBuddyChildren();
            case 'Browser Extension': return this.getBrowserExtChildren();
            case 'App Launcher': return this.getLauncherChildren();
            case 'Runtime Manager': return this.getRuntimeChildren();
            // OS Integration
            case 'File Associations': return this.getFileAssocChildren();
            case 'Theme System': return this.getThemeChildren();
            case 'Installer': return this.getInstallerChildren();
            // Integration
            case 'Omnisystem Bridge': return this.getIntegrationChildren();
            default: return [];
        }
    }
    // ── Helpers ───────────────────────────────────────────────────────────────
    eco(...segments) {
        if (!this.ecosystemRoot) {
            return undefined;
        }
        return vscode.Uri.joinPath(this.ecosystemRoot, ...segments);
    }
    ecoPath(...segments) {
        const u = this.eco(...segments);
        return u?.fsPath;
    }
    exists(...segments) {
        const p = this.ecoPath(...segments);
        return !!p && fs.existsSync(p);
    }
    found(label, ...segs) {
        return this.exists(...segs) ? `$(check) ${label}` : `$(warning) not found`;
    }
    locCount(...segments) {
        const p = this.ecoPath(...segments);
        if (!p || !fs.existsSync(p)) {
            return '';
        }
        try {
            const lines = fs.readFileSync(p, 'utf8').split('\n').length;
            return `${lines} LOC`;
        }
        catch {
            return '';
        }
    }
    titanItem(name, ...segs) {
        const uri = this.eco(...segs);
        const loc = this.locCount(...segs);
        const exists = this.exists(...segs);
        return new DesktopItem(name, vscode.TreeItemCollapsibleState.None, 'titanSource', uri, exists ? 'omnisystem.openFile' : undefined, exists ? loc : '$(warning) missing', exists ? uri?.fsPath : `Not found: ${segs.join('/')}`);
    }
    cmdItem(label, commandId, desc) {
        return new DesktopItem(label, vscode.TreeItemCollapsibleState.None, 'command', undefined, commandId, desc);
    }
    sectionItem(label, desc) {
        return new DesktopItem(label, vscode.TreeItemCollapsibleState.Collapsed, 'section', undefined, undefined, desc);
    }
    // ── Root sections ─────────────────────────────────────────────────────────
    async getRootSections() {
        const eco = this.ecosystemRoot;
        const detected = eco && fs.existsSync(eco.fsPath);
        const infraStatus = detected ? '$(check) ready' : '$(warning) not found';
        const appsStatus = detected ? '$(check) 5 apps' : '$(warning) not found';
        const osStatus = detected ? '$(check) integrated' : '$(warning) not found';
        const harnessStatus = await this.harnessStatusLabel();
        return [
            // Group 1: Infrastructure
            this.sectionItem('Control Panel', infraStatus),
            this.sectionItem('Notification System', infraStatus),
            this.sectionItem('System Tray', infraStatus),
            this.sectionItem('Master Initialization', infraStatus),
            // Group 2: Applications
            this.sectionItem('OmniHarness AI', harnessStatus),
            this.sectionItem('Workspace IDE', appsStatus),
            this.sectionItem('Buddy AI (Android)', appsStatus),
            this.sectionItem('Browser Extension', appsStatus),
            this.sectionItem('App Launcher', appsStatus),
            this.sectionItem('Runtime Manager', appsStatus),
            // Group 3: OS Integration
            this.sectionItem('File Associations', osStatus),
            this.sectionItem('Theme System', osStatus),
            this.sectionItem('Installer', osStatus),
            // Group 4: Integration
            this.sectionItem('Omnisystem Bridge', infraStatus),
        ];
    }
    // ── OmniHarness AI ────────────────────────────────────────────────────────
    /** Live reachability check against the orchestrator's health endpoint. */
    async harnessStatusLabel() {
        const base = vscode.workspace.getConfiguration('omnisystem').get('harness.serverUrl', 'http://localhost:8080').replace(/\/+$/, '');
        try {
            const ctrl = new AbortController();
            const timer = setTimeout(() => ctrl.abort(), 800);
            const resp = await fetch(`${base}/api/health`, { signal: ctrl.signal });
            clearTimeout(timer);
            return resp.ok ? '$(check) running' : '$(warning) unreachable';
        }
        catch {
            return '$(circle-outline) offline';
        }
    }
    getHarnessChildren() {
        return [
            this.cmdItem('Open Chat Panel', 'omnisystem.harnessFocus', 'Focus the OmniHarness AI sidebar'),
            this.cmdItem('Start Server', 'omnisystem.harnessStartServer', 'Launch the OmniHarness orchestrator'),
            this.cmdItem('Stop Server', 'omnisystem.harnessStopServer', 'Stop the orchestrator process'),
            this.cmdItem('New Session', 'omnisystem.harnessNewSession', 'Clear chat history and start fresh'),
            this.cmdItem('Settings', 'omnisystem.harnessSettings', 'Providers, local models, custom agents, MCP servers'),
            new DesktopItem('Capabilities', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Local + API models · MCP · Swarm/Ensemble', 'Any local (Ollama/llama.cpp/LM Studio) or API model (Anthropic/OpenAI/Google/Groq/Mistral/...); VS Code tool bridge; Model Context Protocol client+server'),
        ];
    }
    // ── Control Panel ─────────────────────────────────────────────────────────
    getControlPanelChildren() {
        return [
            this.titanItem('core.ti', 'control-panel', 'core.ti'),
            this.titanItem('api_server.ti', 'control-panel', 'api_server.ti'),
            this.cmdItem('Open on :12345', 'omnisystem.desktopControlPanel', 'Opens localhost:12345'),
            new DesktopItem('Port', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '12345', 'REST API port defined in api_server.ti'),
            new DesktopItem('Endpoints', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '30+ REST APIs', 'system, cpu, memory, services, snapshots'),
        ];
    }
    // ── Notification System ───────────────────────────────────────────────────
    getNotificationsChildren() {
        return [
            this.titanItem('notification_daemon.ti', 'notifications', 'notification_daemon.ti'),
            this.cmdItem('Open Dashboard', 'omnisystem.desktopNotifications', 'Open Desktop Dashboard'),
            new DesktopItem('Queue Limit', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '1,000 max', 'with SQLite persistence'),
            new DesktopItem('Platforms', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Win/macOS/Linux', 'WinRT · NSUserNotification · D-Bus'),
            new DesktopItem('Features', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'DnD · history · badge', 'action buttons, dedup, auto-dismiss'),
        ];
    }
    // ── System Tray ───────────────────────────────────────────────────────────
    getSystemTrayChildren() {
        return [
            this.titanItem('core.ti', 'system-tray', 'core.ti'),
            this.cmdItem('Open Source File', 'omnisystem.desktopSystemTray', 'View system-tray/core.ti'),
            new DesktopItem('Menu Items', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '11 items', 'Open · Workspace · Buddy · Control Panel · Exit…'),
            new DesktopItem('Platforms', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Win/macOS/Linux', 'Win32 NotifyIcon · NSStatusBar · D-Bus'),
            new DesktopItem('Features', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'badge · quick panel', 'left/right/double-click events'),
        ];
    }
    // ── Master Initialization ─────────────────────────────────────────────────
    getInitChildren() {
        return [
            this.titanItem('INITIALIZATION.ti', 'INITIALIZATION.ti'),
            this.cmdItem('Run Init / Diagnostics', 'omnisystem.desktopInit', '5-phase startup or diagnostics'),
            new DesktopItem('Phase 1', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Omnisystem Integration'),
            new DesktopItem('Phase 2', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'System Infrastructure'),
            new DesktopItem('Phase 3', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Application Services'),
            new DesktopItem('Phase 4', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'OS-level Integration'),
            new DesktopItem('Phase 5', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Verification & Health'),
        ];
    }
    // ── Workspace IDE ─────────────────────────────────────────────────────────
    getWorkspaceChildren() {
        const srcTauri = this.eco('workspace', 'src-tauri');
        const items = [
            new DesktopItem('src-tauri/', vscode.TreeItemCollapsibleState.None, 'source', srcTauri, srcTauri && fs.existsSync(srcTauri.fsPath) ? 'omnisystem.openFile' : undefined, this.exists('workspace', 'src-tauri') ? '$(check)' : '$(warning)'),
            this.cmdItem('Launch Dev', 'omnisystem.desktopLaunch', 'npx tauri dev'),
            this.cmdItem('Build Release', 'omnisystem.desktopBuild', 'npx tauri build'),
            new DesktopItem('Runtime', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Tauri v2 + Rust', '177+ source files'),
            new DesktopItem('Languages', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'All 7 supported', 'Titan·Vera·Helix·Aether·Axiom·Sylva·Nexus'),
        ];
        return items;
    }
    // ── Buddy AI ─────────────────────────────────────────────────────────────
    getBuddyChildren() {
        const subApps = [
            ['app', 'Main App'],
            ['app-academy', 'Academy'],
            ['app-ai-power-user', 'AI Power User'],
            ['app-computedonor', 'Compute Donor'],
            ['app-developer-suite', 'Developer Suite'],
            ['app-modelmanager', 'Model Manager'],
            ['app-nodecontroller', 'Node Controller'],
            ['app-workspace', 'Workspace'],
        ];
        const buddyBase = ['buddy', 'android', 'runtime', 'android-runtime'];
        const items = [
            new DesktopItem('Connection Status', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, this.adbStatus(), 'Android Debug Bridge'),
            this.cmdItem('Build APK', 'omnisystem.desktopBuddyBuild', 'Build Android release APK'),
            this.cmdItem('Connect ADB', 'omnisystem.desktopBuddyConnect', 'adb reverse tcp:8081'),
        ];
        for (const [dir, friendlyName] of subApps) {
            const appUri = this.eco(...buddyBase, dir);
            const exists = this.exists(...buddyBase, dir);
            items.push(new DesktopItem(friendlyName, vscode.TreeItemCollapsibleState.None, 'subApp', appUri, exists ? 'omnisystem.openFile' : undefined, exists ? '' : '$(warning) not found'));
        }
        return items;
    }
    // ── Browser Extension ─────────────────────────────────────────────────────
    getBrowserExtChildren() {
        const base = ['browser-extension'];
        return [
            this.titanItem('manifest.json', ...base, 'manifest.json'),
            this.titanItem('popup.ts', ...base, 'src', 'popup.ts'),
            this.titanItem('sidebar.ts', ...base, 'src', 'sidebar.ts'),
            this.titanItem('background.ts', ...base, 'src', 'background.ts'),
            this.cmdItem('Build', 'omnisystem.desktopBrowserExtBuild', 'npm run build'),
            this.cmdItem('Install Dev Mode', 'omnisystem.desktopBrowserExtInstall', 'Load unpacked'),
            new DesktopItem('Bundle', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '55 KB gzip · Manifest v3'),
            new DesktopItem('Targets', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Chrome · Firefox · Edge · Safari'),
        ];
    }
    // ── App Launcher ──────────────────────────────────────────────────────────
    getLauncherChildren() {
        return [
            this.titanItem('app.omnisystem.toml', 'launcher', 'app.omnisystem.toml'),
            this.titanItem('tauri.conf.json', 'launcher', 'tauri.conf.json'),
            this.cmdItem('Launch Dev', 'omnisystem.desktopLaunch', 'Tauri dev mode'),
            new DesktopItem('Health Port', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, ':11450', 'Launcher health check endpoint'),
            new DesktopItem('Pre-registered Apps', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '50+ apps'),
        ];
    }
    // ── Runtime Manager ───────────────────────────────────────────────────────
    getRuntimeChildren() {
        const rtDir = this.eco('runtime');
        return [
            new DesktopItem('runtime/', vscode.TreeItemCollapsibleState.None, 'source', rtDir, rtDir && fs.existsSync(rtDir.fsPath) ? 'omnisystem.openFile' : undefined, this.exists('runtime') ? '$(check)' : '$(warning)'),
            new DesktopItem('Process Control', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'kill · wait · PID'),
            new DesktopItem('WASM Runtime', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'wasmtime'),
            new DesktopItem('Sandboxing', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Windows Job Objects · cgroups v2'),
        ];
    }
    // ── File Associations ─────────────────────────────────────────────────────
    getFileAssocChildren() {
        return [
            this.titanItem('core.ti', 'file-associations', 'core.ti'),
            new DesktopItem('.ti  .omnisystem  .model', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '7 types registered'),
            new DesktopItem('.code  .omnib  .workspace', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '.omnisystem-config'),
            new DesktopItem('Platforms', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Win registry · LaunchServices · mimeapps'),
        ];
    }
    // ── Theme System ──────────────────────────────────────────────────────────
    getThemeChildren() {
        return [
            this.titanItem('core.ti', 'theme-system', 'core.ti'),
            new DesktopItem('Themes', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '10 built-in', 'Dark · Light · High Contrast · Solarized…'),
            new DesktopItem('Customisation', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Color · Font · Spacing'),
            new DesktopItem('Live Switching', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '$(check) supported'),
        ];
    }
    // ── Installer ─────────────────────────────────────────────────────────────
    getInstallerChildren() {
        return [
            this.titanItem('core.ti', 'installer', 'core.ti'),
            this.titanItem('host_detection.ti', 'installer', 'host_detection.ti'),
            new DesktopItem('Preflight Checks', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '$(check) implemented'),
            new DesktopItem('Dependency Mgmt', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '$(check) implemented'),
            new DesktopItem('Platforms', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, 'Windows · macOS · Linux'),
        ];
    }
    // ── Omnisystem Integration Bridge ─────────────────────────────────────────
    getIntegrationChildren() {
        return [
            this.titanItem('omnisystem_integration.ti', 'integration', 'omnisystem_integration.ti'),
            new DesktopItem('Capabilities', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '50+ registered'),
            new DesktopItem('Sub-apps', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '5 apps wired'),
            new DesktopItem('AI Shim', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '6 providers unified'),
            new DesktopItem('Hot-reload', vscode.TreeItemCollapsibleState.None, 'status', undefined, undefined, '$(check) module system'),
            this.cmdItem('Initialize', 'omnisystem.desktopInit', 'Run full ecosystem initialization'),
        ];
    }
    // ── Status helpers ────────────────────────────────────────────────────────
    adbStatus() {
        return '$(circle-outline) Unknown — run "Connect ADB" to check';
    }
}
exports.DesktopExplorerProvider = DesktopExplorerProvider;
//# sourceMappingURL=DesktopExplorer.js.map