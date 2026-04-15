<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import * as monaco from 'monaco-editor';
  import { createEditor, setLanguageFromPath, setEditorTheme, addDiffDecorations } from '$lib/utils/monaco';
  import { invoke } from '@tauri-apps/api/core';
  import { currentDiff, clearCurrentDiff, clearDiffForFile, type DiffHunk } from '$lib/stores/diff';
  import { openFileRequest } from '$lib/stores/openFile';
  import { activeEditorFile } from '$lib/stores/activeEditorFile';
  import { requestAskBonsai, type AskBonsaiAction } from '$lib/stores/chat';
  import { addToast } from '$lib/stores/toast';

  export let theme: 'dark' | 'light' | 'high-contrast' = 'dark';

  // ── State ────────────────────────────────────────────────────────────────
  let container:       HTMLDivElement;
  let editor:          monaco.editor.IStandaloneCodeEditor;
  let currentFilePath  = '';
  let decorationIds:   string[]                        = [];
  let contentWidgets:  monaco.editor.IContentWidget[]  = [];
  let saveTimer:       ReturnType<typeof setTimeout> | null = null;
  let inlineSeq        = 0;
  let inlineDisposables: monaco.IDisposable[] = [];
  let isDirty          = false;
  let errorMsg         = '';

  // ── Theme sync ───────────────────────────────────────────────────────────
  $: if (editor) setEditorTheme(theme);

  // ── Open file from store ─────────────────────────────────────────────────
  // This is the correct pattern: subscribe to a store instead of an exported fn.
  const unsubOpenFile = openFileRequest.subscribe((path) => {
    if (path && editor) openFile(path);
  });

  async function openFile(path: string) {
    if (!path) return;
    try {
      errorMsg = '';
      const content = await invoke<string>('read_file', { path });
      currentFilePath = path;
      editor.setValue(content);
      setLanguageFromPath(editor, path);
      clearDiff();
      isDirty = false;
      activeEditorFile.set({ path, content });
    } catch (e) {
      errorMsg = `Cannot open file: ${e}`;
    }
  }

  // ── Diff rendering ───────────────────────────────────────────────────────
  const unsubDiff = currentDiff.subscribe((diff) => {
    if (!editor) return;
    clearDiff();
    if (!diff || diff.filePath !== currentFilePath) return;
    renderDiff(diff.hunks, diff.rawUnifiedDiff);
  });

  function clearDiff() {
    if (!editor) return;
    decorationIds = editor.deltaDecorations(decorationIds, []);
    contentWidgets.forEach((w) => editor.removeContentWidget(w));
    contentWidgets = [];
  }

  function renderDiff(hunks: DiffHunk[], rawDiff: string) {
    decorationIds = addDiffDecorations(editor, hunks);

    hunks.forEach((hunk) => {
      const widgetNode = buildHunkWidget(hunk.hunkIndex, rawDiff);
      const widget: monaco.editor.IContentWidget = {
        getId:       () => `bonsai-hunk-${hunk.hunkIndex}`,
        getDomNode:  () => widgetNode,
        getPosition: () => ({
          position:   { lineNumber: hunk.startLine, column: 1 },
          preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE],
        }),
      };
      editor.addContentWidget(widget);
      contentWidgets.push(widget);
    });
  }

  function buildHunkWidget(hunkIndex: number, rawDiff: string): HTMLElement {
    const wrap = document.createElement('div');
    wrap.style.cssText = 'display:flex;gap:6px;align-items:center;padding:2px 4px;';

    const acceptBtn = document.createElement('button');
    acceptBtn.textContent  = '✓ Accept';
    acceptBtn.style.cssText =
      'background:#22c55e;color:#fff;border:none;padding:2px 10px;border-radius:4px;' +
      'font-size:11px;cursor:pointer;font-family:system-ui;';
    acceptBtn.setAttribute('aria-label', `Accept hunk ${hunkIndex}`);
    acceptBtn.onclick = () => acceptHunk(hunkIndex, rawDiff);

    const rejectBtn = document.createElement('button');
    rejectBtn.textContent  = '✕ Reject';
    rejectBtn.style.cssText =
      'background:#ef4444;color:#fff;border:none;padding:2px 10px;border-radius:4px;' +
      'font-size:11px;cursor:pointer;font-family:system-ui;';
    rejectBtn.setAttribute('aria-label', `Reject hunk ${hunkIndex}`);
    rejectBtn.onclick = () => rejectHunk(hunkIndex);

    const label = document.createElement('span');
    label.textContent  = `Hunk ${hunkIndex + 1}`;
    label.style.cssText = 'font-size:11px;color:#a1a1aa;font-family:system-ui;';

    wrap.append(label, acceptBtn, rejectBtn);
    return wrap;
  }

  async function acceptHunk(hunkIndex: number, rawDiff: string) {
    if (!currentFilePath) return;
    try {
      await invoke('accept_diff_hunk', { filePath: currentFilePath, hunkIndex, diff: rawDiff });
      const newContent = await invoke<string>('read_file', { path: currentFilePath });
      editor.setValue(newContent);
      clearCurrentDiff();
      clearDiffForFile(currentFilePath);
      clearDiff();
    } catch (e) {
      errorMsg = `Accept failed: ${e}`;
    }
  }

  async function rejectHunk(hunkIndex: number) {
    if (!currentFilePath) return;
    try {
      await invoke('reject_diff_hunk', { filePath: currentFilePath, hunkIndex });
      clearCurrentDiff();
      clearDiffForFile(currentFilePath);
      clearDiff();
    } catch (e) {
      errorMsg = `Reject failed: ${e}`;
    }
  }

  // ── Auto-save (750 ms debounce) ─────────────────────────────────────────
  function setupAutoSave() {
    editor.onDidChangeModelContent(() => {
      isDirty = true;
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(async () => {
        if (!currentFilePath) return;
        try {
          const content = editor.getValue();
          await invoke('write_file', { path: currentFilePath, content });
          isDirty = false;
          // Keep the active file store in sync with saved content.
          activeEditorFile.set({ path: currentFilePath, content });
        } catch (e) {
          errorMsg = `Auto-save failed: ${e}`;
        }
      }, 750);
    });
  }

  function getSelectedText(): string {
    const model = editor.getModel();
    const selection = editor.getSelection();
    if (!model || !selection || selection.isEmpty()) return '';
    return model.getValueInRange(selection);
  }

  function buildAskPrompt(action: AskBonsaiAction, selectedText: string): string {
    const language = editor.getModel()?.getLanguageId() ?? 'text';
    const fileLabel = currentFilePath || 'current file';

    if (action === 'explain') {
      return [
        `Explain this selected code from ${fileLabel}.`,
        'Focus on intent, control flow, and edge cases.',
        '',
        `\`\`\`${language}`,
        selectedText,
        '\`\`\`',
      ].join('\n');
    }

    if (action === 'fix') {
      return [
        `Fix issues in this selected code from ${fileLabel}.`,
        'Keep behavior correct, call out assumptions, and provide a concrete patch-ready suggestion.',
        '',
        `\`\`\`${language}`,
        selectedText,
        '\`\`\`',
      ].join('\n');
    }

    return [
      `Refactor this selected code from ${fileLabel}.`,
      'Improve readability/maintainability without changing behavior. Explain key changes briefly.',
      '',
      `\`\`\`${language}`,
      selectedText,
      '\`\`\`',
    ].join('\n');
  }

  function askBonsaiFromSelection(action: AskBonsaiAction) {
    if (!currentFilePath) {
      addToast('Open a file in the editor first.', 'info');
      return;
    }

    const selectedText = getSelectedText();
    if (!selectedText.trim()) {
      addToast('Select code first to Ask Bonsai.', 'info');
      return;
    }

    requestAskBonsai({
      action,
      prompt: buildAskPrompt(action, selectedText),
    });
  }

  function setupAskBonsaiActions() {
    editor.addAction({
      id: 'bonsai.ask.explain',
      label: 'Ask Bonsai: Explain Selection',
      precondition: 'editorHasSelection',
      contextMenuGroupId: 'navigation',
      contextMenuOrder: 1.1,
      run: () => askBonsaiFromSelection('explain'),
    });

    editor.addAction({
      id: 'bonsai.ask.fix',
      label: 'Ask Bonsai: Fix Selection',
      precondition: 'editorHasSelection',
      contextMenuGroupId: 'navigation',
      contextMenuOrder: 1.2,
      run: () => askBonsaiFromSelection('fix'),
    });

    editor.addAction({
      id: 'bonsai.ask.refactor',
      label: 'Ask Bonsai: Refactor Selection',
      precondition: 'editorHasSelection',
      contextMenuGroupId: 'navigation',
      contextMenuOrder: 1.3,
      run: () => askBonsaiFromSelection('refactor'),
    });
  }

  function setupInlineCompletions() {
    const provider: monaco.languages.InlineCompletionsProvider = {
      provideInlineCompletions: async (model, position, _context, token) => {
        if (token.isCancellationRequested || !currentFilePath) {
          return { items: [] };
        }

        const linePrefix = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
        if (!linePrefix.trim()) {
          return { items: [] };
        }

        const offset = model.getOffsetAt(position);
        const beforeCursor = model.getValue().slice(Math.max(0, offset - 3000), offset);
        const afterCursor = model.getValue().slice(offset, Math.min(model.getValueLength(), offset + 1200));
        const language = model.getLanguageId() || 'plaintext';
        const requestSeq = ++inlineSeq;

        // Debounce: only the newest request survives this delay window.
        await new Promise((resolve) => setTimeout(resolve, 300));

        if (token.isCancellationRequested || requestSeq !== inlineSeq) {
          return { items: [] };
        }

        let completion = '';
        try {
          completion = await invoke<string>('generate_inline_completion', {
            filePath: currentFilePath,
            language,
            beforeCursor,
            afterCursor,
          });
        } catch {
          completion = '';
        }

        if (token.isCancellationRequested || requestSeq !== inlineSeq) {
          return { items: [] };
        }

        const insertText = completion.trim();
        if (!insertText) {
          return { items: [] };
        }

        return {
          items: [
            {
              insertText,
              range: new monaco.Range(
                position.lineNumber,
                position.column,
                position.lineNumber,
                position.column,
              ),
            },
          ],
        };
      },
      freeInlineCompletions: () => {
        // no-op
      },
    };

    inlineDisposables = monaco
      .languages
      .getLanguages()
      .map((lang) => monaco.languages.registerInlineCompletionsProvider(lang.id, provider));
  }

  // ── Lifecycle ────────────────────────────────────────────────────────────
  onMount(() => {
    editor = createEditor(container, '', theme === 'dark' ? 'vs-dark' : theme === 'light' ? 'vs' : 'hc-black');
    setupAutoSave();
    setupAskBonsaiActions();
    setupInlineCompletions();

    // The openFileRequest subscription fires immediately on setup, before the
    // editor exists. Re-check the store now that the editor is ready so a
    // pending file open is not silently dropped.
    const pending = get(openFileRequest);
    if (pending) openFile(pending);
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    unsubOpenFile();
    unsubDiff();
    inlineDisposables.forEach((d) => d.dispose());
    inlineDisposables = [];
    editor?.dispose();
  });
