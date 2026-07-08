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
exports.OmniOSExplorerProvider = exports.SystemItem = void 0;
const vscode = __importStar(require("vscode"));
const fs = __importStar(require("fs"));
// ─── TreeItem ─────────────────────────────────────────────────────────────────
class SystemItem extends vscode.TreeItem {
    constructor(label, collapsibleState, systemKind, resourceUri, description, tooltip, iconName) {
        super(label, collapsibleState);
        this.systemKind = systemKind;
        this.resourceUri = resourceUri;
        this.description = description;
        if (tooltip) {
            this.tooltip = tooltip;
        }
        this.contextValue = `omnios.${systemKind}`;
        this.iconPath = new vscode.ThemeIcon(iconName ?? (systemKind === 'category' ? 'folder' : 'file-code'));
        if (resourceUri && (systemKind === 'systemFile' || systemKind === 'tool')) {
            this.command = {
                command: 'omnisystem.openFile',
                title: 'Open File',
                arguments: [resourceUri],
            };
        }
    }
}
exports.SystemItem = SystemItem;
// ─── Category definitions ─────────────────────────────────────────────────────
const CATEGORIES = [
    {
        id: 'kernel',
        label: 'Kernel',
        icon: 'circuit-board',
        systems: [
            {
                label: 'OmniOS_Bootstrap_Launcher',
                relativePaths: [
                    'src/compiler/OmniOS_Bootstrap_Launcher.titan',
                    'src/systems/OmniOS_Bootstrap_Launcher.titan',
                    'OmniOS_Bootstrap_Launcher.titan',
                ],
            },
            {
                label: 'OmnisystemRuntime',
                relativePaths: [
                    'src/compiler/runtime/OmnisystemRuntime.titan',
                    'src/runtime/OmnisystemRuntime.titan',
                ],
            },
            {
                label: 'OmnisystemRuntimeVM',
                relativePaths: [
                    'src/compiler/runtime/OmnisystemRuntimeVM.titan',
                ],
            },
        ],
    },
    {
        id: 'compiler',
        label: 'Compiler',
        icon: 'symbol-constructor',
        systems: [
            { label: 'TitanFrontend', relativePaths: ['src/compiler/frontend/TitanFrontend.titan'] },
            { label: 'VeraFrontend', relativePaths: ['src/compiler/frontend/VeraFrontend.vera'] },
            { label: 'HelixFrontend', relativePaths: ['src/compiler/frontend/HelixFrontend.helix'] },
            { label: 'AetherFrontend', relativePaths: ['src/compiler/frontend/AetherFrontend.aether'] },
            { label: 'AxiomFrontend', relativePaths: ['src/compiler/frontend/AxiomFrontend.axiom'] },
            { label: 'SylvaFrontend', relativePaths: ['src/compiler/frontend/SylvaFrontend.sylva'] },
            { label: 'NexusFrontend', relativePaths: ['src/compiler/frontend/NexusFrontend.nexus'] },
            { label: 'OmniCC', relativePaths: ['src/compiler/OmniCC.titan'] },
            { label: 'Linker', relativePaths: ['src/compiler/Linker.titan'] },
            { label: 'TitanBackend', relativePaths: ['src/compiler/backend/TitanBackend.titan'] },
            { label: 'NativeBindings', relativePaths: ['src/compiler/native/NativeBindings.titan'] },
        ],
    },
    {
        id: 'stdlib',
        label: 'Standard Library',
        icon: 'book',
        systems: [
            { label: 'TitanStdlib', relativePaths: ['src/stdlib/TitanStdlib.titan'] },
            { label: 'VeraUIStdlib', relativePaths: ['src/stdlib/VeraUIStdlib.vera'] },
            { label: 'HelixGraphicsRuntime', relativePaths: ['src/stdlib/HelixGraphicsRuntime.helix'] },
            { label: 'AetherRuntime', relativePaths: ['src/stdlib/AetherRuntime.aether'] },
            { label: 'AxiomFormalVerification', relativePaths: ['src/stdlib/AxiomFormalVerification.axiom'] },
            { label: 'SylvaMachineLearning', relativePaths: ['src/stdlib/SylvaMachineLearning.sylva'] },
            { label: 'NexusResponsiveDesign', relativePaths: ['src/stdlib/NexusResponsiveDesign.nexus'] },
            { label: 'TitanStdlib (core)', relativePaths: ['src/stdlib/core.ti'] },
            { label: 'TitanStdlib (io)', relativePaths: ['src/stdlib/io.ti'] },
            { label: 'TitanStdlib (math)', relativePaths: ['src/stdlib/math.ti'] },
            { label: 'TitanStdlib (string)', relativePaths: ['src/stdlib/string.ti'] },
            { label: 'TitanStdlib (collections)', relativePaths: ['src/stdlib/collections.ti'] },
        ],
    },
    {
        id: 'applications',
        label: 'Applications',
        icon: 'window',
        systems: [
            {
                label: 'Desktop Environment',
                relativePaths: [
                    'src/systems/applications/omnisystem-desktop-environment/src/main/DesktopEnvironment.vera',
                ],
            },
            {
                label: 'Desktop Environment (Complete)',
                relativePaths: [
                    'src/systems/applications/omnisystem-desktop-environment/src/main/DesktopEnvironmentComplete.vera',
                ],
            },
            {
                label: 'Application Launcher',
                relativePaths: [
                    'src/systems/applications/omnisystem-desktop-environment/src/launcher/ApplicationLauncher.vera',
                ],
            },
            {
                label: 'File Manager',
                relativePaths: [
                    'src/systems/applications/omnisystem-desktop-environment/src/file-manager/FileManager.vera',
                ],
            },
            {
                label: 'Notification System',
                relativePaths: [
                    'src/systems/applications/omnisystem-desktop-environment/src/notifications/NotificationSystem.vera',
                ],
            },
            {
                label: 'Analytics Dashboard',
                relativePaths: [
                    'src/systems/applications/omnisystem-desktop-environment/src/intelligence/AnalyticsDashboard.vera',
                    'src/systems/analytics/AnalyticsEngine.vera',
                ],
            },
        ],
    },
    {
        id: 'infrastructure',
        label: 'Infrastructure',
        icon: 'server',
        systems: [
            { label: 'Networking', relativePaths: ['src/systems/networking/AdvancedNetworking.titan'] },
            { label: 'Service Mesh', relativePaths: ['src/systems/networking/ServiceMesh.titan'] },
            { label: 'Distributed DB', relativePaths: ['src/systems/enterprise/DistributedDatabase.titan'] },
            { label: 'Container Registry', relativePaths: ['src/systems/cloud/ContainerRegistry.aether'] },
            { label: 'Service Discovery', relativePaths: ['src/systems/cloud/ServiceDiscovery.aether'] },
            { label: 'Security Manager', relativePaths: ['src/systems/security/AdvancedSecurityManager.vera'] },
            { label: 'Authentication', relativePaths: ['src/systems/security/auth/AuthenticationManager.vera'] },
            { label: 'Cache Manager', relativePaths: ['src/systems/cache/CacheManager.vera'] },
            { label: 'Monitoring', relativePaths: ['src/systems/monitoring/monitoring_system.titan'] },
            { label: 'Monitoring Dashboard', relativePaths: ['src/systems/monitoring/MonitoringDashboard.vera'] },
            { label: 'API Versioning', relativePaths: ['src/systems/api/APIVersioning.titan'] },
            { label: 'REST/GraphQL', relativePaths: ['src/systems/api/RESTGraphQLFramework.vera'] },
        ],
    },
    {
        id: 'tools',
        label: 'Tools',
        icon: 'tools',
        systems: [
            {
                label: 'OmniPM',
                relativePaths: [
                    'src/package_manager/OmniPM.titan',
                    'src/omnipm/OmniPM.titan',
                ],
            },
            {
                label: 'OmniPM CLI',
                relativePaths: [
                    'src/package_manager/OmniPMCLI.titan',
                    'src/omnipm/OmniPMCLI.titan',
                ],
            },
            {
                label: 'Bootstrap',
                relativePaths: [
                    'src/compiler/bootstrap/Bootstrap.titan',
                    'src/bootstrap/Bootstrap.titan',
                ],
            },
            {
                label: 'Bootstrap Validator',
                relativePaths: [
                    'src/compiler/bootstrap/BootstrapValidator.axiom',
                ],
            },
            {
                label: 'Compiler Integration Tests',
                relativePaths: [
                    'src/compiler/CompilerIntegrationTest.titan',
                    'src/compiler/CompilerIntegrationTests.titan',
                ],
            },
        ],
    },
];
// ─── Provider ─────────────────────────────────────────────────────────────────
class OmniOSExplorerProvider {
    constructor(workspaceRoot) {
        this.workspaceRoot = workspaceRoot;
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh(newRoot) {
        if (newRoot) {
            this.workspaceRoot = newRoot;
        }
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (!element) {
            const launchItem = new SystemItem('Launch OmniOS Desktop', vscode.TreeItemCollapsibleState.None, 'tool', undefined, 'Open interactive desktop environment', 'Click to open the full OmniOS Desktop with terminal, file manager, compiler, ML studio and more', 'rocket');
            launchItem.command = { command: 'omnisystem.omniOsBoot', title: 'Launch OmniOS Desktop' };
            return [launchItem, ...this.getCategoryItems()];
        }
        // Find the matching category by label
        const label = typeof element.label === 'string' ? element.label : '';
        const category = CATEGORIES.find((c) => c.label === label);
        if (category) {
            return this.getSystemItems(category);
        }
        return [];
    }
    // ── Root categories ───────────────────────────────────────────────────────
    getCategoryItems() {
        return CATEGORIES.map((cat) => {
            const found = this.countFoundSystems(cat);
            return new SystemItem(cat.label, vscode.TreeItemCollapsibleState.Collapsed, 'category', undefined, `${found}/${cat.systems.length} systems`, cat.label, cat.icon);
        });
    }
    // ── System file items ─────────────────────────────────────────────────────
    getSystemItems(category) {
        return category.systems.map((sys) => {
            const uri = this.resolveSystemUri(sys);
            const exists = uri !== undefined;
            const lineCount = exists ? this.countLines(uri) : 0;
            return new SystemItem(sys.label, vscode.TreeItemCollapsibleState.None, 'systemFile', uri, exists
                ? (lineCount > 0 ? `${lineCount.toLocaleString()} lines` : '')
                : '$(warning) not found', exists ? uri.fsPath : `${sys.label} — file not found`);
        });
    }
    // ── Helpers ───────────────────────────────────────────────────────────────
    resolveSystemUri(sys) {
        if (!this.workspaceRoot) {
            return undefined;
        }
        // Try each relative path, also prefixed with 'Omnisystem/' in case
        // the workspace is opened one level above the Omnisystem monorepo root.
        const roots = [
            this.workspaceRoot,
            vscode.Uri.joinPath(this.workspaceRoot, 'Omnisystem'),
        ];
        for (const root of roots) {
            for (const relPath of sys.relativePaths) {
                const uri = vscode.Uri.joinPath(root, relPath);
                if (fs.existsSync(uri.fsPath)) {
                    return uri;
                }
            }
        }
        return undefined;
    }
    countFoundSystems(category) {
        return category.systems.filter((sys) => this.resolveSystemUri(sys) !== undefined).length;
    }
    countLines(uri) {
        try {
            const content = fs.readFileSync(uri.fsPath, 'utf8');
            return content.split('\n').length;
        }
        catch {
            return 0;
        }
    }
}
exports.OmniOSExplorerProvider = OmniOSExplorerProvider;
//# sourceMappingURL=OmniOSExplorer.js.map