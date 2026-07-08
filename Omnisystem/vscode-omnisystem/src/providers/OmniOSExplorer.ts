import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

// ─── Data types ───────────────────────────────────────────────────────────────

type SystemItemKind = 'category' | 'systemFile' | 'tool';

interface SystemDescriptor {
    label: string;
    relativePaths: string[];
    description?: string;
}

interface CategoryDescriptor {
    id: string;
    label: string;
    icon: string;
    systems: SystemDescriptor[];
}

// ─── TreeItem ─────────────────────────────────────────────────────────────────

export class SystemItem extends vscode.TreeItem {
    constructor(
        label: string,
        collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly systemKind: SystemItemKind,
        public readonly resourceUri?: vscode.Uri,
        description?: string,
        tooltip?: string,
        iconName?: string,
    ) {
        super(label, collapsibleState);
        this.description = description;
        if (tooltip) { this.tooltip = tooltip; }
        this.contextValue = `omnios.${systemKind}`;
        this.iconPath = new vscode.ThemeIcon(
            iconName ?? (systemKind === 'category' ? 'folder' : 'file-code')
        );

        if (resourceUri && (systemKind === 'systemFile' || systemKind === 'tool')) {
            this.command = {
                command: 'omnisystem.openFile',
                title: 'Open File',
                arguments: [resourceUri],
            };
        }
    }
}

// ─── Category definitions ─────────────────────────────────────────────────────

