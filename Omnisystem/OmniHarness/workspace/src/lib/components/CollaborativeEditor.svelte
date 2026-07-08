<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import {
    remoteCursors, crdtOperations, activeFile,
    initCrdtListeners, destroyCrdtListeners,
    sendCursorPosition, sendCrdtOperation,
  } from '../stores/crdtEditor';

  export let sessionId: string;
  export let peerId: string;
  export let filePath: string = '';
  export let language: string = 'plaintext';

  let container: HTMLDivElement;
  let editor: any = null;
  let monaco: any = null;
  let decorations: string[] = [];
  let applyingRemote = false;
  let opUnsubscribe: (() => void) | null = null;
  let cursorUnsubscribe: (() => void) | null = null;

  onMount(async () => {
    await initCrdtListeners();

    // Dynamically load Monaco (bundled via vite)
    const mod = await import('monaco-editor');
    monaco = mod;

    editor = monaco.editor.create(container, {
      value: '',
      language,
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 13,
      lineNumbers: 'on',
    });

    // Track cursor
    editor.onDidChangeCursorPosition((e: any) => {
      if (!sessionId || !peerId || !filePath) return;
      sendCursorPosition(sessionId, peerId, filePath, e.position.lineNumber, e.position.column);
    });

    // Broadcast local edits as CRDT ops
    editor.onDidChangeModelContent((e: any) => {
      if (applyingRemote) return;
      for (const change of e.changes) {
        const model = editor.getModel();
        if (!model) continue;
        const offset = model.getOffsetAt(change.range.getStartPosition());
        if (change.text.length > 0) {
          sendCrdtOperation(sessionId, {
            file: filePath,
            op_type: 'insert',
            position: offset,
            text: change.text,
            peer_id: peerId,
          });
        }
        if (change.rangeLength > 0) {
          sendCrdtOperation(sessionId, {
            file: filePath,
            op_type: 'delete',
            position: offset,
            length: change.rangeLength,
            peer_id: peerId,
          });
        }
      }
    });

    // Apply incoming remote ops
    opUnsubscribe = crdtOperations.subscribe(ops => {
      if (!editor || ops.length === 0) return;
      const latest = ops[ops.length - 1];
      if (latest.peer_id === peerId) return; // own op echoed back
      applyRemoteOp(latest);
    });

    // Update remote cursor decorations
    cursorUnsubscribe = remoteCursors.subscribe(cursors => {
      if (!editor || !monaco) return;
      const model = editor.getModel();
      if (!model) return;
      const currentFile = filePath;
      const newDecorations = Array.from(cursors.values())
        .filter(c => c.file === currentFile)
        .map(c => ({
          range: new monaco.Range(c.line, c.column, c.line, c.column + 1),
          options: {
            className: 'remote-cursor',
            glyphMarginClassName: 'remote-cursor-glyph',
            hoverMessage: { value: c.displayName },
            afterContentClassName: `remote-cursor-label`,
            stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
          },
        }));
      decorations = editor.deltaDecorations(decorations, newDecorations);
    });

    activeFile.set(filePath);
  });

  onDestroy(() => {
    opUnsubscribe?.();
    cursorUnsubscribe?.();
    editor?.dispose();
    destroyCrdtListeners();
  });

  function applyRemoteOp(op: any) {
    if (!editor) return;
    const model = editor.getModel();
    if (!model || op.file !== filePath) return;
    applyingRemote = true;
    try {
      if (op.op_type === 'insert' && op.text) {
        const pos = model.getPositionAt(op.position);
        model.applyEdits([{ range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column), text: op.text }]);
      } else if (op.op_type === 'delete' && op.length) {
        const start = model.getPositionAt(op.position);
        const end = model.getPositionAt(op.position + op.length);
        model.applyEdits([{ range: new monaco.Range(start.lineNumber, start.column, end.lineNumber, end.column), text: '' }]);
      }
    } finally {
      applyingRemote = false;
    }
  }

  export function setContent(text: string) {
    editor?.getModel()?.setValue(text);
  }

  export function getContent(): string {
    return editor?.getValue() ?? '';
  }
</script>

<div class="collab-editor" bind:this={container} data-workspace-action="Collaboration:Editor"></div>

<style>
  .collab-editor { width: 100%; height: 100%; min-height: 300px; }

  :global(.remote-cursor) { border-left: 2px solid currentColor; }
  :global(.remote-cursor-glyph) { background-color: currentColor; width: 4px !important; margin-left: 3px; }
</style>
