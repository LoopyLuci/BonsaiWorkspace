import * as monaco from 'monaco-editor';

const EXT_LANG: Record<string, string> = {
  rs:   'rust',
  ts:   'typescript',
  tsx:  'typescript',
  js:   'javascript',
  jsx:  'javascript',
  py:   'python',
  md:   'markdown',
  json: 'json',
  toml: 'ini',
  yaml: 'yaml',
  yml:  'yaml',
  css:  'css',
  html: 'html',
  sh:   'shell',
  bash: 'shell',
  sql:  'sql',
  go:   'go',
  cpp:  'cpp',
  c:    'c',
  cs:   'csharp',
  rb:   'ruby',
  kt:   'kotlin',
  swift:'swift',
};

export function createEditor(
  container: HTMLElement,
  initialValue = '',
  theme: 'vs-dark' | 'vs' | 'hc-black' = 'vs-dark',
): monaco.editor.IStandaloneCodeEditor {
  return monaco.editor.create(container, {
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
    padding:           { top: 8, bottom: 8 },
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
}

export function setLanguageFromPath(
  editor: monaco.editor.IStandaloneCodeEditor,
  filePath: string,
) {
  const ext  = filePath.split('.').pop()?.toLowerCase() ?? '';
  const lang = EXT_LANG[ext] ?? 'plaintext';
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
