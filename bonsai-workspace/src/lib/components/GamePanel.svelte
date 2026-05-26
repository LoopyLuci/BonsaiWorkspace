<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import ChessBoard from './ChessBoard.svelte';
  import GoBoard from './GoBoard.svelte';
  import TournamentPanel from './TournamentPanel.svelte';

  type ActiveTab = 'play' | 'tournaments' | 'puzzles';
  type GameType = 'chess' | 'go';

  let activeTab: ActiveTab = 'play';
  let activeGame: GameType | null = null;
  let chessGameId: string | null = null;
  let goGameId: string | null = null;

  // Chess settings
  let chessPlayerName = 'Player';
  let chessColor: 'white' | 'black' = 'white';
  let chessStrength: 'interactive' | 'strong' = 'interactive';

  // Go settings
  let goPlayerName = 'Player';
  let goColor: 'black' | 'white' = 'black';
  let goBoardSize: 9 | 13 | 19 = 19;
  let goKomi = 7.5;

  // Agent thinking state
  let agentThinking = false;
  let agentBestMove = '';
  let agentValuePct = 50;
  let agentSimulations = 0;
  let agentTopMoves: string[] = [];
  let unlisteners: Array<() => void> = [];

  // Puzzle state
  let dailyPuzzle: any = null;
  let puzzleInput = '';
  let puzzleMsg = '';

  onMount(() => {
    listen<any>('agent-thinking-started', e => { agentThinking = true; agentBestMove = ''; agentTopMoves = []; }).then(f => unlisteners.push(f));
    listen<any>('agent-thinking-complete', e => {
      agentThinking = false;
      agentBestMove = e.payload.best_move ?? '';
      agentValuePct = e.payload.value_pct ?? Math.round((e.payload.value ?? 0.5) * 100);
      agentSimulations = e.payload.simulations ?? 0;
      agentTopMoves = e.payload.top_moves ?? [];
    }).then(f => unlisteners.push(f));
  });

  onDestroy(() => unlisteners.forEach(f => f()));

  function startChess() { activeGame = 'chess'; chessGameId = null; }
  function startGo()    { activeGame = 'go';    goGameId = null; }
  function backToMenu() { activeGame = null; }

  function onChessCreated(e: CustomEvent) { chessGameId = e.detail.id; }
  function onGoCreated(e: CustomEvent)    { goGameId = e.detail.id; }

  async function exportGame() {
    if (activeGame === 'chess' && chessGameId) {
      const pgn: string = await invoke('export_chess_pgn', { gameId: chessGameId });
      const blob = new Blob([pgn], { type: 'text/plain' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
      a.download = `game-${chessGameId.slice(0,8)}.pgn`; a.click();
    } else if (activeGame === 'go' && goGameId) {
      const sgf: string = await invoke('export_go_sgf', { gameId: goGameId });
      const blob = new Blob([sgf], { type: 'text/plain' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
      a.download = `game-${goGameId.slice(0,8)}.sgf`; a.click();
    }
  }

  async function loadDailyPuzzle() {
    dailyPuzzle = await invoke('get_daily_puzzle');
    puzzleMsg = '';
    puzzleInput = '';
  }

  async function submitPuzzleGuess() {
    if (!dailyPuzzle || !puzzleInput.trim()) return;
    const result: any = await invoke('check_puzzle_move', {
      puzzleId: dailyPuzzle.id,
      uciMove: puzzleInput.trim(),
    });
    puzzleMsg = result.status === 'solved'
      ? `Solved! ${result.message}`
      : result.status === 'correct'
      ? `Correct! ${result.message}`
      : result.status === 'wrong'
      ? `Not quite. Hint: ${result.hint}`
      : 'Error checking move.';
    puzzleInput = '';
  }

  $: if (activeTab === 'puzzles' && !dailyPuzzle) loadDailyPuzzle();
</script>

<div class="game-panel">
  <!-- Tab bar -->
  <div class="tab-bar">
    <button class="tab" class:active={activeTab === 'play'} on:click={() => { activeTab = 'play'; activeGame = null; }}>Play</button>
    <button class="tab" class:active={activeTab === 'tournaments'} on:click={() => activeTab = 'tournaments'}>Tournaments</button>
    <button class="tab" class:active={activeTab === 'puzzles'} on:click={() => activeTab = 'puzzles'}>Puzzles</button>
  </div>

  <!-- Agent thinking strip -->
  {#if agentThinking}
    <div class="thinking-strip thinking">
      <span class="pulse">●</span>
      <span>BonsAI is thinking…</span>
    </div>
  {:else if agentBestMove}
    <div class="thinking-strip done">
      <span>Best: <strong>{agentBestMove}</strong></span>
      <span class="sims">{agentSimulations.toLocaleString()} sims</span>
      <span class="eval">{agentValuePct}%</span>
      {#each agentTopMoves as m}
        <span class="top-move">{m}</span>
      {/each}
    </div>
  {/if}

  <!-- Play tab -->
  {#if activeTab === 'play'}
    {#if activeGame === null}
      <div class="menu">
        <h2 class="menu-title">Play a Game</h2>

        <div class="game-card">
          <div class="card-icon">♟</div>
          <h3>Chess</h3>
          <p>Play against BonsAI with MCTS engine</p>
          <div class="settings">
            <label>Name<input bind:value={chessPlayerName} type="text" /></label>
            <label>Play as
              <select bind:value={chessColor}>
                <option value="white">White</option>
                <option value="black">Black</option>
              </select>
            </label>
            <label>Strength
              <select bind:value={chessStrength}>
                <option value="interactive">Normal (400 sims)</option>
                <option value="strong">Strong (3200 sims)</option>
              </select>
            </label>
          </div>
          <button class="play-btn" on:click={startChess}>Play Chess</button>
        </div>

        <div class="game-card">
          <div class="card-icon">⚫</div>
          <h3>Go</h3>
          <p>Play Go on up to 19×19 board</p>
          <div class="settings">
            <label>Name<input bind:value={goPlayerName} type="text" /></label>
            <label>Play as
              <select bind:value={goColor}>
                <option value="black">Black</option>
                <option value="white">White</option>
              </select>
            </label>
            <label>Board size
              <select bind:value={goBoardSize}>
                <option value={9}>9×9 (quick)</option>
                <option value={13}>13×13</option>
                <option value={19}>19×19 (full)</option>
              </select>
            </label>
            <label>Komi<input bind:value={goKomi} type="number" step="0.5" min="0" max="15" /></label>
          </div>
          <button class="play-btn" on:click={startGo}>Play Go</button>
        </div>
      </div>

    {:else if activeGame === 'chess'}
      <div class="board-wrapper">
        <div class="board-toolbar">
          <button class="back-btn" on:click={backToMenu}>← Menu</button>
          {#if chessGameId}
            <button class="export-btn" on:click={exportGame}>Export PGN</button>
          {/if}
        </div>
        <ChessBoard
          gameId={chessGameId}
          humanColor={chessColor}
          playerName={chessPlayerName}
          aiStrength={chessStrength}
          on:gameCreated={onChessCreated}
        />
      </div>

    {:else if activeGame === 'go'}
      <div class="board-wrapper">
        <div class="board-toolbar">
          <button class="back-btn" on:click={backToMenu}>← Menu</button>
          {#if goGameId}
            <button class="export-btn" on:click={exportGame}>Export SGF</button>
          {/if}
        </div>
        <GoBoard
          gameId={goGameId}
          humanColor={goColor}
          playerName={goPlayerName}
          boardSize={goBoardSize}
          komi={goKomi}
          on:gameCreated={onGoCreated}
        />
      </div>
    {/if}

  <!-- Tournaments tab -->
  {:else if activeTab === 'tournaments'}
    <TournamentPanel />

  <!-- Puzzles tab -->
  {:else if activeTab === 'puzzles'}
    <div class="puzzle-area">
      {#if !dailyPuzzle}
        <div class="puzzle-loading">Loading puzzle…</div>
      {:else}
        <div class="puzzle-card">
          <h3 class="puzzle-title">Daily Puzzle</h3>
          <div class="puzzle-meta">
            <span class="puzzle-game">{dailyPuzzle.game_type ?? 'chess'}</span>
            <span class="puzzle-difficulty">{dailyPuzzle.difficulty ?? 'medium'}</span>
          </div>
          <p class="puzzle-desc">{dailyPuzzle.description ?? 'Find the best move.'}</p>
          <div class="puzzle-position">
            <code>{dailyPuzzle.position ?? ''}</code>
          </div>
          <div class="puzzle-input-row">
            <input
              bind:value={puzzleInput}
              placeholder="Enter move (e.g. e2e4)"
              class="puzzle-input"
              on:keydown={e => e.key === 'Enter' && submitPuzzleGuess()}
            />
            <button class="puzzle-btn" on:click={submitPuzzleGuess} disabled={!puzzleInput.trim()}>Check</button>
          </div>
          {#if puzzleMsg}
            <div class="puzzle-msg" class:correct={puzzleMsg.startsWith('Solved') || puzzleMsg.startsWith('Correct')}
              class:wrong={puzzleMsg.startsWith('Not')}>
              {puzzleMsg}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .game-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0;
    height: 100%;
    overflow-y: auto;
    background: var(--surface-1, #181825);
  }

  /* Tab bar */
  .tab-bar {
    display: flex;
    width: 100%;
    border-bottom: 1px solid var(--surface-3, #313244);
    background: var(--surface-2, #1e1e2e);
    flex-shrink: 0;
  }
  .tab {
    flex: 1;
    padding: 10px 0;
    font-size: 13px;
    font-weight: 600;
    background: none;
    border: none;
    color: var(--subtext, #888);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab.active { color: var(--mauve, #cba6f7); border-bottom-color: var(--mauve, #cba6f7); }
  .tab:hover:not(.active) { color: var(--text, #cdd6f4); }

  /* Thinking strip */
  .thinking-strip {
    width: 100%;
    padding: 6px 14px;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }
  .thinking-strip.thinking { background: rgba(249,226,175,0.08); color: var(--yellow, #f9e2af); }
  .thinking-strip.done { background: rgba(166,227,161,0.07); color: var(--text, #cdd6f4); }
  .pulse { animation: blink 1s ease infinite; }
  @keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0.2; } }
  .sims, .eval { color: var(--subtext, #888); font-size: 11px; }
  .top-move {
    background: var(--surface-3, #313244);
    padding: 1px 5px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 11px;
    color: var(--mauve, #cba6f7);
  }

  /* Play tab layout */
  .menu {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    width: 100%;
    max-width: 560px;
    padding: 16px;
  }

  .menu-title {
    font-size: 22px;
    font-weight: 700;
    color: var(--text, #cdd6f4);
    margin: 0;
  }

  .game-card {
    background: var(--surface-2, #1e1e2e);
    border-radius: 12px;
    padding: 20px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border: 1px solid var(--surface-3, #313244);
  }

  .card-icon { font-size: 32px; text-align: center; }

  .game-card h3 {
    margin: 0;
    font-size: 18px;
    color: var(--text, #cdd6f4);
    text-align: center;
  }
  .game-card p {
    margin: 0;
    font-size: 13px;
    color: var(--subtext, #888);
    text-align: center;
  }

  .settings {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  label {
    display: flex;
    flex-direction: column;
    font-size: 12px;
    color: var(--subtext, #888);
    gap: 3px;
  }

  input, select {
    background: var(--surface-3, #313244);
    color: var(--text, #cdd6f4);
    border: 1px solid var(--surface-3, #45475a);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 13px;
    outline: none;
  }
  input:focus, select:focus { border-color: var(--mauve, #cba6f7); }

  .play-btn {
    padding: 8px 20px;
    border-radius: 8px;
    background: var(--mauve, #cba6f7);
    color: #1e1e2e;
    font-weight: 700;
    font-size: 14px;
    border: none;
    cursor: pointer;
    transition: background 0.15s;
    align-self: center;
    width: 100%;
  }
  .play-btn:hover { background: var(--lavender, #b4befe); }

  .board-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 12px;
  }

  .board-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .back-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 13px;
    background: var(--surface-2, #1e1e2e);
    color: var(--subtext, #888);
    border: 1px solid var(--surface-3, #313244);
    cursor: pointer;
    transition: color 0.15s;
  }
  .back-btn:hover { color: var(--text, #cdd6f4); }

  .export-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    background: none;
    color: var(--green, #a6e3a1);
    border: 1px solid var(--green, #a6e3a1);
    cursor: pointer;
    margin-left: auto;
  }
  .export-btn:hover { background: var(--green, #a6e3a1); color: #000; }

  /* Puzzle tab */
  .puzzle-area {
    width: 100%;
    max-width: 520px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .puzzle-loading { color: var(--subtext, #888); font-size: 13px; padding: 20px; text-align: center; }
  .puzzle-card {
    background: var(--surface-2, #1e1e2e);
    border-radius: 12px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border: 1px solid var(--surface-3, #313244);
  }
  .puzzle-title { margin: 0; font-size: 17px; color: var(--text, #cdd6f4); }
  .puzzle-meta { display: flex; gap: 8px; }
  .puzzle-game, .puzzle-difficulty {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 4px;
    background: var(--surface-3, #313244);
    color: var(--subtext, #888);
    text-transform: uppercase;
  }
  .puzzle-desc { margin: 0; font-size: 13px; color: var(--subtext, #aaa); }
  .puzzle-position {
    background: var(--surface-3, #313244);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 11px;
    font-family: monospace;
    color: var(--mauve, #cba6f7);
    word-break: break-all;
  }
  .puzzle-input-row { display: flex; gap: 8px; }
  .puzzle-input {
    flex: 1;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--surface-3, #313244);
    color: var(--text, #cdd6f4);
    border: 1px solid var(--surface-3, #45475a);
    font-size: 13px;
    outline: none;
  }
  .puzzle-input:focus { border-color: var(--mauve, #cba6f7); }
  .puzzle-btn {
    padding: 6px 16px;
    border-radius: 6px;
    background: var(--mauve, #cba6f7);
    color: #1e1e2e;
    font-weight: 700;
    font-size: 13px;
    border: none;
    cursor: pointer;
  }
  .puzzle-btn:disabled { opacity: 0.5; cursor: default; }
  .puzzle-msg {
    font-size: 13px;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--surface-3, #313244);
    color: var(--subtext, #888);
  }
  .puzzle-msg.correct { color: var(--green, #a6e3a1); }
  .puzzle-msg.wrong { color: var(--red, #f38ba8); }
</style>
