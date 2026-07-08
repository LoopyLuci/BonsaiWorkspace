import * as vscode from 'vscode';

// ─── Task definition ──────────────────────────────────────────────────────────

export interface OmnisystemTaskDefinition extends vscode.TaskDefinition {
    /** OmniCC sub-command, e.g. "build", "test" */
    command: string;
    /** Additional arguments appended to the command */
    args?: string[];
}

// ─── Task templates ───────────────────────────────────────────────────────────

interface TaskTemplate {
    name: string;
    command: string;
    args: string[];
    group?: vscode.TaskGroup;
    detail?: string;
    isBackground?: boolean;
}

function getTaskTemplates(
    target: string,
    optLevel: string,
): TaskTemplate[] {
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

export class OmnisystemTaskProvider implements vscode.TaskProvider {
    static readonly taskType = 'omnisystem';

    private cachedTasks: vscode.Task[] | undefined;

    provideTasks(): vscode.Task[] {
        if (!this.cachedTasks) {
            this.cachedTasks = this.buildAllTasks();
        }
        return this.cachedTasks;
    }

    resolveTask(task: vscode.Task): vscode.Task | undefined {
        const def = task.definition as OmnisystemTaskDefinition;
        if (
            def.type === OmnisystemTaskProvider.taskType &&
            typeof def.command === 'string' &&
            def.command.length > 0
        ) {
            return this.createTask(def.command, def.args ?? [], task.name, undefined);
        }
        return undefined;
    }

    // ── Internal builders ─────────────────────────────────────────────────────

    private buildAllTasks(): vscode.Task[] {
        const config = vscode.workspace.getConfiguration('omnisystem');
        const omniccPath = config.get<string>('omniccPath', 'omnicc').trim();
        const target     = config.get<string>('buildTarget', 'x86_64-linux');
        const optLevel   = config.get<string>('optimizationLevel', 'O0');

        return getTaskTemplates(target, optLevel).map((tpl) => {
            const def: OmnisystemTaskDefinition = {
                type: OmnisystemTaskProvider.taskType,
                command: tpl.command,
                args: tpl.args,
            };

            const shellCmd = buildShellCommand(omniccPath, tpl.command, tpl.args);
            const task = new vscode.Task(
                def,
                vscode.TaskScope.Workspace,
                tpl.name,
                'omnisystem',
                new vscode.ShellExecution(shellCmd),
                '$omnicc',
            );

            if (tpl.group) { task.group = tpl.group; }
            if (tpl.detail) { task.detail = tpl.detail; }
            if (tpl.isBackground) { task.isBackground = true; }

            return task;
        });
    }

    private createTask(
        command: string,
        args: string[],
        name: string,
        group: vscode.TaskGroup | undefined,
    ): vscode.Task {
        const config = vscode.workspace.getConfiguration('omnisystem');
        const omniccPath = config.get<string>('omniccPath', 'omnicc').trim();

        const def: OmnisystemTaskDefinition = {
            type: OmnisystemTaskProvider.taskType,
            command,
            args,
        };

        const task = new vscode.Task(
            def,
            vscode.TaskScope.Workspace,
            name,
            'omnisystem',
            new vscode.ShellExecution(buildShellCommand(omniccPath, command, args)),
            '$omnicc',
        );

        if (group) { task.group = group; }
        return task;
    }
}

// ─── Shell command builder ────────────────────────────────────────────────────

function buildShellCommand(
    omniccPath: string,
    command: string,
    args: string[],
): string {
    const parts = [omniccPath, command, ...args];
    return parts.join(' ');
}
