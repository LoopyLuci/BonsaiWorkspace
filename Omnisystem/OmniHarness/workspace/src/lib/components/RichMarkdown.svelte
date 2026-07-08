<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  export let content: string = '';

  interface RichBlock {
    type: 'mermaid' | 'bar_chart' | 'line_chart' | 'pie_chart' | 'math' | 'markdown';
    raw: string;
    data?: { label: string; value: number }[];
  }

  let blocks: RichBlock[] = [];
  let svgCache: Record<string, string> = {};
  let rendered = false;

  // Parse content into typed blocks
  function parseBlocks(src: string): RichBlock[] {
    const result: RichBlock[] = [];
    const fenceRe = /```(mermaid|bar_chart|line_chart|pie_chart|math)([\s\S]*?)```/g;
    let last = 0;
    let match: RegExpExecArray | null;

    while ((match = fenceRe.exec(src)) !== null) {
      if (match.index > last) {
        result.push({ type: 'markdown', raw: src.slice(last, match.index) });
      }
      const blockType = match[1] as RichBlock['type'];
      const inner = match[2].trim();

      if (blockType === 'bar_chart' || blockType === 'line_chart' || blockType === 'pie_chart') {
        // Expect JSON array: [{label, value}, ...]
        let data: { label: string; value: number }[] = [];
        try { data = JSON.parse(inner); } catch { /* use empty */ }
        result.push({ type: blockType, raw: inner, data });
      } else {
        result.push({ type: blockType, raw: inner });
      }
      last = match.index + match[0].length;
    }

    if (last < src.length) {
      result.push({ type: 'markdown', raw: src.slice(last) });
    }
    return result;
  }

  async function renderBlock(block: RichBlock): Promise<string> {
    const key = block.type + ':' + block.raw;
    if (svgCache[key]) return svgCache[key];

    if (block.type === 'markdown') {
      // Basic markdown: bold, italic, code, headings, links
      const html = block.raw
        .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.+?)\*/g, '<em>$1</em>')
        .replace(/`(.+?)`/g, '<code>$1</code>')
        .replace(/^### (.+)$/gm, '<h3>$1</h3>')
        .replace(/^## (.+)$/gm, '<h2>$1</h2>')
        .replace(/^# (.+)$/gm, '<h1>$1</h1>')
        .replace(/\[(.+?)\]\((.+?)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
        .replace(/\n\n/g, '</p><p>')
        .replace(/\n/g, '<br>');
      svgCache[key] = `<p>${html}</p>`;
      return svgCache[key];
    }

    try {
      const svg: string = await invoke('render_rich_block', {
        blockType: block.type,
        content: block.raw,
        data: block.data ?? null,
      });
      svgCache[key] = svg;
      return svg;
    } catch (e) {
      return `<span class="rich-error">Render error: ${e}</span>`;
    }
  }

  let renderedHtml: string[] = [];

  async function render() {
    blocks = parseBlocks(content);
    renderedHtml = await Promise.all(blocks.map(renderBlock));
    rendered = true;
  }

  $: if (content) { rendered = false; render(); }
</script>

<div class="rich-markdown">
  {#if rendered}
    {#each renderedHtml as html}
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html html}
    {/each}
  {:else}
    <span class="rich-loading">Rendering…</span>
  {/if}
</div>

<style>
  .rich-markdown {
    color: #e2e8f0;
    font-size: 14px;
    line-height: 1.6;
  }
  .rich-markdown :global(h1) { font-size: 1.5em; margin: 0.5em 0; }
  .rich-markdown :global(h2) { font-size: 1.25em; margin: 0.5em 0; }
  .rich-markdown :global(h3) { font-size: 1.1em; margin: 0.4em 0; }
  .rich-markdown :global(code) {
    background: #1e293b;
    border-radius: 3px;
    padding: 1px 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.9em;
  }
  .rich-markdown :global(a) { color: #38bdf8; }
  .rich-markdown :global(svg) { display: block; max-width: 100%; margin: 0.75em 0; }
  .rich-markdown :global(.katex-block) {
    display: block;
    padding: 0.5em;
    background: #1e293b;
    border-radius: 6px;
    margin: 0.75em 0;
    font-style: italic;
  }
  .rich-loading { color: #64748b; font-style: italic; }
  .rich-markdown :global(.rich-error) { color: #f87171; font-size: 0.85em; }
</style>
