import * as monaco from 'monaco-editor';
import { detectFileType } from '$lib/utils/filetypes';

type EditorScope = 'global' | 'canvas';

interface EditorEntry {
  editor: monaco.editor.IStandaloneCodeEditor;
  scope: EditorScope;
  mountedAt: number;
}

const MAX_EDITORS_BY_SCOPE: Record<EditorScope, number> = {
  global: 14,
  canvas: 6,
};

const activeEditors = new Set<EditorEntry>();

function enforceEditorBudget(scope: EditorScope) {
  const limit = MAX_EDITORS_BY_SCOPE[scope];
  const scoped = [...activeEditors]
    .filter((entry) => entry.scope === scope)
    .sort((a, b) => a.mountedAt - b.mountedAt);

  while (scoped.length >= limit) {
    const oldest = scoped.shift();
    if (!oldest) break;
    activeEditors.delete(oldest);
    oldest.editor.dispose();
  }
}

function trackEditor(editor: monaco.editor.IStandaloneCodeEditor, scope: EditorScope) {
  const entry: EditorEntry = {
    editor,
    scope,
    mountedAt: Date.now(),
  };
  activeEditors.add(entry);
  editor.onDidDispose(() => {
    activeEditors.delete(entry);
  });
}

export function createEditor(
  container: HTMLElement,
  initialValue = '',
  theme: 'vs-dark' | 'vs' | 'hc-black' = 'vs-dark',
  options: { scope?: EditorScope } = {},
): monaco.editor.IStandaloneCodeEditor {
  const scope = options.scope ?? 'global';
  enforceEditorBudget(scope);

  const editor = monaco.editor.create(container, {
    value:             initialValue,
    language:          'plaintext',
    theme,
    automaticLayout:   true,
    fontSize:          14,
    lineHeight:        22,
    fontFamily:        "'JetBrains Mono', 'Fira Code', Menlo, monospace",
    fontLigatures:     true,
    minimap:           { enabled: false },
    wordWrap:          'on',
    scrollBeyondLastLine: false,
    smoothScrolling:   true,
    cursorBlinking:    'smooth',
    cursorSmoothCaretAnimation: 'on',
    bracketPairColorization: { enabled: true },
    renderLineHighlight: 'gutter',
    // Keep first line visible beneath overlay pills/buttons in MonacoEditor.
    padding:           { top: 44, bottom: 8 },
    tabSize:           2,
    insertSpaces:      true,
    formatOnPaste:     true,
    formatOnType:      false,
    suggest: {
      showKeywords:   true,
      showSnippets:   true,
      showClasses:    true,
      showFunctions:  true,
    },
  });

  trackEditor(editor, scope);
  return editor;
}

export function setLanguageFromPath(
  editor: monaco.editor.IStandaloneCodeEditor,
  filePath: string,
) {
  const lang = detectFileType(filePath).languageId;
  const model = editor.getModel();
  if (model) monaco.editor.setModelLanguage(model, lang);
}

export function setEditorTheme(theme: 'dark' | 'light' | 'high-contrast') {
  const map = { dark: 'vs-dark', light: 'vs', 'high-contrast': 'hc-black' } as const;
  monaco.editor.setTheme(map[theme]);
}

/** Create a minimal diff decoration set for a set of line ranges. */
export function addDiffDecorations(
  editor:     monaco.editor.IStandaloneCodeEditor,
  hunks:      Array<{ startLine: number; endLine: number; type: string }>,
): string[] {
  const decorations: monaco.editor.IModelDeltaDecoration[] = hunks.map((h) => ({
    range: new monaco.Range(h.startLine, 1, Math.max(h.startLine, h.endLine), 1),
    options: {
      isWholeLine:      true,
      className:        h.type === 'delete' ? 'diff-delete-line' : 'diff-insert-line',
      glyphMarginClassName: h.type === 'delete' ? 'diff-glyph-delete' : 'diff-glyph-insert',
      overviewRuler:    { color: h.type === 'delete' ? '#ef4444' : '#22c55e', position: 1 },
    },
  }));
  const ids = editor.deltaDecorations([], decorations);
  return ids;
}
