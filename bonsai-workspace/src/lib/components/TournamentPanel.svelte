<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import SpectatorPanel from './SpectatorPanel.svelte';

  interface Participant {
    agent_id: string;
    name: string;
    score: number;
    wins: number;
    losses: number;
    draws: number;
  }

  interface Tournament {
    id: string;
    name: string;
    game_type: string;
    state: string;
    participants: Participant[];
    pairings: any[];
    created_at: number;
  }

  let tournaments: Tournament[] = [];
  let loading = false;
  let error = '';

  // Create form
  let newName = '';
  let agentsInput = '';
  let creating = false;

  // Spectator
  let spectatingId: string | null = null;
  let spectatingType: 'chess' | 'go' = 'chess';

  // Expanded tournament
  let expandedId: string | null = null;

  onMount(loadTournaments);

  async function loadTournaments() {
    loading = true;
    try {
      tournaments = await invoke<Tournament[]>('list_tournaments');
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function createTournament() {
    if (!newName.trim() || !agentsInput.trim()) return;
    creating = true;
    try {
      const agents = agentsInput.split(',').map(a => a.trim()).filter(Boolean);
      await invoke('create_tournament', {
        name: newName.trim(),
        gameType: 'chess',
        agentIds: agents,
        agentNames: agents,
      });
      newName = '';
      agentsInput = '';
      await loadTournaments();
    } catch (e: any) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  function stateColor(s: string): string {
    switch (s) {
      case 'Running': return 'var(--green, #a6e3a1)';
      case 'Completed': return 'var(--mauve, #cba6f7)';
      case 'Pending': return 'var(--yellow, #f9e2af)';
      default: return 'var(--subtext, #888)';
    }
  }
</script>

<div class="tournament-panel">
  {#if spectatingId}
    <div class="spectator-wrapper">
      <button class="back-btn" on:click={() => spectatingId = null}>← Back to tournaments</button>
      <SpectatorPanel sessionId={spectatingId} gameType={spectatingType} />
    </div>
  {:else}
    <div class="header">
      <h2>Tournaments</h2>
      <button class="refresh-btn" on:click={loadTournaments} disabled={loading}>↻</button>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <!-- Create form -->
    <div class="create-form">
      <h3>New Tournament</h3>
      <div class="form-row">
        <input bind:value={newName} placeholder="Tournament name" class="form-input" />
        <input bind:value={agentsInput} placeholder="Agents (comma-separated IDs)" class="form-input" />
        <button class="create-btn" on:click={createTournament} disabled={creating || !newName || !agentsInput}>
          {creating ? 'Creating…' : 'Create'}
        </button>
      </div>
    </div>

    <!-- Tournament list -->
    {#if loading}
      <div class="loading">Loading tournaments…</div>
    {:else if tournaments.length === 0}
      <div class="empty">No tournaments yet. Create one above.</div>
    {:else}
      <div class="tournament-list">
        {#each tournaments as t}
          <div class="tournament-card" class:expanded={expandedId === t.id}>
            <div class="card-header" on:click={() => expandedId = expandedId === t.id ? null : t.id}
              role="button" tabindex="0" on:keydown={e => e.key === 'Enter' && (expandedId = expandedId === t.id ? null : t.id)}>
              <div class="card-title">
                <span class="t-name">{t.name}</span>
                <span class="t-type">{t.game_type}</span>
              </div>
              <div class="card-meta">
                <span class="state-badge" style="color: {stateColor(t.state)}">{t.state}</span>
                <span class="participant-count">{t.participants.length} players</span>
                <span class="chevron">{expandedId === t.id ? '▲' : '▼'}</span>
              </div>
            </div>

            {#if expandedId === t.id}
              <div class="standings">
                <h4>Standings</h4>
                <table class="standings-table">
                  <thead>
                    <tr><th>#</th><th>Player</th><th>Score</th><th>W</th><th>D</th><th>L</th></tr>
                  </thead>
                  <tbody>
                    {#each t.participants.sort((a, b) => b.score - a.score) as p, i}
                      <tr class:leader={i === 0}>
                        <td>{i + 1}</td>
                        <td class="player-name">{p.name}</td>
                        <td class="score">{p.score.toFixed(1)}</td>
                        <td>{p.wins}</td>
                        <td>{p.draws}</td>
                        <td>{p.losses}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>

                {#if t.pairings.filter(p => p.session_id).length > 0}
                  <h4>Active Games</h4>
                  <div class="pairings">
                    {#each t.pairings.filter(p => p.session_id) as pairing}
                      <div class="pairing">
                        <span>{pairing.white} vs {pairing.black}</span>
                        {#if pairing.result}
                          <span class="pairing-result">{pairing.result}</span>
                        {:else}
                          <button class="watch-btn" on:click={() => { spectatingId = pairing.session_id; spectatingType = t.game_type as 'chess' | 'go'; }}>
                            Watch
                          </button>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .tournament-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--surface-1, #181825);
    border-radius: 12px;
    height: 100%;
    overflow-y: auto;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .header h2 { margin: 0; font-size: 20px; color: var(--text, #cdd6f4); }
  .refresh-btn {
    background: none; border: none; color: var(--subtext, #888);
    font-size: 18px; cursor: pointer;
  }
  .refresh-btn:hover { color: var(--text, #cdd6f4); }

  .error { color: var(--red, #f38ba8); font-size: 13px; }
  .loading, .empty { color: var(--subtext, #888); font-size: 13px; }

  .create-form {
    background: var(--surface-2, #1e1e2e);
    padding: 14px;
    border-radius: 10px;
  }
  .create-form h3 { margin: 0 0 10px; font-size: 15px; color: var(--text, #cdd6f4); }
  .form-row { display: flex; gap: 8px; flex-wrap: wrap; }
  .form-input {
    flex: 1;
    min-width: 150px;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--surface-3, #313244);
    color: var(--text, #cdd6f4);
    border: 1px solid var(--surface-3, #45475a);
    font-size: 13px;
    outline: none;
  }
  .form-input:focus { border-color: var(--mauve, #cba6f7); }
  .create-btn {
    padding: 6px 16px;
    border-radius: 6px;
    background: var(--mauve, #cba6f7);
    color: #1e1e2e;
    font-weight: 700;
    font-size: 13px;
    border: none;
    cursor: pointer;
  }
  .create-btn:disabled { opacity: 0.5; cursor: default; }

  .tournament-list { display: flex; flex-direction: column; gap: 8px; }

  .tournament-card {
    background: var(--surface-2, #1e1e2e);
    border-radius: 10px;
    border: 1px solid var(--surface-3, #313244);
    overflow: hidden;
    transition: border-color 0.15s;
  }
  .tournament-card.expanded { border-color: var(--mauve, #cba6f7); }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 14px;
    cursor: pointer;
  }
  .card-header:hover { background: var(--surface-3, #313244); }

  .card-title { display: flex; align-items: center; gap: 8px; }
  .t-name { font-weight: 600; color: var(--text, #cdd6f4); font-size: 15px; }
  .t-type {
    font-size: 11px; padding: 2px 6px;
    background: var(--surface-3, #313244);
    border-radius: 4px;
    color: var(--subtext, #888);
    text-transform: uppercase;
  }

  .card-meta { display: flex; align-items: center; gap: 10px; font-size: 12px; }
  .state-badge { font-weight: 600; }
  .participant-count { color: var(--subtext, #888); }
  .chevron { color: var(--subtext, #888); }

  .standings { padding: 12px 14px; }
  .standings h4 { margin: 0 0 8px; font-size: 13px; color: var(--subtext, #888); text-transform: uppercase; letter-spacing: 0.05em; }

  .standings-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .standings-table th {
    text-align: left;
    padding: 4px 6px;
    color: var(--subtext, #888);
    font-weight: 500;
    border-bottom: 1px solid var(--surface-3, #313244);
  }
  .standings-table td { padding: 5px 6px; color: var(--text, #cdd6f4); }
  .standings-table tr.leader td { color: var(--yellow, #f9e2af); font-weight: 600; }
  .player-name { font-weight: 500; }
  .score { color: var(--mauve, #cba6f7); font-weight: 600; }

  .pairings { display: flex; flex-direction: column; gap: 6px; }
  .pairing {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 8px;
    background: var(--surface-3, #313244);
    border-radius: 6px;
    font-size: 13px;
    color: var(--text, #cdd6f4);
  }
  .pairing-result { color: var(--mauve, #cba6f7); font-weight: 600; }
  .watch-btn {
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 12px;
    background: var(--surface-1, #181825);
    color: var(--green, #a6e3a1);
    border: 1px solid var(--green, #a6e3a1);
    cursor: pointer;
  }
  .watch-btn:hover { background: var(--green, #a6e3a1); color: #000; }

  .spectator-wrapper { display: flex; flex-direction: column; gap: 10px; }
  .back-btn {
    align-self: flex-start;
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 13px;
    background: var(--surface-2, #1e1e2e);
    color: var(--subtext, #888);
    border: 1px solid var(--surface-3, #313244);
    cursor: pointer;
  }
  .back-btn:hover { color: var(--text, #cdd6f4); }
</style>