const CATEGORIES: CategoryDescriptor[] = [
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
            { label: 'TitanFrontend',  relativePaths: ['src/compiler/frontend/TitanFrontend.titan'] },
            { label: 'VeraFrontend',   relativePaths: ['src/compiler/frontend/VeraFrontend.vera'] },
            { label: 'HelixFrontend',  relativePaths: ['src/compiler/frontend/HelixFrontend.helix'] },
            { label: 'AetherFrontend', relativePaths: ['src/compiler/frontend/AetherFrontend.aether'] },
            { label: 'AxiomFrontend',  relativePaths: ['src/compiler/frontend/AxiomFrontend.axiom'] },
            { label: 'SylvaFrontend',  relativePaths: ['src/compiler/frontend/SylvaFrontend.sylva'] },
            { label: 'NexusFrontend',  relativePaths: ['src/compiler/frontend/NexusFrontend.nexus'] },
            { label: 'OmniCC',         relativePaths: ['src/compiler/OmniCC.titan'] },
            { label: 'Linker',         relativePaths: ['src/compiler/Linker.titan'] },
            { label: 'TitanBackend',   relativePaths: ['src/compiler/backend/TitanBackend.titan'] },
            { label: 'NativeBindings', relativePaths: ['src/compiler/native/NativeBindings.titan'] },
        ],
    },
    {
        id: 'stdlib',
        label: 'Standard Library',
        icon: 'book',
        systems: [
            { label: 'TitanStdlib',            relativePaths: ['src/stdlib/TitanStdlib.titan'] },
            { label: 'VeraUIStdlib',            relativePaths: ['src/stdlib/VeraUIStdlib.vera'] },
            { label: 'HelixGraphicsRuntime',    relativePaths: ['src/stdlib/HelixGraphicsRuntime.helix'] },
            { label: 'AetherRuntime',           relativePaths: ['src/stdlib/AetherRuntime.aether'] },
            { label: 'AxiomFormalVerification', relativePaths: ['src/stdlib/AxiomFormalVerification.axiom'] },
            { label: 'SylvaMachineLearning',    relativePaths: ['src/stdlib/SylvaMachineLearning.sylva'] },
            { label: 'NexusResponsiveDesign',   relativePaths: ['src/stdlib/NexusResponsiveDesign.nexus'] },
            { label: 'TitanStdlib (core)',       relativePaths: ['src/stdlib/core.ti'] },
            { label: 'TitanStdlib (io)',         relativePaths: ['src/stdlib/io.ti'] },
            { label: 'TitanStdlib (math)',       relativePaths: ['src/stdlib/math.ti'] },
            { label: 'TitanStdlib (string)',     relativePaths: ['src/stdlib/string.ti'] },
            { label: 'TitanStdlib (collections)',relativePaths: ['src/stdlib/collections.ti'] },
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
            { label: 'Networking',          relativePaths: ['src/systems/networking/AdvancedNetworking.titan'] },
            { label: 'Service Mesh',        relativePaths: ['src/systems/networking/ServiceMesh.titan'] },
            { label: 'Distributed DB',     relativePaths: ['src/systems/enterprise/DistributedDatabase.titan'] },
            { label: 'Container Registry', relativePaths: ['src/systems/cloud/ContainerRegistry.aether'] },
            { label: 'Service Discovery',  relativePaths: ['src/systems/cloud/ServiceDiscovery.aether'] },
            { label: 'Security Manager',   relativePaths: ['src/systems/security/AdvancedSecurityManager.vera'] },
            { label: 'Authentication',     relativePaths: ['src/systems/security/auth/AuthenticationManager.vera'] },
            { label: 'Cache Manager',      relativePaths: ['src/systems/cache/CacheManager.vera'] },
            { label: 'Monitoring',         relativePaths: ['src/systems/monitoring/monitoring_system.titan'] },
            { label: 'Monitoring Dashboard',relativePaths: ['src/systems/monitoring/MonitoringDashboard.vera'] },
            { label: 'API Versioning',     relativePaths: ['src/systems/api/APIVersioning.titan'] },
            { label: 'REST/GraphQL',       relativePaths: ['src/systems/api/RESTGraphQLFramework.vera'] },
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

export class OmniOSExplorerProvider
    implements vscode.TreeDataProvider<SystemItem>
{
    private readonly _onDidChangeTreeData =
        new vscode.EventEmitter<SystemItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private workspaceRoot: vscode.Uri | undefined) {}

    refresh(newRoot?: vscode.Uri): void {
        if (newRoot) { this.workspaceRoot = newRoot; }
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: SystemItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: SystemItem): Promise<SystemItem[]> {
        if (!element) {
            const launchItem = new SystemItem(
                'Launch OmniOS Desktop',
                vscode.TreeItemCollapsibleState.None,
                'tool',
                undefined,
                'Open interactive desktop environment',
                'Click to open the full OmniOS Desktop with terminal, file manager, compiler, ML studio and more',
                'rocket',
            );
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

    private getCategoryItems(): SystemItem[] {
        return CATEGORIES.map((cat) => {
            const found = this.countFoundSystems(cat);
            return new SystemItem(
                cat.label,
                vscode.TreeItemCollapsibleState.Collapsed,
                'category',
                undefined,
                `${found}/${cat.systems.length} systems`,
                cat.label,
                cat.icon,
            );
        });
    }

    // ── System file items ─────────────────────────────────────────────────────

    private getSystemItems(category: CategoryDescriptor): SystemItem[] {
        return category.systems.map((sys) => {
            const uri = this.resolveSystemUri(sys);
            const exists = uri !== undefined;
            const lineCount = exists ? this.countLines(uri!) : 0;

            return new SystemItem(
                sys.label,
                vscode.TreeItemCollapsibleState.None,
                'systemFile',
                uri,
                exists
                    ? (lineCount > 0 ? `${lineCount.toLocaleString()} lines` : '')
                    : '$(warning) not found',
                exists ? uri!.fsPath : `${sys.label} — file not found`,
            );
        });
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private resolveSystemUri(sys: SystemDescriptor): vscode.Uri | undefined {
        if (!this.workspaceRoot) { return undefined; }

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

    private countFoundSystems(category: CategoryDescriptor): number {
        return category.systems.filter(
            (sys) => this.resolveSystemUri(sys) !== undefined
        ).length;
    }

    private countLines(uri: vscode.Uri): number {
        try {
            const content = fs.readFileSync(uri.fsPath, 'utf8');
            return content.split('\n').length;
        } catch {
            return 0;
        }
    }
}
