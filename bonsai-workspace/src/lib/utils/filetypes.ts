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
  extension:      string;
  languageId:     string;
  icon:           string;   // emoji/glyph shown in tree + pill
  iconColor:      string;   // CSS color for the icon span
  label:          string;
  toolingProfile: ToolingProfileId;
};

// ── Name-based overrides (full filename, lowercased) ─────────────────────────
const NAME_MAP: Record<string, Omit<FileTypeInfo, 'extension'>> = {
  'dockerfile':      { languageId: 'dockerfile',  icon: '◈',  iconColor: '#2496ed', label: 'Dockerfile',       toolingProfile: 'config' },
  'makefile':        { languageId: 'makefile',     icon: '⚙',  iconColor: '#888',    label: 'Makefile',         toolingProfile: 'config' },
  'justfile':        { languageId: 'makefile',     icon: '⚙',  iconColor: '#888',    label: 'Justfile',         toolingProfile: 'config' },
  '.gitignore':      { languageId: 'ignore',       icon: '⊘',  iconColor: '#f05033', label: 'Git Ignore',       toolingProfile: 'config' },
  '.gitattributes':  { languageId: 'ignore',       icon: '⊘',  iconColor: '#f05033', label: 'Git Attributes',   toolingProfile: 'config' },
  '.env':            { languageId: 'shell',        icon: '⬥',  iconColor: '#ecc94b', label: 'Environment',      toolingProfile: 'config' },
  '.env.local':      { languageId: 'shell',        icon: '⬥',  iconColor: '#ecc94b', label: 'Env (local)',       toolingProfile: 'config' },
  '.env.example':    { languageId: 'shell',        icon: '⬥',  iconColor: '#888',    label: 'Env (example)',    toolingProfile: 'config' },
  'cargo.toml':      { languageId: 'toml',         icon: '⬡',  iconColor: '#ce422b', label: 'Cargo Config',     toolingProfile: 'rust' },
  'cargo.lock':      { languageId: 'toml',         icon: '⬡',  iconColor: '#ce422b', label: 'Cargo Lock',       toolingProfile: 'rust' },
  'package.json':    { languageId: 'json',         icon: '⬡',  iconColor: '#cb3837', label: 'Package Manifest', toolingProfile: 'web' },
  'package-lock.json': { languageId: 'json',       icon: '⬡',  iconColor: '#888',    label: 'NPM Lockfile',     toolingProfile: 'web' },
  'yarn.lock':       { languageId: 'plaintext',    icon: '⬡',  iconColor: '#2c8ebb', label: 'Yarn Lockfile',    toolingProfile: 'web' },
  'pnpm-lock.yaml':  { languageId: 'yaml',         icon: '⬡',  iconColor: '#f69220', label: 'PNPM Lockfile',    toolingProfile: 'web' },
  'bun.lockb':       { languageId: 'plaintext',    icon: '⬡',  iconColor: '#fbf0df', label: 'Bun Lockfile',     toolingProfile: 'web' },
  'tsconfig.json':   { languageId: 'json',         icon: '◆',  iconColor: '#3178c6', label: 'TS Config',        toolingProfile: 'web' },
  'jsconfig.json':   { languageId: 'json',         icon: '◆',  iconColor: '#f0d04a', label: 'JS Config',        toolingProfile: 'web' },
  'vite.config.ts':  { languageId: 'typescript',   icon: '⚡',  iconColor: '#646cff', label: 'Vite Config',      toolingProfile: 'web' },
  'vite.config.js':  { languageId: 'javascript',   icon: '⚡',  iconColor: '#646cff', label: 'Vite Config',      toolingProfile: 'web' },
  'webpack.config.js': { languageId: 'javascript', icon: '⬡',  iconColor: '#8dd6f9', label: 'Webpack Config',   toolingProfile: 'web' },
  'rollup.config.js':  { languageId: 'javascript', icon: '⬡',  iconColor: '#ff3333', label: 'Rollup Config',    toolingProfile: 'web' },
  'svelte.config.js':  { languageId: 'javascript', icon: '◈',  iconColor: '#ff3e00', label: 'Svelte Config',    toolingProfile: 'web' },
  'tailwind.config.js': { languageId: 'javascript',icon: '◈',  iconColor: '#06b6d4', label: 'Tailwind Config',  toolingProfile: 'web' },
  'tailwind.config.ts': { languageId: 'typescript',icon: '◈',  iconColor: '#06b6d4', label: 'Tailwind Config',  toolingProfile: 'web' },
  'postcss.config.js':  { languageId: 'javascript',icon: '◈',  iconColor: '#dd3735', label: 'PostCSS Config',   toolingProfile: 'web' },
  '.eslintrc':       { languageId: 'json',         icon: '◈',  iconColor: '#4b32c3', label: 'ESLint Config',    toolingProfile: 'web' },
  '.eslintrc.json':  { languageId: 'json',         icon: '◈',  iconColor: '#4b32c3', label: 'ESLint Config',    toolingProfile: 'web' },
  '.eslintrc.js':    { languageId: 'javascript',   icon: '◈',  iconColor: '#4b32c3', label: 'ESLint Config',    toolingProfile: 'web' },
  '.prettierrc':     { languageId: 'json',         icon: '◈',  iconColor: '#f7b93e', label: 'Prettier Config',  toolingProfile: 'web' },
  '.prettierrc.json': { languageId: 'json',        icon: '◈',  iconColor: '#f7b93e', label: 'Prettier Config',  toolingProfile: 'web' },
  'readme.md':       { languageId: 'markdown',     icon: '□',  iconColor: '#4a9eff', label: 'Readme',           toolingProfile: 'docs' },
  'changelog.md':    { languageId: 'markdown',     icon: '□',  iconColor: '#4a9eff', label: 'Changelog',        toolingProfile: 'docs' },
  'license':         { languageId: 'plaintext',    icon: '□',  iconColor: '#888',    label: 'License',          toolingProfile: 'docs' },
  'license.md':      { languageId: 'markdown',     icon: '□',  iconColor: '#888',    label: 'License',          toolingProfile: 'docs' },
  'tauri.conf.json': { languageId: 'json',         icon: '◆',  iconColor: '#ffc131', label: 'Tauri Config',     toolingProfile: 'config' },
  '.github':         { languageId: 'yaml',         icon: '◈',  iconColor: '#f05033', label: 'GitHub',           toolingProfile: 'config' },
  'docker-compose.yml': { languageId: 'yaml',      icon: '◈',  iconColor: '#2496ed', label: 'Docker Compose',  toolingProfile: 'config' },
  'docker-compose.yaml': { languageId: 'yaml',     icon: '◈',  iconColor: '#2496ed', label: 'Docker Compose',  toolingProfile: 'config' },
};

