<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import {
    swarms, swarmsError, selectedSwarmId, selectedHierarchy, selectedLedger, selectedTasks,
    refreshSwarmList, selectSwarm, refreshSwarmDetail, cancelSwarm,
    describeStatus, describeLedgerEvent, statusIsTerminal,
    type SwarmSnapshot, type HierarchyNode,
  } from '$lib/stores/swarmCommander';

  let pollHandle: ReturnType<typeof setInterval> | undefined;

  onMount(() => {
    refreshSwarmList();
    // Live view of real, currently-running swarms — polling (rather than a
    // dedicated event stream) keeps this panel simple; every value shown
    // still comes from the real bridge, just refreshed every few seconds
    // instead of instantly.
    pollHandle = setInterval(async () => {
      await refreshSwarmList();
      const id = get(selectedSwarmId);
      if (id) await refreshSwarmDetail(id);
    }, 3000);
  });

  onDestroy(() => {
    if (pollHandle) clearInterval(pollHandle);
  });

  function rootNodes(nodes: HierarchyNode[]): HierarchyNode[] {
    return nodes.filter((n) => !n.parent_id);
  }
  function childrenOf(nodes: HierarchyNode[], parentId: string): HierarchyNode[] {
    return nodes.filter((n) => n.parent_id === parentId);
  }

  function statusClass(status: unknown): string {
    const s = describeStatus(status as never);
    if (s.includes('working') || s.includes('running')) return 'status-active';
    if (s.includes('error') || s.includes('failed')) return 'status-error';
    if (s.includes('completed')) return 'status-done';
    return 'status-idle';
  }

  async function onCancel(swarmId: string) {
    if (!confirm('Cancel this swarm run? In-flight worker inference will be stopped.')) return;
    await cancelSwarm(swarmId);
  }
</script>

