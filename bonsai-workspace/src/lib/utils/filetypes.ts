export type ToolingProfileId =
  | 'web'
  | 'python'
  | 'rust'
  | 'powershell'
  | 'shell'
  | 'config'
  | 'docs'
  | 'data'
  | 'generic';

export type FileTypeInfo = {
  extension: string;
  languageId: string;
  icon: string;
  label: string;
  toolingProfile: ToolingProfileId;
};

const NAME_MAP: Record<string, Omit<FileTypeInfo, 'extension'>> = {
  'dockerfile':        { languageId: 'dockerfile', icon: '🐳', label: 'Dockerfile', toolingProfile: 'config' },
  'makefile':          { languageId: 'makefile', icon: '🛠', label: 'Makefile', toolingProfile: 'config' },
  '.gitignore':        { languageId: 'ignore', icon: '🚫', label: 'Git Ignore', toolingProfile: 'config' },
  '.env':              { languageId: 'shell', icon: '🔑', label: 'Environment', toolingProfile: 'config' },
  'cargo.toml':        { languageId: 'toml', icon: '🦀', label: 'Cargo Config', toolingProfile: 'rust' },
  'package.json':      { languageId: 'json', icon: '📦', label: 'Package Manifest', toolingProfile: 'web' },
  'tsconfig.json':     { languageId: 'json', icon: '🔷', label: 'TypeScript Config', toolingProfile: 'web' },
  'vite.config.ts':    { languageId: 'typescript', icon: '⚡', label: 'Vite Config', toolingProfile: 'web' },
  'readme.md':         { languageId: 'markdown', icon: '📘', label: 'Readme', toolingProfile: 'docs' },
};

const EXT_MAP: Record<string, Omit<FileTypeInfo, 'extension'>> = {
  rs:      { languageId: 'rust', icon: '🦀', label: 'Rust', toolingProfile: 'rust' },
  ts:      { languageId: 'typescript', icon: '🔷', label: 'TypeScript', toolingProfile: 'web' },
  tsx:     { languageId: 'typescript', icon: '🔷', label: 'TypeScript React', toolingProfile: 'web' },
  js:      { languageId: 'javascript', icon: '🟨', label: 'JavaScript', toolingProfile: 'web' },
  jsx:     { languageId: 'javascript', icon: '🟨', label: 'JavaScript React', toolingProfile: 'web' },
  vue:     { languageId: 'javascript', icon: '🟩', label: 'Vue', toolingProfile: 'web' },
  svelte:  { languageId: 'svelte', icon: '🧡', label: 'Svelte', toolingProfile: 'web' },
  py:      { languageId: 'python', icon: '🐍', label: 'Python', toolingProfile: 'python' },
  ps1:     { languageId: 'powershell', icon: '💠', label: 'PowerShell', toolingProfile: 'powershell' },
  psm1:    { languageId: 'powershell', icon: '💠', label: 'PowerShell Module', toolingProfile: 'powershell' },
  sh:      { languageId: 'shell', icon: '💲', label: 'Shell', toolingProfile: 'shell' },
  bash:    { languageId: 'shell', icon: '💲', label: 'Bash', toolingProfile: 'shell' },
  zsh:     { languageId: 'shell', icon: '💲', label: 'Zsh', toolingProfile: 'shell' },
  cmd:     { languageId: 'bat', icon: '🧩', label: 'Batch', toolingProfile: 'powershell' },
  bat:     { languageId: 'bat', icon: '🧩', label: 'Batch', toolingProfile: 'powershell' },
  json:    { languageId: 'json', icon: '📋', label: 'JSON', toolingProfile: 'data' },
  jsonc:   { languageId: 'json', icon: '📋', label: 'JSONC', toolingProfile: 'data' },
  yaml:    { languageId: 'yaml', icon: '⚙', label: 'YAML', toolingProfile: 'config' },
  yml:     { languageId: 'yaml', icon: '⚙', label: 'YAML', toolingProfile: 'config' },
  toml:    { languageId: 'toml', icon: '⚙', label: 'TOML', toolingProfile: 'config' },
  ini:     { languageId: 'ini', icon: '⚙', label: 'INI', toolingProfile: 'config' },
  conf:    { languageId: 'ini', icon: '⚙', label: 'Config', toolingProfile: 'config' },
  md:      { languageId: 'markdown', icon: '📝', label: 'Markdown', toolingProfile: 'docs' },
  txt:     { languageId: 'plaintext', icon: '📄', label: 'Text', toolingProfile: 'docs' },
  html:    { languageId: 'html', icon: '🌐', label: 'HTML', toolingProfile: 'web' },
  css:     { languageId: 'css', icon: '🎨', label: 'CSS', toolingProfile: 'web' },
  scss:    { languageId: 'scss', icon: '🎨', label: 'SCSS', toolingProfile: 'web' },
  sql:     { languageId: 'sql', icon: '🗄', label: 'SQL', toolingProfile: 'data' },
  go:      { languageId: 'go', icon: '🐹', label: 'Go', toolingProfile: 'generic' },
  c:       { languageId: 'c', icon: '⚡', label: 'C', toolingProfile: 'generic' },
  cpp:     { languageId: 'cpp', icon: '⚡', label: 'C++', toolingProfile: 'generic' },
  h:       { languageId: 'cpp', icon: '⚡', label: 'Header', toolingProfile: 'generic' },
  hpp:     { languageId: 'cpp', icon: '⚡', label: 'Header', toolingProfile: 'generic' },
  cs:      { languageId: 'csharp', icon: '🔵', label: 'C#', toolingProfile: 'generic' },
  rb:      { languageId: 'ruby', icon: '💎', label: 'Ruby', toolingProfile: 'generic' },
  kt:      { languageId: 'kotlin', icon: '🎯', label: 'Kotlin', toolingProfile: 'generic' },
  swift:   { languageId: 'swift', icon: '🍎', label: 'Swift', toolingProfile: 'generic' },
  lock:    { languageId: 'plaintext', icon: '🔒', label: 'Lockfile', toolingProfile: 'config' },
};

const DEFAULT_INFO: FileTypeInfo = {
  extension: '',
  languageId: 'plaintext',
  icon: '📄',
  label: 'Plain Text',
  toolingProfile: 'generic',
};

export function detectFileType(pathOrName: string): FileTypeInfo {
  const name = pathOrName.split(/[\\/]/).pop()?.toLowerCase() ?? '';
  const nameMapped = NAME_MAP[name];
  if (nameMapped) {
    return { extension: '', ...nameMapped };
  }

  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) {
    return DEFAULT_INFO;
  }

  const extension = name.slice(dot + 1);
  const mapped = EXT_MAP[extension];
  if (!mapped) {
    return { ...DEFAULT_INFO, extension };
  }

  return { extension, ...mapped };
}
