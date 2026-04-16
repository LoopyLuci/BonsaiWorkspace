import { writable } from 'svelte/store';
import { detectFileType, type ToolingProfileId } from '$lib/utils/filetypes';

export type ToolingCommandKind = 'load' | 'lint' | 'format' | 'test';

export type ToolingProfile = {
  id: ToolingProfileId;
  title: string;
  enabled: boolean;
  loadCommand: string;
  lintCommand: string;
  formatCommand: string;
  testCommand: string;
  languageTools: string[];
};

export type ToolingProfileMap = Record<ToolingProfileId, ToolingProfile>;

const STORAGE_KEY = 'bonsai-editor-tooling-profiles-v1';

function defaults(): ToolingProfileMap {
  return {
    web: {
      id: 'web',
      title: 'Web / Node',
      enabled: true,
      loadCommand: 'npm install -D eslint prettier @typescript-eslint/parser @typescript-eslint/eslint-plugin',
      lintCommand: 'npx eslint "{file}"',
      formatCommand: 'npx prettier --write "{file}"',
      testCommand: 'npm test',
      languageTools: ['TypeScript IntelliSense', 'ESLint', 'Prettier'],
    },
    python: {
      id: 'python',
      title: 'Python',
      enabled: true,
      loadCommand: 'pip install ruff black',
      lintCommand: 'python -m ruff check "{file}"',
      formatCommand: 'python -m ruff format "{file}"',
      testCommand: 'python -m pytest',
      languageTools: ['Pylance', 'Ruff', 'Pytest'],
    },
    rust: {
      id: 'rust',
      title: 'Rust',
      enabled: true,
      loadCommand: 'rustup component add clippy rustfmt',
      lintCommand: 'cargo clippy',
      formatCommand: 'cargo fmt',
      testCommand: 'cargo test',
      languageTools: ['rust-analyzer', 'clippy', 'rustfmt'],
    },
    powershell: {
      id: 'powershell',
      title: 'PowerShell',
      enabled: true,
      loadCommand: 'Install-Module -Name PSScriptAnalyzer -Scope CurrentUser -Force',
      lintCommand: 'powershell -NoProfile -Command "Invoke-ScriptAnalyzer -Path \"{file}\""',
      formatCommand: '',
      testCommand: 'powershell -NoProfile -Command "Invoke-Pester"',
      languageTools: ['PowerShell Language Server', 'PSScriptAnalyzer', 'Pester'],
    },
    shell: {
      id: 'shell',
      title: 'Shell',
      enabled: true,
      loadCommand: '',
      lintCommand: 'shellcheck "{file}"',
      formatCommand: 'shfmt -w "{file}"',
      testCommand: '',
      languageTools: ['ShellCheck', 'shfmt'],
    },
    config: {
      id: 'config',
      title: 'Config Files',
      enabled: true,
      loadCommand: 'npm install -D prettier',
      lintCommand: '',
      formatCommand: 'npx prettier --write "{file}"',
      testCommand: '',
      languageTools: ['YAML/JSON validation', 'Prettier'],
    },
    docs: {
      id: 'docs',
      title: 'Docs / Markdown',
      enabled: true,
      loadCommand: 'npm install -D prettier markdownlint-cli',
      lintCommand: 'npx markdownlint "{file}"',
      formatCommand: 'npx prettier --write "{file}"',
      testCommand: '',
      languageTools: ['Markdown lint', 'Markdown preview', 'Prettier'],
    },
    data: {
      id: 'data',
      title: 'Data / SQL',
      enabled: true,
      loadCommand: '',
      lintCommand: '',
      formatCommand: '',
      testCommand: '',
      languageTools: ['JSON schema validation', 'SQL syntax highlighting'],
    },
    generic: {
      id: 'generic',
      title: 'Generic',
      enabled: true,
      loadCommand: '',
      lintCommand: '',
      formatCommand: '',
      testCommand: '',
      languageTools: ['Syntax highlighting'],
    },
  };
}

export const editorToolingProfiles = writable<ToolingProfileMap>(defaults());

export function loadEditorToolingSettings(): void {
  if (typeof window === 'undefined') return;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Partial<ToolingProfileMap>;
    editorToolingProfiles.update((base) => ({ ...base, ...parsed }));
  } catch {
    // Ignore corrupt storage and use defaults.
  }
}

export function saveEditorToolingSettings(map: ToolingProfileMap): void {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(map));
}

export function updateToolingProfile(id: ToolingProfileId, patch: Partial<ToolingProfile>): void {
  editorToolingProfiles.update((current) => {
    const next: ToolingProfileMap = {
      ...current,
      [id]: {
        ...current[id],
        ...patch,
      },
    };
    saveEditorToolingSettings(next);
    return next;
  });
}

export function setToolingLanguageTools(id: ToolingProfileId, value: string): void {
  const tools = value
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  updateToolingProfile(id, { languageTools: tools });
}

export function resetEditorToolingSettings(): void {
  const next = defaults();
  editorToolingProfiles.set(next);
  saveEditorToolingSettings(next);
}

export function resolveToolingProfileId(path: string): ToolingProfileId {
  return detectFileType(path).toolingProfile;
}

export function buildToolCommand(
  map: ToolingProfileMap,
  filePath: string,
  workspacePath: string,
  kind: ToolingCommandKind,
): string {
  const profileId = resolveToolingProfileId(filePath);
  const profile = map[profileId] ?? map.generic;
  if (!profile || !profile.enabled) return '';

  const template =
    kind === 'load'
      ? profile.loadCommand
      : kind === 'lint'
      ? profile.lintCommand
      : kind === 'format'
      ? profile.formatCommand
      : profile.testCommand;

  if (!template.trim()) return '';

  const dir = filePath.replace(/[\\/][^\\/]+$/, '');
  return template
    .replaceAll('{file}', filePath)
    .replaceAll('{workspace}', workspacePath)
    .replaceAll('{dir}', dir);
}
