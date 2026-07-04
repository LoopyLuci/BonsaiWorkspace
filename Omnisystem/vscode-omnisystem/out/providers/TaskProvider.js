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
exports.OmnisystemTaskProvider = void 0;
const vscode = __importStar(require("vscode"));
function getTaskTemplates(target, optLevel) {
    return [
        {
            name: 'Build',
            command: 'build',
            args: ['--target', target, '--opt', optLevel],
            group: vscode.TaskGroup.Build,
            detail: `Build the Omnisystem project for ${target}`,
        },
        {
            name: 'Build Release',
            command: 'build',
            args: ['--target', target, '--opt', 'O3', '--release'],
            group: vscode.TaskGroup.Build,
            detail: `Build an optimized release binary for ${target}`,
        },
        {
            name: 'Build WASM',
            command: 'build',
            args: ['--target', 'wasm32-unknown', '--opt', optLevel],
            group: vscode.TaskGroup.Build,
            detail: 'Compile to WebAssembly',
        },
        {
            name: 'Run',
            command: 'run',
            args: ['--target', target, '--opt', optLevel],
            detail: 'Build and run the project',
        },
        {
            name: 'Test',
            command: 'test',
            args: [],
            group: vscode.TaskGroup.Test,
            detail: 'Run the full test suite',
        },
        {
            name: 'Clean',
            command: 'clean',
            args: [],
            group: vscode.TaskGroup.Clean,
            detail: 'Remove build artifacts',
        },
        {
            name: 'Benchmark',
            command: 'bench',
            args: ['--target', target],
            detail: 'Run benchmark suite',
        },
        {
            name: 'Check',
            command: 'check',
            args: [],
            detail: 'Type-check all source files without emitting code',
        },
        {
            name: 'Format',
            command: 'fmt',
            args: ['--all'],
            detail: 'Format all source files',
        },
        {
            name: 'Generate Docs',
            command: 'doc',
            args: ['--open'],
            detail: 'Generate and open project documentation',
        },
    ];
}
// ─── Provider ─────────────────────────────────────────────────────────────────
class OmnisystemTaskProvider {
    provideTasks() {
        if (!this.cachedTasks) {
            this.cachedTasks = this.buildAllTasks();
        }
        return this.cachedTasks;
    }
    resolveTask(task) {
        const def = task.definition;
        if (def.type === OmnisystemTaskProvider.taskType &&
            typeof def.command === 'string' &&
            def.command.length > 0) {
            return this.createTask(def.command, def.args ?? [], task.name, undefined);
        }
        return undefined;
    }
    // ── Internal builders ─────────────────────────────────────────────────────
    buildAllTasks() {
        const config = vscode.workspace.getConfiguration('omnisystem');
        const omniccPath = config.get('omniccPath', 'omnicc').trim();
        const target = config.get('buildTarget', 'x86_64-linux');
        const optLevel = config.get('optimizationLevel', 'O0');
        return getTaskTemplates(target, optLevel).map((tpl) => {
            const def = {
                type: OmnisystemTaskProvider.taskType,
                command: tpl.command,
                args: tpl.args,
            };
            const shellCmd = buildShellCommand(omniccPath, tpl.command, tpl.args);
            const task = new vscode.Task(def, vscode.TaskScope.Workspace, tpl.name, 'omnisystem', new vscode.ShellExecution(shellCmd), '$omnicc');
            if (tpl.group) {
                task.group = tpl.group;
            }
            if (tpl.detail) {
                task.detail = tpl.detail;
            }
            if (tpl.isBackground) {
                task.isBackground = true;
            }
            return task;
        });
    }
    createTask(command, args, name, group) {
        const config = vscode.workspace.getConfiguration('omnisystem');
        const omniccPath = config.get('omniccPath', 'omnicc').trim();
        const def = {
            type: OmnisystemTaskProvider.taskType,
            command,
            args,
        };
        const task = new vscode.Task(def, vscode.TaskScope.Workspace, name, 'omnisystem', new vscode.ShellExecution(buildShellCommand(omniccPath, command, args)), '$omnicc');
        if (group) {
            task.group = group;
        }
        return task;
    }
}
exports.OmnisystemTaskProvider = OmnisystemTaskProvider;
OmnisystemTaskProvider.taskType = 'omnisystem';
// ─── Shell command builder ────────────────────────────────────────────────────
function buildShellCommand(omniccPath, command, args) {
    const parts = [omniccPath, command, ...args];
    return parts.join(' ');
}
//# sourceMappingURL=TaskProvider.js.map