<div class="commander-panel">
  <p class="commander-hint">
    Real-time view of Custom Swarm runs (start one from the chat panel by selecting "Custom Swarm" in the model picker) — hierarchy, task progress, and audit ledger for each run.
  </p>

  {#if $swarmsError}<p class="error-msg">{$swarmsError}</p>{/if}

  <div class="commander-body">
    <div class="swarm-list">
      {#if $swarms.length === 0}
        <div class="empty-state">No swarm runs yet.</div>
      {:else}
        {#each $swarms as s (s.id)}
          <button
            class="swarm-list-item"
            class:selected={$selectedSwarmId === s.id}
            on:click={() => selectSwarm(s.id)}
          >
            <span class="swarm-name">{s.name}</span>
            <span class="swarm-status {statusClass(s.status)}">{describeStatus(s.status)}</span>
            <span class="swarm-progress-bar">
              <span class="swarm-progress-fill" style="width:{Math.round(s.progress * 100)}%"></span>
            </span>
            <span class="swarm-meta">{s.task_completed}/{s.task_total} tasks · {s.agent_stats.total} agents</span>
          </button>
        {/each}
      {/if}
    </div>

    {#if $selectedSwarmId}
      {@const current = $swarms.find((s) => s.id === $selectedSwarmId)}
      <div class="swarm-detail">
        {#if current}
          <div class="detail-header">
            <div>
              <div class="detail-title">{current.name}</div>
              <div class="detail-goal">{current.goal}</div>
            </div>
            {#if !statusIsTerminal(current.status)}
              <button class="btn-small btn-warning" on:click={() => onCancel(current.id)}>Cancel Run</button>
            {/if}
          </div>

          <div class="detail-section">
            <div class="section-title">Agents ({$selectedHierarchy.length})</div>
            <div class="hierarchy-tree">
              {#each rootNodes($selectedHierarchy) as root (root.id)}
                <div class="hierarchy-node {statusClass(root.status)}">
                  <span class="node-name">{root.display_name}</span>
                  <span class="node-role">{root.role}</span>
                  <span class="node-status">{describeStatus(root.status)}</span>
                  {#if root.current_task}<span class="node-task">{root.current_task}</span>{/if}
                </div>
                {#each childrenOf($selectedHierarchy, root.id) as child (child.id)}
                  <div class="hierarchy-node child {statusClass(child.status)}">
                    <span class="node-name">↳ {child.display_name}</span>
                    <span class="node-role">{child.role}</span>
                    <span class="node-status">{describeStatus(child.status)}</span>
                    {#if child.current_task}<span class="node-task">{child.current_task}</span>{/if}
                  </div>
                {/each}
              {/each}
            </div>
          </div>

          <div class="detail-section">
            <div class="section-title">Tasks ({$selectedTasks.length})</div>
            <div class="task-list">
              {#each $selectedTasks as t (t.id)}
                <div class="task-row {statusClass(t.status)}">
                  <span class="task-desc">{t.description}</span>
                  <span class="task-status">{describeStatus(t.status)}</span>
                </div>
              {/each}
            </div>
          </div>

          <div class="detail-section">
            <div class="section-title">Ledger ({$selectedLedger.length} entries)</div>
            <div class="ledger-list">
              {#each $selectedLedger as entry (entry.seq)}
                <div class="ledger-row">
                  <span class="ledger-seq">#{entry.seq}</span>
                  <span class="ledger-event">{describeLedgerEvent(entry.event)}</span>
                  <span class="ledger-time">{new Date(entry.timestamp).toLocaleTimeString()}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .commander-panel { display: flex; flex-direction: column; gap: 10px; height: 100%; }
  .commander-hint { font-size: 12px; color: var(--text-dim); margin: 0; }
  .empty-state { color: var(--text-dim); font-size: 13px; padding: 24px 0; text-align: center; }
  .error-msg { color: #f87171; font-size: 12px; margin: 0; }

  .commander-body { display: grid; grid-template-columns: 260px 1fr; gap: 12px; min-height: 0; flex: 1; overflow: hidden; }

  .swarm-list { display: flex; flex-direction: column; gap: 6px; overflow-y: auto; }
  .swarm-list-item {
    display: flex; flex-direction: column; gap: 4px; text-align: left;
    padding: 8px 10px; border-radius: 8px; border: 1px solid var(--border);
    background: var(--bg); color: var(--text); cursor: pointer; font-size: 12px;
  }
  .swarm-list-item.selected { border-color: #4a9eff; background: rgba(74,158,255,0.08); }
  .swarm-name { font-weight: 600; }
  .swarm-status { font-size: 11px; }
  .swarm-progress-bar { height: 4px; border-radius: 2px; background: rgba(255,255,255,0.08); overflow: hidden; }
  .swarm-progress-fill { display: block; height: 100%; background: #4a9eff; }
  .swarm-meta { font-size: 10px; color: var(--text-dim); }

  .status-active { color: #4a9eff; }
  .status-error { color: #f87171; }
  .status-done { color: #22c55e; }
  .status-idle { color: var(--text-dim); }

  .swarm-detail { overflow-y: auto; display: flex; flex-direction: column; gap: 14px; padding-right: 4px; }
  .detail-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .detail-title { font-weight: 600; font-size: 15px; }
  .detail-goal { font-size: 12px; color: var(--text-dim); }

  .detail-section { display: flex; flex-direction: column; gap: 6px; }
  .section-title { font-size: 12px; font-weight: 600; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.03em; }

  .hierarchy-tree { display: flex; flex-direction: column; gap: 4px; }
  .hierarchy-node {
    display: grid; grid-template-columns: 1fr auto auto 2fr; gap: 8px; align-items: center;
    padding: 6px 8px; border-radius: 6px; background: var(--bg); border: 1px solid var(--border); font-size: 12px;
  }
  .hierarchy-node.child { margin-left: 16px; }
  .node-name { font-weight: 500; }
  .node-role { font-size: 10px; color: var(--text-dim); }
  .node-task { font-size: 11px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .task-list { display: flex; flex-direction: column; gap: 4px; }
  .task-row {
    display: flex; justify-content: space-between; gap: 10px;
    padding: 6px 8px; border-radius: 6px; background: var(--bg); border: 1px solid var(--border); font-size: 12px;
  }
  .task-desc { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .ledger-list { display: flex; flex-direction: column; gap: 2px; max-height: 240px; overflow-y: auto; }
  .ledger-row {
    display: grid; grid-template-columns: auto 1fr auto; gap: 8px;
    padding: 4px 8px; font-size: 11px; color: var(--text-dim);
  }
  .ledger-seq { color: var(--text); font-weight: 600; }

  .btn-small {
    padding: 5px 10px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--bg); color: var(--text); font-size: 12px; cursor: pointer;
  }
  .btn-small.btn-warning { border-color: rgba(245,158,11,0.4); color: #f59e0b; }
</style>
