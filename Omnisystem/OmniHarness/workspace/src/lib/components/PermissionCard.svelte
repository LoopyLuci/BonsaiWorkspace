<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { PermissionCardData } from '$lib/stores/chat';

  export let card: PermissionCardData;

  const dispatch = createEventDispatcher<{ approve: void; deny: void }>();

  let confirmText = '';
  $: needsConfirm = card.type === 'file_delete';
  $: confirmTarget = card.paths_affected?.[0] ?? '';
  $: canApprove = !needsConfirm || confirmText.trim() === confirmTarget.trim();
</script>

<div class="perm-card" class:danger={card.type === 'file_delete' || card.type === 'shell_command'}>
  <div class="perm-header">
    <span class="perm-icon">
      {#if card.type === 'file_delete'}🗑{:else if card.type === 'shell_command'}⚡{:else}🔐{/if}
    </span>
    <span class="perm-type">
      {#if card.type === 'file_delete'}Delete file{:else if card.type === 'shell_command'}Shell command{:else}Permission required{/if}
    </span>
  </div>

  <p class="perm-rationale">{card.rationale ?? card.description ?? ''}</p>

  {#if card.paths_affected?.length}
    <div class="perm-paths">
      {#each card.paths_affected as p}
        <code class="path-chip">{p}</code>
      {/each}
    </div>
  {/if}

  {#if card.command}
    <pre class="perm-cmd">{card.command}</pre>
  {/if}

  {#if needsConfirm}
    <label class="confirm-label">
      Type <strong>{confirmTarget}</strong> to confirm deletion:
      <input
        class="confirm-input"
        bind:value={confirmText}
        placeholder={confirmTarget}
        autocomplete="off"
        spellcheck="false"
      />
    </label>
  {/if}

  <div class="perm-actions">
    <button
      class="btn-approve"
      disabled={!canApprove}
      on:click={() => dispatch('approve')}
    >Approve</button>
    <button class="btn-deny" on:click={() => dispatch('deny')}>Deny</button>
  </div>
</div>

<style>
  .perm-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 8px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
  }
  .perm-card.danger { border-left-color: var(--red); }

  .perm-header { display: flex; align-items: center; gap: 6px; }
  .perm-icon   { font-size: 14px; }
  .perm-type   { font-weight: 600; font-size: 12px; }

  .perm-rationale { color: var(--text-dim); line-height: 1.5; }

  .perm-paths { display: flex; flex-wrap: wrap; gap: 4px; }
  .path-chip  {
    background: var(--bg2);
    border: 1px solid var(--border);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }

  .perm-cmd {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 11px;
    font-family: monospace;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .confirm-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--text-dim);
  }
  .confirm-input {
    background: var(--bg2);
    border: 1px solid var(--red);
    border-radius: 5px;
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text);
    font-family: monospace;
    outline: none;
  }
  .confirm-input:focus { border-color: var(--accent); }

  .perm-actions { display: flex; gap: 6px; }

  .btn-approve {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 5px;
    padding: 5px 14px;
    font-size: 12px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .btn-approve:hover:not(:disabled) { opacity: 0.85; }
  .btn-approve:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-deny {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 5px 14px;
    font-size: 12px;
    color: var(--text-dim);
    cursor: pointer;
  }
  .btn-deny:hover { background: var(--bg-hover); color: var(--text); }
</style>
