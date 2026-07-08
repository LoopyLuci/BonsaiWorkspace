import * as vscode from 'vscode';
export interface OmnisystemTaskDefinition extends vscode.TaskDefinition {
    /** OmniCC sub-command, e.g. "build", "test" */
    command: string;
    /** Additional arguments appended to the command */
    args?: string[];
}
export declare class OmnisystemTaskProvider implements vscode.TaskProvider {
    static readonly taskType = "omnisystem";
    private cachedTasks;
    provideTasks(): vscode.Task[];
    resolveTask(task: vscode.Task): vscode.Task | undefined;
    private buildAllTasks;
    private createTask;
}
//# sourceMappingURL=TaskProvider.d.ts.map