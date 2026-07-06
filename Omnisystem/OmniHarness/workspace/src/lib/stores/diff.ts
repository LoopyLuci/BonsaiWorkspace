import { writable } from 'svelte/store';

export interface DiffHunk {
  hunkIndex: number;
  startLine: number;
  endLine:   number;
  type:      'insert' | 'delete' | 'replace';
  newText:   string;
  oldText:   string;
}

export interface CurrentDiff {
  filePath:       string;
  hunks:          DiffHunk[];
  rawUnifiedDiff: string;
}

export const currentDiff  = writable<CurrentDiff | null>(null);
export const pendingDiffs  = writable<Map<string, CurrentDiff>>(new Map());

/** Parse a unified diff string into structured hunks. */
export function parseUnifiedDiff(diffText: string): DiffHunk[] {
  const lines   = diffText.split('\n');
  const hunks:  DiffHunk[] = [];
  let hi        = 0;
  let newLine   = 1;
  let hunkStart = 1;
  let oldText   = '';
  let newText   = '';
  let inHunk    = false;

  function flush() {
    if (!inHunk) return;
    const type: DiffHunk['type'] =
      oldText && newText ? 'replace' :
      newText            ? 'insert'  : 'delete';
    hunks.push({ hunkIndex: hi++, startLine: hunkStart, endLine: newLine, type, newText, oldText });
    oldText = '';
    newText = '';
    inHunk  = false;
  }

  for (const line of lines) {
    if (line.startsWith('@@')) {
      flush();
      const m = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
      hunkStart = m ? +m[2] : newLine;
      newLine   = hunkStart;
      inHunk    = true;
      continue;
    }
    if (!inHunk) continue;
    if (line.startsWith('+') && !line.startsWith('+++')) {
      newText += line.slice(1) + '\n';
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      oldText += line.slice(1) + '\n';
    } else if (line.startsWith(' ')) {
      newLine++;
    }
  }
  flush();
  return hunks;
}

export function receiveAgentDiff(filePath: string, unifiedDiff: string) {
  const hunks = parseUnifiedDiff(unifiedDiff);
  const diff: CurrentDiff = { filePath, hunks, rawUnifiedDiff: unifiedDiff };
  currentDiff.set(diff);
  pendingDiffs.update((m) => { m.set(filePath, diff); return m; });
}

export function clearCurrentDiff() {
  currentDiff.set(null);
}

export function clearDiffForFile(path: string) {
  pendingDiffs.update((m) => { m.delete(path); return m; });
}
