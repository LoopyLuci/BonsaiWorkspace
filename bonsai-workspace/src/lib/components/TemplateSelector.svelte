<script lang="ts">
  import { invoke }              from '@tauri-apps/api/core';
  import { setWorkspace }        from '$lib/stores/workspace';
  import { addAssistantMessage } from '$lib/stores/chat';

  const templates = [
    { id: 'nextjs',   name: 'Next.js + Tailwind',  desc: 'Modern full-stack React app',          icon: '⚡' },
    { id: 'fastapi',  name: 'FastAPI + SQLModel',   desc: 'Python REST API with typed DB layer',  icon: '🐍' },
    { id: 'rust-cli', name: 'Rust CLI Tool',        desc: 'High-performance command-line utility', icon: '🦀' },
    { id: 'svelte',   name: 'SvelteKit App',        desc: 'Full-stack Svelte with file routing',  icon: '🔶' },
  ];

  let creating   = false;
  let status     = '';
  let errorMsg   = '';

  async function createProject(templateId: string, templateName: string) {
    if (creating) return;
    const name = prompt(`Project name for "${templateName}"?`)?.trim();
    if (!name) return;
    creating = true;
    status   = `Creating ${name}…`;
    errorMsg = '';
    try {
      const projectPath = await invoke<string>('create_project_from_template', {
        templateId,
        projectName: name,
      });
      setWorkspace(projectPath);
      status = `AI scaffolding ${name}…`;
      const result = await invoke<string>('ai_scaffold_project', {
        projectPath,
        templateId,
        userPrompt: `Scaffold a complete ${templateName} project called "${name}".`,
      });
      addAssistantMessage(`✅ **${name}** scaffolded!\n\n${result}`);
      status = '✓ Done';
    } catch (e) {
      errorMsg = String(e);
      status   = '';
    } finally {
      creating = false;
    }
  }
</script>

<div class="template-selector">
  <h2 class="title">Start with a template</h2>
  <p class="sub">Bonsai will scaffold a complete project structure with AI</p>

  <div class="grid">
    {#each templates as t}
      <button
        class="card"
        on:click={() => createProject(t.id, t.name)}
        disabled={creating}
        aria-label="Create {t.name} project"
      >
        <span class="card-icon">{t.icon}</span>
        <span class="card-name">{t.name}</span>
        <span class="card-desc">{t.desc}</span>
      </button>
    {/each}
  </div>

  {#if status}
    <div class="status">{status}</div>
  {/if}
  {#if errorMsg}
    <div class="error">{errorMsg}</div>
  {/if}
</div>

<style>
  .template-selector { padding: 32px; }
  .title { font-size: 18px; font-weight: 700; margin-bottom: 4px; }
  .sub   { font-size: 13px; color: var(--text-dim); margin-bottom: 20px; }
  .grid  { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }

  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 18px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s, box-shadow 0.15s;
    color: var(--text);
  }
  .card:hover:not(:disabled) { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .card:disabled { opacity: 0.5; cursor: not-allowed; }

  .card-icon { font-size: 24px; }
  .card-name { font-size: 14px; font-weight: 600; }
  .card-desc { font-size: 12px; color: var(--text-dim); }

  .status { margin-top: 16px; font-size: 13px; color: var(--text-dim); }
  .error  { margin-top: 12px; font-size: 13px; color: var(--red); }
</style>