// ── Extension-based fallbacks ─────────────────────────────────────────────────
const EXT_MAP: Record<string, Omit<FileTypeInfo, 'extension'>> = {
  // Rust
  rs:     { languageId: 'rust',        icon: '⬡',  iconColor: '#ce422b', label: 'Rust',            toolingProfile: 'rust' },
  // TypeScript / JavaScript
  ts:     { languageId: 'typescript',  icon: '◆',  iconColor: '#3178c6', label: 'TypeScript',       toolingProfile: 'web' },
  tsx:    { languageId: 'typescript',  icon: '◆',  iconColor: '#61dafb', label: 'TSX',              toolingProfile: 'web' },
  js:     { languageId: 'javascript',  icon: '◆',  iconColor: '#f0d04a', label: 'JavaScript',       toolingProfile: 'web' },
  jsx:    { languageId: 'javascript',  icon: '◆',  iconColor: '#61dafb', label: 'JSX',              toolingProfile: 'web' },
  mjs:    { languageId: 'javascript',  icon: '◆',  iconColor: '#f0d04a', label: 'ES Module',        toolingProfile: 'web' },
  cjs:    { languageId: 'javascript',  icon: '◆',  iconColor: '#888',    label: 'CommonJS',         toolingProfile: 'web' },
  // Frontend frameworks
  vue:    { languageId: 'javascript',  icon: '◈',  iconColor: '#42b883', label: 'Vue',              toolingProfile: 'web' },
  svelte: { languageId: 'svelte',      icon: '◈',  iconColor: '#ff3e00', label: 'Svelte',           toolingProfile: 'web' },
  astro:  { languageId: 'html',        icon: '◈',  iconColor: '#ff5d01', label: 'Astro',            toolingProfile: 'web' },
  // Python
  py:     { languageId: 'python',      icon: '◆',  iconColor: '#3572a5', label: 'Python',           toolingProfile: 'python' },
  pyw:    { languageId: 'python',      icon: '◆',  iconColor: '#3572a5', label: 'Python (win)',     toolingProfile: 'python' },
  ipynb:  { languageId: 'python',      icon: '◈',  iconColor: '#f37626', label: 'Notebook',         toolingProfile: 'python' },
  // Shell / scripts
  ps1:    { languageId: 'powershell',  icon: '▷',  iconColor: '#5391fe', label: 'PowerShell',       toolingProfile: 'powershell' },
  psm1:   { languageId: 'powershell',  icon: '▷',  iconColor: '#5391fe', label: 'PS Module',        toolingProfile: 'powershell' },
  psd1:   { languageId: 'powershell',  icon: '▷',  iconColor: '#5391fe', label: 'PS Data',          toolingProfile: 'powershell' },
  sh:     { languageId: 'shell',       icon: '▷',  iconColor: '#89e051', label: 'Shell',            toolingProfile: 'shell' },
  bash:   { languageId: 'shell',       icon: '▷',  iconColor: '#89e051', label: 'Bash',             toolingProfile: 'shell' },
  zsh:    { languageId: 'shell',       icon: '▷',  iconColor: '#89e051', label: 'Zsh',              toolingProfile: 'shell' },
  fish:   { languageId: 'shell',       icon: '▷',  iconColor: '#89e051', label: 'Fish',             toolingProfile: 'shell' },
  cmd:    { languageId: 'bat',         icon: '▷',  iconColor: '#c1f12e', label: 'Batch',            toolingProfile: 'powershell' },
  bat:    { languageId: 'bat',         icon: '▷',  iconColor: '#c1f12e', label: 'Batch',            toolingProfile: 'powershell' },
  // Data / config
  json:   { languageId: 'json',        icon: '{ }', iconColor: '#f0d04a', label: 'JSON',            toolingProfile: 'data' },
  jsonc:  { languageId: 'json',        icon: '{ }', iconColor: '#f0d04a', label: 'JSONC',           toolingProfile: 'data' },
  json5:  { languageId: 'json',        icon: '{ }', iconColor: '#f0d04a', label: 'JSON5',           toolingProfile: 'data' },
  yaml:   { languageId: 'yaml',        icon: '≡',   iconColor: '#cb171e', label: 'YAML',            toolingProfile: 'config' },
  yml:    { languageId: 'yaml',        icon: '≡',   iconColor: '#cb171e', label: 'YAML',            toolingProfile: 'config' },
  toml:   { languageId: 'toml',        icon: '≡',   iconColor: '#9c4121', label: 'TOML',            toolingProfile: 'config' },
  ini:    { languageId: 'ini',         icon: '≡',   iconColor: '#888',    label: 'INI',             toolingProfile: 'config' },
  conf:   { languageId: 'ini',         icon: '≡',   iconColor: '#888',    label: 'Config',          toolingProfile: 'config' },
  env:    { languageId: 'shell',       icon: '⬥',   iconColor: '#ecc94b', label: 'Environment',     toolingProfile: 'config' },
  // Docs
  md:     { languageId: 'markdown',    icon: '□',   iconColor: '#4a9eff', label: 'Markdown',        toolingProfile: 'docs' },
  mdx:    { languageId: 'markdown',    icon: '□',   iconColor: '#4a9eff', label: 'MDX',             toolingProfile: 'docs' },
  txt:    { languageId: 'plaintext',   icon: '□',   iconColor: '#888',    label: 'Text',            toolingProfile: 'docs' },
  rst:    { languageId: 'plaintext',   icon: '□',   iconColor: '#888',    label: 'reStructuredText', toolingProfile: 'docs' },
  // Web
  html:   { languageId: 'html',        icon: '◈',   iconColor: '#e34f26', label: 'HTML',            toolingProfile: 'web' },
  htm:    { languageId: 'html',        icon: '◈',   iconColor: '#e34f26', label: 'HTML',            toolingProfile: 'web' },
  css:    { languageId: 'css',         icon: '◈',   iconColor: '#563d7c', label: 'CSS',             toolingProfile: 'web' },
  scss:   { languageId: 'scss',        icon: '◈',   iconColor: '#c6538c', label: 'SCSS',            toolingProfile: 'web' },
  sass:   { languageId: 'scss',        icon: '◈',   iconColor: '#c6538c', label: 'Sass',            toolingProfile: 'web' },
  less:   { languageId: 'less',        icon: '◈',   iconColor: '#1d365d', label: 'Less',            toolingProfile: 'web' },
  // Database
  sql:    { languageId: 'sql',         icon: '◧',   iconColor: '#e38d13', label: 'SQL',             toolingProfile: 'data' },
  db:     { languageId: 'sql',         icon: '◧',   iconColor: '#888',    label: 'Database',        toolingProfile: 'data' },
  sqlite: { languageId: 'sql',         icon: '◧',   iconColor: '#003b57', label: 'SQLite',          toolingProfile: 'data' },
  csv:    { languageId: 'plaintext',   icon: '◧',   iconColor: '#89e051', label: 'CSV',             toolingProfile: 'data' },
  // Systems
  go:     { languageId: 'go',          icon: '◆',   iconColor: '#00add8', label: 'Go',              toolingProfile: 'generic' },
  c:      { languageId: 'c',           icon: '◆',   iconColor: '#555596', label: 'C',               toolingProfile: 'generic' },
  cpp:    { languageId: 'cpp',         icon: '◆',   iconColor: '#f34b7d', label: 'C++',             toolingProfile: 'generic' },
  cc:     { languageId: 'cpp',         icon: '◆',   iconColor: '#f34b7d', label: 'C++',             toolingProfile: 'generic' },
  h:      { languageId: 'cpp',         icon: '◇',   iconColor: '#555596', label: 'Header',          toolingProfile: 'generic' },
  hpp:    { languageId: 'cpp',         icon: '◇',   iconColor: '#f34b7d', label: 'C++ Header',      toolingProfile: 'generic' },
  cs:     { languageId: 'csharp',      icon: '◆',   iconColor: '#178600', label: 'C#',              toolingProfile: 'generic' },
  fs:     { languageId: 'fsharp',      icon: '◆',   iconColor: '#b845fc', label: 'F#',              toolingProfile: 'generic' },
  rb:     { languageId: 'ruby',        icon: '◆',   iconColor: '#701516', label: 'Ruby',            toolingProfile: 'generic' },
  kt:     { languageId: 'kotlin',      icon: '◆',   iconColor: '#7f52ff', label: 'Kotlin',          toolingProfile: 'generic' },
  kts:    { languageId: 'kotlin',      icon: '◆',   iconColor: '#7f52ff', label: 'Kotlin Script',   toolingProfile: 'generic' },
  java:   { languageId: 'java',        icon: '◆',   iconColor: '#b07219', label: 'Java',            toolingProfile: 'generic' },
  swift:  { languageId: 'swift',       icon: '◆',   iconColor: '#f05138', label: 'Swift',           toolingProfile: 'generic' },
  dart:   { languageId: 'dart',        icon: '◆',   iconColor: '#00b4ab', label: 'Dart',            toolingProfile: 'generic' },
  zig:    { languageId: 'zig',         icon: '◆',   iconColor: '#ec915c', label: 'Zig',             toolingProfile: 'generic' },
  lua:    { languageId: 'lua',         icon: '◆',   iconColor: '#000080', label: 'Lua',             toolingProfile: 'generic' },
  ex:     { languageId: 'elixir',      icon: '◆',   iconColor: '#6e4a7e', label: 'Elixir',          toolingProfile: 'generic' },
  exs:    { languageId: 'elixir',      icon: '◆',   iconColor: '#6e4a7e', label: 'Elixir Script',   toolingProfile: 'generic' },
  // Lockfiles
  lock:   { languageId: 'plaintext',   icon: '⬥',   iconColor: '#888',    label: 'Lockfile',        toolingProfile: 'config' },
  // Media / binary (treat as generic)
  png:    { languageId: 'plaintext',   icon: '◧',   iconColor: '#888',    label: 'PNG',             toolingProfile: 'generic' },
  jpg:    { languageId: 'plaintext',   icon: '◧',   iconColor: '#888',    label: 'JPEG',            toolingProfile: 'generic' },
  jpeg:   { languageId: 'plaintext',   icon: '◧',   iconColor: '#888',    label: 'JPEG',            toolingProfile: 'generic' },
  gif:    { languageId: 'plaintext',   icon: '◧',   iconColor: '#888',    label: 'GIF',             toolingProfile: 'generic' },
  svg:    { languageId: 'html',        icon: '◧',   iconColor: '#ffb13b', label: 'SVG',             toolingProfile: 'generic' },
  webp:   { languageId: 'plaintext',   icon: '◧',   iconColor: '#888',    label: 'WebP',            toolingProfile: 'generic' },
  ico:    { languageId: 'plaintext',   icon: '◧',   iconColor: '#888',    label: 'Icon',            toolingProfile: 'generic' },
  wasm:   { languageId: 'plaintext',   icon: '◆',   iconColor: '#654ff0', label: 'WASM',            toolingProfile: 'generic' },
};

const DEFAULT_INFO: FileTypeInfo = {
  extension:      '',
  languageId:     'plaintext',
  icon:           '□',
  iconColor:      '#888',
  label:          'Plain Text',
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
