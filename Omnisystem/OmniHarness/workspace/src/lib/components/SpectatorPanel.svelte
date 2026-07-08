<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import ChessBoard from './ChessBoard.svelte';
  import GoBoard from './GoBoard.svelte';

  export let sessionId: string;
  export let gameType: 'chess' | 'go' = 'chess';

  interface ThinkingState {
    agent: string;
    bestMove: string;
    valuePct: number;
    simulations: number;
    topMoves: string[];
    active: boolean;
  }

  let gameState: any = null;
  let error = '';
  let thinking: ThinkingState = { agent: '', bestMove: '', valuePct: 50, simulations: 0, topMoves: [], active: false };
  let moveHistory: string[] = [];
  let unlisteners: Array<() => void> = [];

  onMount(async () => {
    await loadState();
    setupListeners();
  });

  onDestroy(() => {
    unlisteners.forEach(f => f());
  });

  async function loadState() {
    try {
      gameState = await invoke('spectate_game', { gameId: sessionId, gameType });
    } catch (e: any) {
      error = String(e);
    }
  }

  function setupListeners() {
    listen<any>('agent-thinking-started', (e) => {
      if (e.payload.session_id !== sessionId) return;
      thinking = { ...thinking, agent: e.payload.agent, active: true, topMoves: [], bestMove: '' };
    }).then(f => unlisteners.push(f));

    listen<any>('agent-thinking-complete', (e) => {
      if (e.payload.session_id !== sessionId) return;
      thinking = {
        agent: e.payload.agent,
        bestMove: e.payload.best_move ?? e.payload.best_move ?? '',
        valuePct: e.payload.value_pct ?? Math.round((e.payload.value ?? 0.5) * 100),
        simulations: e.payload.simulations ?? 0,
        topMoves: e.payload.top_moves ?? [],
        active: false,
      };
      // Reload board state
      loadState();
    }).then(f => unlisteners.push(f));

    listen<any>('game-state-update', (e) => {
      loadState();
    }).then(f => unlisteners.push(f));
  }

  $: evalBar = thinking.valuePct;
</script>

<div class="spectator">
  {#if error}
    <div class="error">{error}</div>
  {:else if !gameState}
    <div class="loading">Loading game…</div>
  {:else}
    <!-- Evaluation bar -->
    <div class="eval-bar-container" title="White advantage">
      <div class="eval-bar" style="height: {evalBar}%"></div>
      <span class="eval-label white-label">{evalBar}%</span>
      <span class="eval-label black-label">{100 - evalBar}%</span>
    </div>

    <div class="board-area">
      {#if gameType === 'chess'}
        <ChessBoard gameId={sessionId} humanColor="white" playerName="Spectator" />
      {:else}
        <GoBoard gameId={sessionId} humanColor="black" playerName="Spectator"
          boardSize={/** @type {9|13|19} */ (gameState.board_size ?? 19)} />
      {/if}
    </div>

    <!-- Agent thinking panel -->
    {#if thinking.active}
      <div class="thinking-panel">
        <span class="pulse">●</span>
        <span class="thinking-agent">{thinking.agent} is thinking…</span>
      </div>
    {:else if thinking.bestMove}
      <div class="thinking-panel complete">
        <span class="agent-move">
          {thinking.agent} played <strong>{thinking.bestMove}</strong>
        </span>
        <span class="sims">{thinking.simulations.toLocaleString()} sims</span>
        {#if thinking.topMoves.length > 0}
          <div class="top-moves">
            {#each thinking.topMoves as m}
              <span class="top-move">{m}</span>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="game-meta">
      <span>Game: <code>{sessionId.slice(0, 8)}…</code></span>
      <span>Result: {gameState.result}</span>
    </div>
  {/if}
</div>

<style>
  .spectator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 12px;
    background: var(--surface-1, #181825);
    border-radius: 12px;
    position: relative;
  }

  .error { color: var(--red, #f38ba8); font-size: 13px; }
  .loading { color: var(--subtext, #888); font-size: 14px; }

  .eval-bar-container {
    width: 100%;
    max-width: 448px;
    height: 10px;
    background: #333;
    border-radius: 5px;
    position: relative;
    overflow: hidden;
  }
  .eval-bar {
    position: absolute;
    bottom: 0;
    width: 100%;
    background: #eee;
    transition: height 0.4s ease;
  }
  .eval-label {
    position: absolute;
    font-size: 9px;
    font-weight: 700;
  }
  .white-label { left: 4px; top: 0; color: #111; }
  .black-label { right: 4px; bottom: 0; color: #eee; }

  .board-area { width: 100%; display: flex; justify-content: center; }

  .thinking-panel {
    width: 100%;
    max-width: 448px;
    padding: 8px 12px;
    background: var(--surface-2, #1e1e2e);
    border-radius: 8px;
    font-size: 13px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    border-left: 3px solid var(--yellow, #f9e2af);
  }
  .thinking-panel.complete { border-left-color: var(--green, #a6e3a1); }

  .pulse {
    color: var(--yellow, #f9e2af);
    animation: blink 1s ease infinite;
  }
  @keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0.2; } }

  .thinking-agent { color: var(--yellow, #f9e2af); }
  .agent-move { color: var(--text, #cdd6f4); }
  .sims { color: var(--subtext, #888); font-size: 11px; }

  .top-moves {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    width: 100%;
  }
  .top-move {
    background: var(--surface-3, #313244);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
    color: var(--mauve, #cba6f7);
  }

  .game-meta {
    display: flex;
    gap: 16px;
    font-size: 11px;
    color: var(--subtext, #585b70);
  }
</style>