</script>

<div class="editor-host">
  {#if errorMsg}
    <div class="error-banner" role="alert">
      {errorMsg}
      <button on:click={() => (errorMsg = '')}>✕</button>
    </div>
  {/if}

  {#if currentFilePath}
    <div class="file-pill">
      {currentFilePath.split(/[/\\]/).pop()}
      {#if isDirty}<span class="dirty-dot" title="Unsaved changes">●</span>{/if}
    </div>
  {/if}

  <div bind:this={container} class="monaco-container"></div>

  {#if !currentFilePath}
    <div class="empty-state">
      <div class="empty-icon">🌿</div>
      <div class="empty-title">Bonsai Workspace</div>
      <div class="empty-sub">Open a folder and select a file to start editing</div>
      <div class="empty-hint">Ctrl+K — Command Palette</div>
    </div>
  {/if}
</div>

<style>
  .editor-host {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .monaco-container {
    flex: 1;
    width: 100%;
    min-height: 0;
  }

  .error-banner {
    background: var(--red);
    color: #fff;
    font-size: 12px;
    padding: 6px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }
  .error-banner button {
    background: transparent;
    border: none;
    color: #fff;
    cursor: pointer;
    font-size: 14px;
    padding: 0 4px;
  }

  .file-pill {
    position: absolute;
    top: 8px;
    right: 16px;
    z-index: 10;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 20px;
    font-size: 11px;
    padding: 2px 10px;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 4px;
    pointer-events: none;
  }
  .dirty-dot { color: var(--amber); font-size: 16px; line-height: 1; }

  .empty-state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    pointer-events: none;
    user-select: none;
  }
  .empty-icon  { font-size: 40px; }
  .empty-title { font-size: 20px; font-weight: 600; color: var(--text); }
  .empty-sub   { font-size: 13px; color: var(--text-dim); }
  .empty-hint  {
    font-size: 11px;
    color: var(--text-dim);
    background: var(--bg2);
    border: 1px solid var(--border);
    padding: 3px 10px;
    border-radius: 6px;
    margin-top: 8px;
  }
</style>
