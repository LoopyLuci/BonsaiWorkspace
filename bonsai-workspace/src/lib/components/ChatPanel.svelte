<script lang="ts">
  import { afterUpdate, onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import DOMPurify from 'dompurify';
  import {
    messages, addUserMessage, addAssistantMessage,
    permissionCards, addPermissionCard, removePermissionCard,
    isThinking, tokenSpeed,
    currentSessionId, currentSessionTitle, setCurrentSession,
    clearCurrentSession, clearChat,
    askBonsaiRequest,
  } from '$lib/stores/chat';
  import { addToast } from '$lib/stores/toast';
  import { activeEditorFile } from '$lib/stores/activeEditorFile';
  import { requestOpenFile } from '$lib/stores/openFile';
  import { receiveAgentDiff, clearCurrentDiff, clearDiffForFile } from '$lib/stores/diff';

  const dispatch = createEventDispatcher<{ openSession: void }>();

  function openSessionPanel() {
    dispatch('openSession');
  }
  import { currentWorkspace, fileTreeRefresh } from '$lib/stores/workspace';
  import { modelSwitchStatus } from '$lib/stores/models';
  import ModelSelector from '$lib/components/ModelSelector.svelte';

  let input       = '';
  let isRecording = false;
  let errorMsg    = '';
  let scrollEl:   HTMLDivElement;
  let stopRequested = false;
  let voiceStopRequested = false;
  let isSpeechActive = false;
  let speechStatusTimer: ReturnType<typeof setInterval> | null = null;
  let showToolSkills = false;

  type ToolInfo = {
    name: string;
    description: string;
    requires_approval: boolean;
    is_custom: boolean;
  };

  type SkillInfo = {
    id: string;
    name: string;
    description: string;
    tools: string[];
  };

  const TOOL_PREFS_KEY = 'bonsai-tool-prefs-v1';
  const SKILL_PREFS_KEY = 'bonsai-skill-prefs-v1';

  const SKILLS: SkillInfo[] = [
    {
      id: 'codebase',
      name: 'Codebase Discovery',
      description: 'Read/search/list files to understand the workspace before changing code.',
      tools: ['read_file', 'list_files', 'list_all_files', 'search_files', 'grep_files'],
    },
    {
      id: 'editing',
      name: 'Code Editing',
      description: 'Create, edit, and delete files with human approval where required.',
      tools: ['write_file', 'edit_file', 'create_dir', 'delete_file'],
    },
    {
      id: 'terminal',
      name: 'Terminal Automation',
      description: 'Run shell commands in the workspace (always approval-gated).',
      tools: ['run_command'],
    },
  ];

  let availableTools: ToolInfo[] = [];
  let toolEnabled: Record<string, boolean> = {};
  let skillEnabled: Record<string, boolean> = {};

  // ── Streaming / inference state ───────────────────────────────────────────────
  let rawBuffer      = '';    // all tokens as they arrive
  let streamThinking = '';    // content inside <think>...</think>
  let streamResponse = '';    // content after </think>
  let streamingToolCallOnly = false; // true when stream currently contains only tool_call XML
  let thinkingDone   = false; // true once </think> has been seen
  let isStreaming    = false;  // true while tokens are arriving

  /** Tool the model is currently executing (shown as a live indicator). */
  let liveToolStatus = '';
  /** Tools that have already completed this inference turn (shown as history). */
  let completedTools: string[] = [];
  /** When true the open editor file is injected as context on every send. */
  let includeFileContext = true;
  /** Debounce timer for auto-save to avoid rapid double-writes. */
  let saveDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  let unlistenEvents: Array<() => void> = [];

  // Auto-scroll on new messages or streaming update
  afterUpdate(() => {
    if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  });

  /** Parse rawBuffer into thinking vs response parts. */
  function parseBuffer() {
    const visibleBuffer = stripToolCallBlocks(rawBuffer);
    const OPEN  = '<think>';
    const CLOSE = '</think>';
    if (visibleBuffer.startsWith(OPEN)) {
      const closeIdx = visibleBuffer.indexOf(CLOSE);
      if (closeIdx !== -1) {
        streamThinking = visibleBuffer.slice(OPEN.length, closeIdx);
        streamResponse = visibleBuffer.slice(closeIdx + CLOSE.length).replace(/^\n+/, '');
        thinkingDone   = true;
      } else {
        streamThinking = visibleBuffer.slice(OPEN.length);
        streamResponse = '';
        thinkingDone   = false;
      }
    } else {
      streamThinking = '';
      streamResponse = visibleBuffer;
      thinkingDone   = true;
    }

    streamingToolCallOnly = /<\s*tool_call\s*>/i.test(rawBuffer)
      && streamResponse.trim().length === 0;
  }

  // ── Inference helpers ────────────────────────────────────────────────────────

  /** Reset all streaming state and set isThinking. */
  function beginInference() {
    isThinking.set(true);
    rawBuffer = ''; streamThinking = ''; streamResponse = '';
    streamingToolCallOnly = false;
    thinkingDone = false; isStreaming = false; liveToolStatus = '';
    completedTools = [];
  }

  /** Clean up after inference completes. */
  function endInference() {
    isThinking.set(false);
    tokenSpeed.set(0);
    isStreaming = false; rawBuffer = ''; streamThinking = ''; streamResponse = '';
    streamingToolCallOnly = false;
    liveToolStatus = '';
  }

  /**
   * Run submit_chat with the current message history and return the result.
   * Registers a token-stream listener for the duration and tears it down after.
   * Trims to the last CONTEXT_LIMIT messages to avoid context-window overflow.
   */
  async function runChat(extraMessages: Array<{ role: string; content: string }> = []) {
    const unlistenStream = await listen<string>('token-stream', (e) => {
      rawBuffer += e.payload;
      parseBuffer();
      isStreaming = true;
    });

    function buildHistory(
      contextLimit: number,
      includeFileCtx: boolean,
    ): Array<{ role: string; content: string }> {
      const msgs = $messages.slice(-contextLimit);
      const FILE_CHAR_LIMIT = 1200;
      const fileCtx: Array<{ role: string; content: string }> =
        (includeFileCtx && includeFileContext && $activeEditorFile)
          ? [
              {
                role: 'user',
                content:
                  `[Open file: \`${$activeEditorFile.path}\`]\n` +
                  '```\n' +
                  $activeEditorFile.content.slice(0, FILE_CHAR_LIMIT) +
                  ($activeEditorFile.content.length > FILE_CHAR_LIMIT ? '\n… (truncated)' : '') +
                  '\n```',
              },
              { role: 'assistant', content: 'I can see the open file. Ready to help.' },
            ]
          : [];

      return [
        ...fileCtx,
        ...msgs.map((msg) => ({ role: msg.role, content: msg.content })),
        ...extraMessages,
      ];
    }

    try {
      const history = buildHistory(14, true);
      return await invoke<{
        content:        string;
        stats:          { prompt_tokens: number; completion_tokens: number; tokens_per_second: number; time_to_first_token_ms: number; total_time_ms: number };
        action_handled: boolean;
        tools_used:     string[];
      }>('submit_chat', {
        messages:      history,
        workspacePath: $currentWorkspace?.path,
        enabledTools:  getEnabledToolNames(),
      });
    } catch (e) {
      const msg = String(e);
      // Some llama-server builds return 400 when context payload is too large.
      // Retry once with shorter history and without file-context injection.
      if (msg.includes('HTTP 400')) {
        const fallbackHistory = buildHistory(4, false);
        return await invoke<{
          content:        string;
          stats:          { prompt_tokens: number; completion_tokens: number; tokens_per_second: number; time_to_first_token_ms: number; total_time_ms: number };
          action_handled: boolean;
          tools_used:     string[];
        }>('submit_chat', {
          messages:      fallbackHistory,
          workspacePath: $currentWorkspace?.path,
          enabledTools:  getEnabledToolNames(),
        });
      }
      throw e;
    } finally {
      unlistenStream();
    }
  }

  async function send() {
    const text = input.trim();
    if (!text || $isThinking) return;
    await sendText(text);
  }

  async function sendText(text: string) {
    const noWorkspace = !$currentWorkspace?.path;
    const looksLikeFileListingRequest = /(list|show|display|enumerate)\s+.*(files|directory|folder)|\b(list files|readme|read me|read file|read the file|show files)\b/i.test(text);
    if (noWorkspace && looksLikeFileListingRequest) {
      addUserMessage(text);
      addAssistantMessage('No workspace folder is open yet. Use Open Folder in the left pane, then ask again and I will list/read files with tools.');
      addToast('Open a folder first to use file tools.', 'info');
      return;
    }

    stopRequested = false;
    addUserMessage(text);
    input = '';
    errorMsg = '';
    beginInference();

    // Save before the AI turn so the user message is always persisted.
    debouncedAutoSave();

    try {
      const result = await runChat();
      const clean  = stripThinkTags(result.content ?? rawBuffer);
      if (!result.action_handled) {
        addAssistantMessage(clean, result.stats ?? undefined, result.tools_used ?? []);
        // Save again after the AI responds so the assistant message is captured.
        debouncedAutoSave();
      }
    } catch (e) {
      const msg = String(e);
      const cancelled = stopRequested && msg.toLowerCase().includes('cancel');
      if (!cancelled) {
        errorMsg = `Chat error: ${msg}`;
      }
      const visible = stripToolCallBlocks(streamResponse).trim();
      if (visible) {
        addAssistantMessage(cancelled ? `${visible}\n\n⏹ Response stopped.` : visible);
      } else if (cancelled) {
        addAssistantMessage('⏹ Response stopped.');
      }
    } finally {
      endInference();
      rawBuffer      = '';
      streamThinking = '';
      streamResponse = '';
    }
  }

  async function autoSaveSession() {
    const history = $messages.map((msg) => ({ role: msg.role, content: msg.content }));
    const firstUserMsg = history.find((m) => m.role === 'user')?.content ?? '';
    const title = $currentSessionTitle?.trim()
      || firstUserMsg.slice(0, 60).trim()
      || `Chat ${new Date().toLocaleDateString()}`;

    try {
      const result = await invoke<{ id: string }>('save_chat_session', {
        sessionId: $currentSessionId || undefined,
        title,
        workspacePath: $currentWorkspace?.path,
        messages: history,
      });

      if (result?.id) {
        if (!$currentSessionId) {
          setCurrentSession(result.id, title);
          addToast('Saved new chat session.', 'success');
        } else {
          setCurrentSession(result.id, title);
        }
      }
    } catch (e) {
      console.warn('Auto-save failed:', e);
    }
  }

  /** Debounced auto-save — coalesces rapid back-to-back calls into one write. */
  function debouncedAutoSave() {
    if (saveDebounceTimer) clearTimeout(saveDebounceTimer);
    saveDebounceTimer = setTimeout(() => {
      void autoSaveSession();
      saveDebounceTimer = null;
    }, 400);
  }

  /** Start a fresh conversation, detaching from any active session. */
  function newChat() {
    clearChat();
    clearCurrentSession();
    input    = '';
    errorMsg = '';
  }

  /** Strip <think>...</think> block (and optional leading newlines after it). */
  function stripThinkTags(text: string): string {
    return text.replace(/^<think>[\s\S]*?<\/think>\n*/,'').trim();
  }

  /** Remove tool_call blocks from display text, including unfinished trailing blocks. */
  function stripToolCallBlocks(text: string): string {
    const complete = text.replace(/<\s*tool_call\s*>[\s\S]*?<\s*\/\s*tool_call\s*>/gi, '');
    return complete.replace(/<\s*tool_call\s*>[\s\S]*$/i, '').trimEnd();
  }

  async function startVoice() {
    if ($isThinking) return;

    if (isRecording) {
      await stopVoiceCapture();
      return;
    }

    voiceStopRequested = false;
    isRecording = true;
    errorMsg    = '';
    try {
      const transcript = await invoke<string>('voice_transcribe');
      if (transcript) {
        input = transcript;
        // Optionally auto-send:
        // await send();
      }
    } catch (e) {
      const msg = String(e);
      if (!(voiceStopRequested && msg.toLowerCase().includes('cancel'))) {
        errorMsg = `Voice error: ${msg}`;
      }
    } finally {
      isRecording = false;
      voiceStopRequested = false;
    }
  }

  async function stopVoiceCapture() {
    voiceStopRequested = true;
    try {
      await invoke('stop_voice_capture');
    } catch (e) {
      errorMsg = `Voice stop error: ${e}`;
    }
  }

  function stopSpeechPlayback() {
    if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
      window.speechSynthesis.cancel();
      isSpeechActive = false;
    }
  }

  async function stopResponseGeneration() {
    stopRequested = true;
    stopSpeechPlayback();
    try {
      await invoke('stop_chat_generation');
    } catch (e) {
      errorMsg = `Stop error: ${e}`;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function getEnabledToolNames(): string[] {
    return availableTools
      .filter((tool) => toolEnabled[tool.name] !== false)
      .map((tool) => tool.name);
  }

  function persistToolPrefs() {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(TOOL_PREFS_KEY, JSON.stringify(toolEnabled));
    window.localStorage.setItem(SKILL_PREFS_KEY, JSON.stringify(skillEnabled));
  }

  function loadPersistedPrefs() {
    if (typeof window === 'undefined') return;
    try {
      const rawTools = window.localStorage.getItem(TOOL_PREFS_KEY);
      if (rawTools) {
        const parsed = JSON.parse(rawTools) as Record<string, boolean>;
        toolEnabled = { ...toolEnabled, ...parsed };
      }
      const rawSkills = window.localStorage.getItem(SKILL_PREFS_KEY);
      if (rawSkills) {
        const parsed = JSON.parse(rawSkills) as Record<string, boolean>;
        skillEnabled = { ...skillEnabled, ...parsed };
      }
    } catch {
      // Ignore malformed persisted settings.
    }
  }

  function toggleTool(name: string) {
    toolEnabled[name] = !(toolEnabled[name] ?? true);
    toolEnabled = { ...toolEnabled };
    persistToolPrefs();
  }

  function toggleSkill(skill: SkillInfo) {
    const next = !(skillEnabled[skill.id] ?? true);
    skillEnabled[skill.id] = next;
    skillEnabled = { ...skillEnabled };

    for (const toolName of skill.tools) {
      if (toolName in toolEnabled) {
        toolEnabled[toolName] = next;
      }
    }
    toolEnabled = { ...toolEnabled };
    persistToolPrefs();
  }

  async function loadToolCatalog() {
    try {
      availableTools = await invoke<ToolInfo[]>('list_available_chat_tools', {
        workspacePath: $currentWorkspace?.path,
      });

      const defaults: Record<string, boolean> = {};
      for (const tool of availableTools) {
        defaults[tool.name] = true;
      }
      toolEnabled = defaults;

      const skillDefaults: Record<string, boolean> = {};
      for (const skill of SKILLS) {
        skillDefaults[skill.id] = true;
      }
      skillEnabled = skillDefaults;

      loadPersistedPrefs();
      persistToolPrefs();
    } catch (e) {
      console.warn('Unable to load tools catalog:', e);
    }
  }

  onMount(async () => {
    await loadToolCatalog();

    const handleGlobalClick = (event: MouseEvent) => {
      void applyCodeBlockToEditor(event);
    };
    document.addEventListener('click', handleGlobalClick);

    if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
      speechStatusTimer = setInterval(() => {
        isSpeechActive = window.speechSynthesis.speaking;
      }, 250);
    }

    const unlistenPermission = await listen<any>('permission-request', (e) => {
      if (e.payload.tool === 'write_file' && e.payload.file_path && e.payload.unified_diff) {
        requestOpenFile(e.payload.file_path);
        receiveAgentDiff(e.payload.file_path, e.payload.unified_diff);
      }

      addPermissionCard({
        type:           e.payload.type ?? 'tool_approval',
        description:    e.payload.description ?? e.payload.rationale ?? 'Approve tool execution',
        rationale:      e.payload.rationale ?? 'The model requested permission to run a tool.',
        paths_affected: e.payload.paths_affected ?? [],
        command:        e.payload.command,
        action:         e.payload.action,
        tool:           e.payload.tool,
        args:           e.payload.args,
        // carry the full ctx snapshot so we can continue after approval
        raw_response:   e.payload.raw_response,
        ctx_snapshot:   e.payload.ctx_snapshot,
        file_path:      e.payload.file_path,
        unified_diff:   e.payload.unified_diff,
      });
    });

    const unlistenToolUsed = await listen<{ tool: string; output: string }>('tool-used', (e) => {
      // Move the completed tool into history and briefly show "done" before the next turn.
      completedTools = [...completedTools, e.payload.tool];
      liveToolStatus = `✅ \`${e.payload.tool}\` done — thinking…`;
      // Clear the transient status after 1.2 s (next token-stream will take over).
      setTimeout(() => { liveToolStatus = ''; }, 1200);
      // Refresh the file tree if the tool mutated the filesystem.
      if (['write_file', 'edit_file', 'create_dir', 'delete_file'].includes(e.payload.tool)) {
        fileTreeRefresh.set(Date.now());
      }
    });

    const unlistenToken = await listen<number>('token-speed', (e) => {
      tokenSpeed.set(e.payload);
    });

    const unsubAskBonsai = askBonsaiRequest.subscribe(async (request) => {
      if (!request) return;

      if ($isThinking) {
        input = request.prompt;
        addToast('Queued prompt in input while current response finishes.', 'info');
        askBonsaiRequest.set(null);
        return;
      }

      await sendText(request.prompt);
      askBonsaiRequest.set(null);
    });

    unlistenEvents = [
      () => document.removeEventListener('click', handleGlobalClick),
      unlistenPermission,
      unlistenToolUsed,
      unlistenToken,
      unsubAskBonsai,
    ];
  });

  onDestroy(() => {
    if (saveDebounceTimer) {
      clearTimeout(saveDebounceTimer);
      saveDebounceTimer = null;
    }
    if (speechStatusTimer) clearInterval(speechStatusTimer);
    unlistenEvents.forEach((u) => u());
  });

  /** Minimal, safe markdown → HTML. */
  function renderMarkdown(text: string): string {
    const codeBlocks: Array<{ lang: string; raw: string }> = [];
    const withPlaceholders = text.replace(/```(\w*)\n([\s\S]*?)```/g, (_m, lang = '', code = '') => {
      const index = codeBlocks.push({ lang, raw: code }) - 1;
      return `@@CODE_BLOCK_${index}@@`;
    });

    const escaped = withPlaceholders
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    const html = escaped
      // Inline code
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      // Bold
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      // Italic
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      // Links
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>')
      // Line breaks
      .replace(/\n/g, '<br>')
      // Restore code blocks with an apply button that routes through diff preview
      .replace(/@@CODE_BLOCK_(\d+)@@/g, (_m, idx) => {
        const block = codeBlocks[Number(idx)];
        if (!block) return '';
        const escapedCode = block.raw
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;');
        const encodedCode = encodeURIComponent(block.raw);
        return `<div class="code-block-wrap"><div class="code-block-actions"><button type="button" class="apply-code-btn" data-code="${encodedCode}" data-lang="${block.lang}">Apply to editor</button></div><pre><code class="lang-${block.lang}">${escapedCode}</code></pre></div>`;
      });

    return DOMPurify.sanitize(html, {
      ALLOWED_TAGS: ['strong','em','code','pre','a','br','span','div','button'],
      ALLOWED_ATTR: ['class','href','target','rel','type','data-code','data-lang'],
    });
  }

  async function applyCodeBlockToEditor(event: MouseEvent) {
    const target = (event.target as HTMLElement).closest('.apply-code-btn') as HTMLButtonElement | null;
    if (!target) return;

    const encodedCode = target.dataset.code;
    if (!encodedCode) return;

    if (!$activeEditorFile?.path) {
      addToast('Open a file in the editor first.', 'info');
      return;
    }

    const code = decodeURIComponent(encodedCode);
    const filePath = $activeEditorFile.path;

    try {
      requestOpenFile(filePath);
      const unifiedDiff = await invoke<string>('create_unified_diff', {
        filePath,
        newContent: code,
      });
      receiveAgentDiff(filePath, unifiedDiff);
      addToast('Code loaded into diff preview.', 'success');
    } catch (e) {
      addAssistantMessage(`❌ Could not prepare editor diff: ${e}`);
    }
  }

  function formatTime(d: Date) {
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  // ── Permission card actions ───────────────────────────────────────────────

  async function approveCard(card: typeof $permissionCards[number]) {
    removePermissionCard(card.id);
    try {
      if (card.type === 'tool_approval' && card.action) {
        if (card.tool === 'write_file' && card.file_path) {
          clearDiffForFile(card.file_path);
          clearCurrentDiff();
        }

        if (!card.ctx_snapshot || !card.raw_response) {
          // Fallback path for legacy cards.
          const toolOutput = await invoke<string>('execute_tool_call', {
            action:        card.action,
            workspacePath: $currentWorkspace?.path,
          });
          addAssistantMessage(`**🔧 \`${card.tool}\` result:**\n\n\`\`\`\n${toolOutput}\n\`\`\``);
          return;
        }

        await continueAfterHITL(card, true);
        if (['write_file', 'delete_file', 'create_dir', 'edit_file'].includes(card.tool ?? '')) {
          fileTreeRefresh.set(Date.now());
        }

      } else if (card.type === 'shell_command' && card.command) {
        await invoke('run_terminal_command', { command: card.command });
        addAssistantMessage(`✅ Executed: \`${card.command}\``);
      } else if (card.type === 'file_delete' && card.paths_affected?.length) {
        for (const p of card.paths_affected) {
          await invoke('delete_file', { path: p });
        }
        fileTreeRefresh.set(Date.now());
        addAssistantMessage(`✅ Deleted: ${card.paths_affected.map((p) => `\`${p}\``).join(', ')}`);
      } else {
        addAssistantMessage(`✅ Approved: ${card.description ?? card.rationale}`);
      }
    } catch (e) {
      addAssistantMessage(`❌ Action failed: ${e}`);
    }
  }

  /**
   * Resume the model after a HITL decision through the backend continuation command.
   */
  async function continueAfterHITL(
    card:     typeof $permissionCards[number],
    approved: boolean,
  ) {
    stopRequested = false;
    errorMsg = '';
    beginInference();
    const unlistenStream = await listen<string>('token-stream', (e) => {
      rawBuffer += e.payload;
      parseBuffer();
      isStreaming = true;
    });

    try {
      const result = await invoke<{
        content:        string;
        stats:          { prompt_tokens: number; completion_tokens: number; tokens_per_second: number; time_to_first_token_ms: number; total_time_ms: number };
        action_handled: boolean;
        tools_used:     string[];
      }>('resume_tool_call', {
        ctxSnapshot:   card.ctx_snapshot ?? [],
        rawResponse:   card.raw_response ?? '',
        action:        card.action ?? {},
        approved,
        workspacePath: $currentWorkspace?.path,
        enabledTools:  getEnabledToolNames(),
      });

      const clean = stripThinkTags(result.content ?? rawBuffer).trim();
      if (clean) addAssistantMessage(clean, result.stats ?? undefined, result.tools_used ?? []);
      debouncedAutoSave();
      // If action_handled again, another permission card will appear via the event.
    } catch (e) {
      const msg = String(e);
      const cancelled = stopRequested && msg.toLowerCase().includes('cancel');
      if (!cancelled) {
        errorMsg = `Tool continuation error: ${msg}`;
      }
      const visible = stripToolCallBlocks(streamResponse).trim();
      if (visible) {
        addAssistantMessage(cancelled ? `${visible}\n\n⏹ Response stopped.` : visible);
      } else if (cancelled) {
        addAssistantMessage('⏹ Response stopped.');
      }
    } finally {
      unlistenStream();
      endInference();
    }
  }

  async function denyCard(card: typeof $permissionCards[number]) {
    if (card.file_path) {
      clearDiffForFile(card.file_path);
      clearCurrentDiff();
    }
    removePermissionCard(card.id);
    if (card.type === 'tool_approval' && card.action && card.ctx_snapshot && card.raw_response) {
      await continueAfterHITL(card, false);
      return;
    }
    addAssistantMessage(`🚫 Denied: ${card.description ?? card.rationale}`);
  }
</script>

<div class="chat-panel">
  <!-- Message list -->
  <div class="chat-header">
    <div class="chat-header-left">
      <span class="chat-title">Chat</span>
      {#if $currentSessionTitle}
        <span class="chat-session-tag">{$currentSessionTitle}</span>
      {/if}
      {#if $activeEditorFile}
        <button
          class="file-ctx-badge"
          class:active={includeFileContext}
          on:click={() => (includeFileContext = !includeFileContext)}
          title={includeFileContext ? 'File context ON — click to exclude' : 'File context OFF — click to include'}
        >
          📎 {$activeEditorFile.path.split(/[/\\]/).pop()}
        </button>
      {/if}
    </div>
    <div class="chat-header-actions">
      <button class="btn-new-chat" on:click={newChat} title="New chat (clears current conversation)">＋ New</button>
      <button class="btn-tools" on:click={() => (showToolSkills = true)} aria-label="Open tools and skills">
        Tools/Skills
      </button>
      <button class="btn-session" on:click={openSessionPanel} aria-label="Open session manager">Sessions</button>
    </div>
  </div>
  <div class="messages" bind:this={scrollEl} aria-live="polite" aria-label="Chat messages">
    {#if $messages.length === 0 && !$isThinking}
      <div class="empty-chat">
        <div class="empty-icon">💬</div>
        <div>Ask Bonsai anything about your code</div>
        <div class="empty-hint">Shift+Enter for newline</div>
      </div>
    {:else}
      {#each $messages as msg (msg.id)}
        <div class="msg-row {msg.role}">
          <div class="msg-bubble">
            {#if msg.role === 'assistant'}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html renderMarkdown(msg.content)}
            {:else}
              {msg.content}
            {/if}
          </div>

          {#if msg.role === 'assistant'}
            <div class="msg-meta">
              <span class="msg-time">{formatTime(msg.timestamp)}</span>
              {#if msg.tools_used?.length}
                <span class="tool-badges">
                  {#each msg.tools_used as t}
                    <span class="tool-badge">🔧 {t}</span>
                  {/each}
                </span>
              {/if}
              {#if msg.stats}
                <span class="msg-stats">
                  {msg.stats.completion_tokens} tok
                  · {msg.stats.tokens_per_second.toFixed(1)} tok/s
                  · {msg.stats.total_time_ms < 1000
                      ? `${msg.stats.total_time_ms}ms`
                      : `${(msg.stats.total_time_ms / 1000).toFixed(1)}s`}
                  {#if msg.stats.time_to_first_token_ms > 0}
                    · TTFT {msg.stats.time_to_first_token_ms}ms
                  {/if}
                </span>
              {/if}
            </div>
          {:else}
            <div class="msg-time">{formatTime(msg.timestamp)}</div>
          {/if}
        </div>
      {/each}

      {#if $isThinking}
        <div class="msg-row assistant">
          {#if completedTools.length}
            <div class="completed-tools">
              {#each completedTools as t}
                <span class="completed-tool">✅ {t}</span>
              {/each}
            </div>
          {/if}
          {#if liveToolStatus}
            <div class="live-tool">{liveToolStatus}</div>
          {/if}
          {#if streamThinking}
            <details class="think-block" open={!thinkingDone}>
              <summary class="think-summary">
                {thinkingDone ? 'Thought' : 'Thinking…'}
              </summary>
              <div class="think-content">{streamThinking}</div>
            </details>
          {/if}
          <div class="msg-bubble" class:thinking={!streamResponse && !isStreaming}>
            {#if streamResponse}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html renderMarkdown(streamResponse)}
            {:else if isStreaming && streamingToolCallOnly}
              <span class="stream-placeholder">Preparing tool call…</span>
            {:else if !isStreaming}
              <span class="dot"></span><span class="dot"></span><span class="dot"></span>
            {/if}
          </div>
        </div>
      {/if}
    {/if}

    <!-- Permission cards -->
    {#each $permissionCards as card (card.id)}
      {@const isDanger = ['file_delete','shell_command','delete_file','run_command'].includes(card.type ?? card.tool ?? '')}
      <div class="perm-card" class:danger={isDanger}>
        <div class="perm-header">
          <span class="perm-icon">
            {#if card.tool === 'write_file' || card.tool === 'create_dir'}✏️
            {:else if card.tool === 'delete_file'}🗑
            {:else if card.tool === 'run_command'}⚡
            {:else if card.tool === 'read_file'}📄
            {:else if card.tool === 'list_files'}📁
            {:else if card.tool === 'search_files'}🔍
            {:else if card.type === 'shell_command'}⚡
            {:else}🔐{/if}
          </span>
          <span class="perm-title">{card.description ?? card.rationale}</span>
        </div>

        {#if card.args && Object.keys(card.args).length > 0}
          <div class="perm-args">
            {#each Object.entries(card.args) as [k, v]}
              <div class="perm-arg-row">
                <span class="perm-arg-key">{k}</span>
                <code class="perm-arg-val">{String(v).length > 120 ? String(v).slice(0, 120) + '…' : v}</code>
              </div>
            {/each}
          </div>
        {:else if card.paths_affected?.length}
          <div class="perm-paths">
            {#each card.paths_affected as p}
              <code class="perm-path">{p}</code>
            {/each}
          </div>
        {/if}

        {#if card.command}
          <pre class="perm-cmd">{card.command}</pre>
        {/if}

        {#if card.tool === 'write_file' && card.unified_diff}
          <div class="perm-diff-hint">Diff preview loaded in editor. Accept/reject hunks in Monaco, then approve here to continue.</div>
        {/if}

        <div class="perm-actions">
          <button class="btn-approve" on:click={() => approveCard(card)}>✓ Approve</button>
          <button class="btn-deny"    on:click={() => denyCard(card)}>✕ Deny</button>
        </div>
      </div>
    {/each}
  </div>

  <!-- Error banner -->
  {#if errorMsg}
    <div class="error-bar" role="alert">
      {errorMsg}
      <button on:click={() => (errorMsg = '')}>✕</button>
    </div>
  {/if}

  {#if $modelSwitchStatus}
    <div class="model-progress-badge">
      <span>🔄 { $modelSwitchStatus }</span>
    </div>
  {/if}
  {#if $isThinking}
    <div class="response-status">
      <span class="spinner"></span>
      {#if $tokenSpeed > 0}
        <span>Streaming response…</span>
        <span class="status-detail">{Math.round($tokenSpeed)} tok/s</span>
      {:else}
        <span>Waiting for model…</span>
        <span class="status-detail">inference may take a moment</span>
      {/if}
      <button class="btn-stop" on:click={stopResponseGeneration} aria-label="Stop response generation">Stop</button>
    </div>
  {/if}

  <!-- Input area -->
  <div class="input-area">
    <textarea
      class="chat-input"
      bind:value={input}
      on:keydown={handleKeydown}
      placeholder="Message Bonsai… (Enter to send, Shift+Enter for newline)"
      rows={3}
      disabled={$isThinking}
      aria-label="Chat input"
    ></textarea>
    <div class="input-actions">
      <ModelSelector inline={true} />
      <button
        class="btn-send"
        on:click={send}
        disabled={$isThinking || !input.trim()}
        aria-label="Send message"
      >
        {$isThinking ? '…' : '↑ Send'}
      </button>
      <button
        class="btn-voice"
        on:click={startVoice}
        disabled={$isThinking}
        aria-label={isRecording ? 'Recording voice…' : 'Start voice input'}
        class:recording={isRecording}
      >
        {isRecording ? '⏹ Stop' : '🎤'}
      </button>
      <button
        class="btn-tts"
        on:click={stopSpeechPlayback}
        disabled={!isSpeechActive}
        aria-label="Stop text to speech"
      >
        ⏹ TTS
      </button>
    </div>
  </div>

  {#if showToolSkills}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="tools-overlay" on:click|self={() => (showToolSkills = false)} role="presentation">
      <div class="tools-panel" role="dialog" aria-modal="true" aria-label="Tools and skills">
        <div class="tools-header">
          <h3>Tools and Skills</h3>
          <button class="tools-close" on:click={() => (showToolSkills = false)} aria-label="Close tools and skills">✕</button>
        </div>

        <div class="tools-section">
          <div class="tools-section-title">Skills</div>
          {#each SKILLS as skill}
            <label class="toggle-row">
              <div class="toggle-copy">
                <div class="toggle-name">{skill.name}</div>
                <div class="toggle-desc">{skill.description}</div>
              </div>
              <input type="checkbox" checked={skillEnabled[skill.id] ?? true} on:change={() => toggleSkill(skill)} />
            </label>
          {/each}
        </div>

        <div class="tools-section">
          <div class="tools-section-title">Tools</div>
          {#if availableTools.length === 0}
            <div class="tools-empty">No tools discovered for this workspace yet.</div>
          {:else}
            {#each availableTools as tool}
              <label class="toggle-row">
                <div class="toggle-copy">
                  <div class="toggle-name">
                    {tool.name}
                    {#if tool.requires_approval}<span class="tool-chip">approval</span>{/if}
                    {#if tool.is_custom}<span class="tool-chip custom">custom</span>{/if}
                  </div>
                  <div class="toggle-desc">{tool.description}</div>
                </div>
                <input type="checkbox" checked={toolEnabled[tool.name] ?? true} on:change={() => toggleTool(tool.name)} />
              </label>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .chat-panel {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg2);
    border-left: 1px solid var(--border);
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    scroll-behavior: smooth;
  }



  .chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 0;
    gap: 12px;
  }

  .chat-header-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .chat-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
  }

  .chat-session-tag {
    background: rgba(34, 197, 94, 0.12);
    border: 1px solid rgba(34, 197, 94, 0.2);
    border-radius: 999px;
    color: var(--text);
    padding: 5px 10px;
    font-size: 12px;
  }

  .chat-header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .btn-new-chat {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 999px;
    padding: 6px 12px;
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
  }
  .btn-new-chat:hover { background: var(--bg-hover); color: var(--text); }

  .btn-session {
    background: var(--bg);
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 8px 14px;
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
  }

  .btn-session:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text);
  }

  .btn-tools {
    background: transparent;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 8px 12px;
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
  }
  .btn-tools:hover { background: var(--bg-hover); color: var(--text); }

  /* Active file context badge — toggleable */
  .file-ctx-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    padding: 3px 9px;
    border-radius: 999px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text-dim);
    white-space: nowrap;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }
  .file-ctx-badge.active {
    background: rgba(34, 197, 94, 0.1);
    border-color: rgba(34, 197, 94, 0.35);
    color: var(--accent-hl);
  }
  .file-ctx-badge:hover { opacity: 0.85; }

  .empty-chat {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    color: var(--text-dim);
    font-size: 13px;
    text-align: center;
  }
  .empty-icon { font-size: 32px; }
  .empty-hint {
    font-size: 11px;
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: 6px;
    margin-top: 4px;
  }

  .msg-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .msg-row.user   { align-items: flex-end; }
  .msg-row.assistant { align-items: flex-start; }

  .input-area {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .btn-sm {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 7px 12px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .btn-sm:hover:not(:disabled) { opacity: 0.9; }
  .btn-sm:disabled { opacity: 0.55; cursor: not-allowed; }
  .btn-sm.danger { background: var(--red); }

  .msg-bubble {
    max-width: 85%;
    padding: 8px 12px;
    border-radius: 12px;
    font-size: 13px;
    line-height: 1.5;
    word-break: break-word;
  }
  .msg-row.user .msg-bubble {
    background: var(--accent);
    color: #fff;
    border-bottom-right-radius: 3px;
  }
  .msg-row.assistant .msg-bubble {
    background: var(--bg);
    border: 1px solid var(--border);
    border-bottom-left-radius: 3px;
  }

  .msg-bubble :global(code) {
    background: rgba(0,0,0,0.25);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 12px;
  }
  .msg-bubble :global(pre) {
    background: rgba(0,0,0,0.3);
    border-radius: 6px;
    padding: 8px;
    overflow-x: auto;
    margin: 4px 0;
  }
  .msg-bubble :global(.code-block-wrap) {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 6px 0;
  }
  .msg-bubble :global(.code-block-actions) {
    display: flex;
    justify-content: flex-end;
  }
  .msg-bubble :global(.apply-code-btn) {
    background: rgba(34, 197, 94, 0.16);
    border: 1px solid rgba(34, 197, 94, 0.35);
    color: var(--accent-hl);
    border-radius: 6px;
    padding: 3px 8px;
    font-size: 11px;
    cursor: pointer;
  }
  .msg-bubble :global(.apply-code-btn:hover) { opacity: 0.88; }
  .msg-bubble :global(a) { color: var(--accent-hl); }

  .msg-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 4px;
    min-height: 16px;
  }

  .msg-time {
    font-size: 10px;
    color: var(--text-dim);
  }

  .msg-stats {
    font-size: 10px;
    color: var(--text-dim);
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }

  .tool-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tool-badge {
    font-size: 10px;
    background: rgba(251, 191, 36, 0.1);
    border: 1px solid rgba(251, 191, 36, 0.25);
    color: #f59e0b;
    border-radius: 999px;
    padding: 1px 7px;
    white-space: nowrap;
  }

  /* Completed tool history shown during multi-turn inference */
  .completed-tools {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    max-width: 85%;
    margin-bottom: 4px;
  }

  .completed-tool {
    font-size: 10px;
    background: rgba(34, 197, 94, 0.08);
    border: 1px solid rgba(34, 197, 94, 0.2);
    color: var(--green);
    border-radius: 999px;
    padding: 2px 8px;
    white-space: nowrap;
  }

  /* Live tool-use indicator shown during multi-turn */
  .live-tool {
    max-width: 85%;
    font-size: 11px;
    color: #f59e0b;
    background: rgba(251, 191, 36, 0.07);
    border: 1px solid rgba(251, 191, 36, 0.2);
    border-radius: 6px;
    padding: 4px 10px;
    margin-bottom: 4px;
    animation: pulse 1.4s infinite;
  }

  /* Thinking block (<details>) */
  .think-block {
    max-width: 85%;
    background: rgba(251, 191, 36, 0.06);
    border: 1px solid rgba(251, 191, 36, 0.25);
    border-radius: 8px;
    margin-bottom: 4px;
    overflow: hidden;
    font-size: 12px;
  }
  .think-summary {
    cursor: pointer;
    padding: 5px 10px;
    color: #f59e0b;
    font-weight: 600;
    font-size: 11px;
    letter-spacing: 0.03em;
    user-select: none;
    list-style: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .think-summary::before {
    content: '▶';
    font-size: 9px;
    transition: transform 0.2s;
  }
  details[open] .think-summary::before { transform: rotate(90deg); }
  .think-content {
    padding: 6px 10px 8px;
    color: var(--text-dim);
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.5;
    border-top: 1px solid rgba(251, 191, 36, 0.15);
    max-height: 260px;
    overflow-y: auto;
    font-size: 11px;
    font-family: monospace;
  }

  /* Thinking animation (dots — shown before any tokens arrive) */
  .thinking {
    display: flex;
    gap: 4px;
    align-items: center;
    min-width: 40px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-dim);
    animation: bounce 1.2s infinite;
  }
  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }

  .stream-placeholder {
    color: var(--text-dim);
    font-size: 13px;
  }
  @keyframes bounce {
    0%, 80%, 100% { transform: scale(0.7); opacity: 0.5; }
    40%            { transform: scale(1.0); opacity: 1;   }
  }

  /* Permission cards */
  .perm-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    max-width: 92%;
  }
  .perm-card.danger { border-left-color: var(--red); }

  .perm-header {
    display: flex;
    align-items: flex-start;
    gap: 7px;
  }
  .perm-icon  { font-size: 14px; flex-shrink: 0; }
  .perm-title { font-weight: 600; font-size: 12px; line-height: 1.4; }

  .perm-args {
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: var(--bg2);
    border-radius: 6px;
    padding: 6px 8px;
  }
  .perm-arg-row { display: flex; align-items: baseline; gap: 6px; }
  .perm-arg-key {
    font-size: 10px;
    color: var(--text-dim);
    min-width: 52px;
    flex-shrink: 0;
    font-family: monospace;
  }
  .perm-arg-val {
    font-size: 11px;
    font-family: monospace;
    word-break: break-all;
    color: var(--text);
  }

  .perm-paths { display: flex; flex-wrap: wrap; gap: 4px; }
  .perm-path  {
    background: var(--bg2);
    border: 1px solid var(--border);
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }

  .perm-cmd {
    background: var(--bg2);
    border: 1px solid var(--border);
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
    white-space: pre-wrap;
    margin: 0;
  }

  .perm-actions { display: flex; gap: 6px; }
  .perm-diff-hint {
    font-size: 11px;
    color: var(--text-dim);
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
  }
  .btn-approve {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 5px 14px;
    font-size: 12px;
    cursor: pointer;
    font-weight: 500;
  }
  .btn-approve:hover { opacity: 0.85; }
  .btn-deny {
    background: transparent;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 14px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-deny:hover { background: var(--bg-hover); }

  /* Error */
  .error-bar {
    background: var(--red);
    color: #fff;
    font-size: 12px;
    padding: 6px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .error-bar button {
    background: transparent;
    border: none;
    color: #fff;
    cursor: pointer;
    font-size: 14px;
  }

  /* Input */
  .input-area {
    border-top: 1px solid var(--border);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }

  .chat-input {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 13px;
    color: var(--text);
    resize: none;
    outline: none;
    font-family: inherit;
    line-height: 1.5;
    transition: border-color 0.15s;
    width: 100%;
  }
  .chat-input:focus { border-color: var(--accent); }
  .chat-input:disabled { opacity: 0.6; }

  .input-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-start;
    align-items: center;
    flex-wrap: nowrap;
  }

  .btn-send {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 7px;
    padding: 6px 16px;
    font-size: 13px;
    cursor: pointer;
    transition: opacity 0.15s;
    min-width: 92px;
  }
  .btn-send:hover:not(:disabled) { opacity: 0.85; }
  .btn-send:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-voice {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    font-size: 14px;
    cursor: pointer;
    transition: background 0.15s;
    color: var(--text);
  }
  .btn-voice:hover:not(:disabled) { background: var(--bg-hover); }
  .btn-voice:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-voice.recording {
    background: var(--red);
    color: #fff;
    border-color: var(--red);
    animation: pulse 1s infinite;
  }

  .btn-tts {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    font-size: 12px;
    cursor: pointer;
    color: var(--text-dim);
  }
  .btn-tts:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
  .btn-tts:disabled { opacity: 0.5; cursor: not-allowed; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.7; }
  }

  /* Model switch progress badge */
  .model-progress-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(34, 197, 94, 0.12);
    border-top: 1px solid rgba(34, 197, 94, 0.3);
    border-bottom: 1px solid rgba(34, 197, 94, 0.3);
    color: var(--accent-hl);
    font-size: 12px;
    animation: pulse 1.4s infinite;
  }

  /* Thinking status bar */
  .response-status {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: rgba(34, 197, 94, 0.08);
    border-top: 1px solid var(--border);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }
  .btn-stop {
    margin-left: auto;
    background: var(--red);
    border: none;
    color: #fff;
    border-radius: 7px;
    padding: 5px 10px;
    font-size: 11px;
    cursor: pointer;
  }
  .btn-stop:hover { opacity: 0.9; }
  .status-detail {
    color: var(--accent-hl);
    font-size: 11px;
    font-weight: 400;
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(34, 197, 94, 0.25);
    border-top-color: var(--accent-hl);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .tools-overlay {
    position: absolute;
    inset: 0;
    background: rgba(7, 10, 18, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 40;
    padding: 16px;
  }
  .tools-panel {
    width: min(720px, 100%);
    max-height: 80vh;
    overflow: auto;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.35);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tools-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .tools-header h3 {
    margin: 0;
    font-size: 15px;
    color: var(--text);
  }
  .tools-close {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 8px;
    width: 28px;
    height: 28px;
    cursor: pointer;
  }
  .tools-close:hover { background: var(--bg-hover); }
  .tools-section {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .tools-section-title {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .toggle-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 8px;
    border-radius: 8px;
    border: 1px solid transparent;
    background: var(--bg);
  }
  .toggle-row:hover { border-color: var(--border); }
  .toggle-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .toggle-name {
    font-size: 13px;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .toggle-desc {
    font-size: 11px;
    color: var(--text-dim);
    line-height: 1.4;
  }
  .tool-chip {
    font-size: 10px;
    border: 1px solid rgba(34, 197, 94, 0.4);
    color: var(--accent-hl);
    border-radius: 999px;
    padding: 1px 6px;
  }
  .tool-chip.custom {
    border-color: rgba(16, 185, 129, 0.45);
    color: #34d399;
  }
  .tools-empty {
    font-size: 12px;
    color: var(--text-dim);
    padding: 8px;
  }
</style>
