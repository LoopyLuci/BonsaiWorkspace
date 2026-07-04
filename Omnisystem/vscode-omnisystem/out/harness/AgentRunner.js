"use strict";
// AgentRunner — the agentic loop that drives a model turn-by-turn to work on the
// user's project inside VS Code.
//
// It runs *in the extension host* (not on the server) so tool calls execute
// against the live editor. Model inference is delegated to the OmniHarness
// orchestrator, so ANY model works — local Ollama or any API provider.
//
// Two execution modes:
//   • native — real function calling: tool schemas are sent to the provider and
//     structured tool_calls come back (Anthropic, OpenAI-family, etc.).
//   • text   — a universal fenced ```tool JSON protocol that works with ANY model,
//     including small local models that lack native function-calling.
// The provider picks the mode; both share the same event/UI surface.
Object.defineProperty(exports, "__esModule", { value: true });
exports.AgentRunner = void 0;
const VscodeTools_1 = require("./VscodeTools");
const TOOL_BLOCK_RE = /```(?:tool|json)\s*([\s\S]*?)```/i;
class AgentRunner {
    constructor(client, tools, events, external) {
        this.client = client;
        this.tools = tools;
        this.events = events;
        this.external = external;
        this.aborted = false;
    }
    abort() { this.aborted = true; }
    /** MCP tools are offered when the agent allows all tools (tools = ['*']). */
    externalEnabled(agent) {
        return !!this.external && agent.tools.length === 1 && agent.tools[0] === '*';
    }
    /** Dispatch a tool call to the built-in VS Code tools or an external (MCP) tool. */
    async execTool(name, args) {
        if (this.external && this.external.has(name)) {
            return this.external.execute(name, args);
        }
        const r = await this.tools.execute(name, args);
        return { ok: r.ok, summary: r.summary, content: r.content ?? r.error ?? '', diff: r.diff, checkpoint: r.checkpoint };
    }
    buildSystemPrompt(agent) {
        const allowed = this.allowedTools(agent);
        const toolDocs = allowed.map((t) => {
            const params = t.params.map((p) => `      - ${p.name} (${p.type}${p.required ? ', required' : ''}): ${p.description}`).join('\n');
            return `  • ${t.name} — ${t.description}\n${params || '      (no parameters)'}`;
        }).join('\n');
        const mcpDocs = this.externalEnabled(agent) ? this.external.describe() : '';
        return [
            agent.systemPrompt.trim(),
            '',
            'You are operating as an autonomous agent inside the user\'s VS Code workspace.',
            'You can act on the project using tools. Work step by step: think, take ONE action, observe the result, then continue.',
            '',
            'AVAILABLE TOOLS:',
            toolDocs,
            mcpDocs ? '\nMCP SERVER TOOLS (call by their full name):\n' + mcpDocs : '',
            '',
            'HOW TO ACT — output exactly ONE fenced block when you want to use a tool:',
            '```tool',
            '{"tool": "read_file", "args": {"path": "src/main.ts"}}',
            '```',
            'After each tool block, STOP and wait — the tool result will be provided back to you before you continue.',
            '',
            'WHEN YOU ARE DONE (no more tools needed), output a final answer as a fenced block:',
            '```tool',
            '{"final": "A concise summary of what you did / the answer to the user."}',
            '```',
            'Only ONE block per message. Put any reasoning as plain text before the block. Never invent tool results.',
        ].join('\n');
    }
    allowedTools(agent) {
        const all = VscodeTools_1.VscodeTools.catalog();
        if (agent.tools.length === 1 && agent.tools[0] === '*') {
            return all;
        }
        const set = new Set(agent.tools);
        return all.filter((t) => set.has(t.name));
    }
    parseAction(text, agent) {
        const m = text.match(TOOL_BLOCK_RE);
        if (!m) {
            return { kind: 'none', text };
        }
        let obj;
        try {
            obj = JSON.parse(m[1].trim());
        }
        catch {
            return { kind: 'none', text };
        }
        if (typeof obj.final === 'string') {
            return { kind: 'final', text: obj.final };
        }
        if (typeof obj.tool === 'string') {
            const allowed = new Set(this.allowedTools(agent).map((t) => t.name));
            const isExternal = this.externalEnabled(agent) && this.external.has(obj.tool);
            if (!allowed.has(obj.tool) && !isExternal) {
                return { kind: 'none', text: `Tool "${obj.tool}" is not permitted for this agent.` };
            }
            return { kind: 'tool', tool: obj.tool, args: obj.args ?? {} };
        }
        return { kind: 'none', text };
    }
    /**
     * Run the agent loop. `history` is the prior conversation; `userText` is the
     * new user message. Streams assistant output and executes tool calls until the
     * model emits a final answer or `maxSteps` is reached.
     */
    async run(agent, model, history, userText, maxSteps = 16) {
        this.aborted = false;
        const system = this.buildSystemPrompt(agent);
        const messages = [...history, { role: 'user', content: userText }];
        for (let step = 0; step < maxSteps; step++) {
            if (this.aborted) {
                this.events.onStatus('Stopped.');
                return;
            }
            let full = '';
            try {
                full = await this.client.chatStream({
                    model_id: model,
                    messages,
                    system,
                    temperature: agent.temperature,
                    max_tokens: agent.maxTokens,
                }, (delta) => { if (!this.aborted) {
                    this.events.onAssistantDelta(delta);
                } });
            }
            catch (err) {
                this.events.onError(err instanceof Error ? err.message : String(err));
                return;
            }
            if (this.aborted) {
                this.events.onStatus('Stopped.');
                return;
            }
            this.events.onAssistantDone(full);
            messages.push({ role: 'assistant', content: full });
            const action = this.parseAction(full, agent);
            if (action.kind === 'final') {
                this.events.onFinal(action.text ?? '');
                return;
            }
            if (action.kind === 'none') {
                // No structured action — treat the plain text as the final answer.
                this.events.onFinal(full);
                return;
            }
            // Tool call (built-in VS Code tool or an MCP tool).
            const id = `t${Date.now().toString(36)}${step}`;
            this.events.onToolCall(id, action.tool, action.args ?? {});
            this.events.onStatus(`Running ${action.tool}…`);
            const result = await this.execTool(action.tool, action.args ?? {});
            this.events.onToolResult(id, result);
            const observation = result.ok
                ? `TOOL RESULT (${action.tool}):\n${result.content || '(ok)'}`
                : `TOOL ERROR (${action.tool}): ${result.content || 'failed'}`;
            messages.push({ role: 'user', content: observation });
        }
        this.events.onStatus(`Reached step limit (${maxSteps}).`);
        this.events.onFinal('Reached the maximum number of steps. Ask me to continue if more work is needed.');
    }
    /**
     * Native function-calling loop. Sends tool schemas to the provider and acts on
     * the structured tool_calls it returns. Non-streaming per step (the model
     * returns tool_calls as data, not tokens), but shares the same UI events.
     */
    async runNative(agent, model, history, userText, maxSteps = 16) {
        this.aborted = false;
        const system = this.buildNativeSystemPrompt(agent);
        const schemas = [
            ...VscodeTools_1.VscodeTools.toFunctionSchemas(agent.tools),
            ...(this.externalEnabled(agent) ? this.external.functionSchemas() : []),
        ];
        const messages = [...history, { role: 'user', content: userText }];
        for (let step = 0; step < maxSteps; step++) {
            if (this.aborted) {
                this.events.onStatus('Stopped.');
                return;
            }
            this.events.onStatus('Thinking…');
            let res;
            try {
                res = await this.client.chat({
                    model_id: model,
                    messages,
                    system,
                    temperature: agent.temperature,
                    max_tokens: agent.maxTokens,
                    tools: schemas,
                });
            }
            catch (err) {
                this.events.onError(err instanceof Error ? err.message : String(err));
                return;
            }
            if (this.aborted) {
                this.events.onStatus('Stopped.');
                return;
            }
            // Surface any assistant narration.
            if (res.content) {
                this.events.onAssistantDelta(res.content);
                this.events.onAssistantDone(res.content);
            }
            const calls = res.tool_calls ?? [];
            if (calls.length === 0) {
                this.events.onFinal(res.content || '');
                return;
            }
            // Record the assistant turn (with its tool_calls) so the provider
            // accepts the following tool-result messages.
            messages.push({ role: 'assistant', content: res.content || '', tool_calls: calls });
            for (const call of calls) {
                if (this.aborted) {
                    this.events.onStatus('Stopped.');
                    return;
                }
                this.events.onToolCall(call.id, call.name, call.arguments || {});
                this.events.onStatus(`Running ${call.name}…`);
                const result = await this.execTool(call.name, call.arguments || {});
                this.events.onToolResult(call.id, result);
                const observation = result.ok ? (result.content || '(ok)') : `ERROR: ${result.content || 'failed'}`;
                messages.push({ role: 'tool', tool_call_id: call.id, content: observation });
            }
        }
        this.events.onStatus(`Reached step limit (${maxSteps}).`);
        this.events.onFinal('Reached the maximum number of steps. Ask me to continue if more work is needed.');
    }
    buildNativeSystemPrompt(agent) {
        return [
            agent.systemPrompt.trim(),
            '',
            'You are operating as an autonomous agent inside the user\'s VS Code workspace.',
            'Use the provided tools to inspect and modify the project. Read files before editing them.',
            'Work step by step and stop calling tools once the task is complete, then give a concise summary.',
        ].join('\n');
    }
    /** Simple non-agentic single-turn streaming chat (for the "Ask" quick path). */
    async chatOnce(model, system, history, userText, temperature, maxTokens) {
        this.aborted = false;
        const messages = [...history, { role: 'user', content: userText }];
        try {
            const full = await this.client.chatStream({ model_id: model, messages, system, temperature, max_tokens: maxTokens }, (delta) => { if (!this.aborted) {
                this.events.onAssistantDelta(delta);
            } });
            this.events.onAssistantDone(full);
            this.events.onFinal(full);
        }
        catch (err) {
            this.events.onError(err instanceof Error ? err.message : String(err));
        }
    }
}
exports.AgentRunner = AgentRunner;
//# sourceMappingURL=AgentRunner.js.map