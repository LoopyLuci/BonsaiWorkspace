<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher, onMount } from 'svelte';

  export let gameId: string | null = null;
  export let humanColor: 'black' | 'white' = 'black';
  export let playerName: string = 'Player';
  export let boardSize: 9 | 13 | 19 = 19;
  export let komi: number = 7.5;

  const dispatch = createEventDispatcher();

  interface StoneView { x: number; y: number; color: string; }
  interface GoGameView {
    id: string;
    size: number;
    stones: StoneView[];
    current_player_id: string;
    current_color: string;
    result: string;
    black_name: string;
    white_name: string;
    black_captures: number;
    white_captures: number;
    komi: number;
    move_count: number;
    score_estimate: number;
  }

  let game: GoGameView | null = null;
  let hoverPoint: { x: number; y: number } | null = null;
  let status = '';
  let loading = false;
  let error = '';

  // Canvas sizing
  const CELL = 32;
  const MARGIN = CELL;
  $: CANVAS_SIZE = MARGIN * 2 + (boardSize - 1) * CELL;

  onMount(async () => {
    if (gameId) {
      await loadGame(gameId);
    } else {
      await startGame();
    }
  });

  async function startGame() {
    loading = true;
    error = '';
    try {
      game = await invoke<GoGameView>('create_go_game', {
        req: { human_name: playerName, human_color: humanColor, board_size: boardSize, komi }
      });
      gameId = game.id;
      dispatch('gameCreated', { id: game.id });
      updateStatus();
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function loadGame(id: string) {
    try {
      game = await invoke<GoGameView>('get_go_game', { gameId: id });
      updateStatus();
    } catch (e: any) {
      error = String(e);
    }
  }

  function updateStatus() {
    if (!game) return;
    if (game.result === '*') {
      const isMyTurn = game.current_player_id === 'user';
      status = isMyTurn ? `Your turn (${game.current_color})` : 'BonsAI is thinking…';
    } else {
      status = `Game over: ${game.result}`;
    }
  }

  function coordToPixel(v: number): number {
    return MARGIN + v * CELL;
  }

  function pixelToCoord(px: number): number {
    return Math.round((px - MARGIN) / CELL);
  }

  function isStarPoint(x: number, y: number, size: number): boolean {
    const stars19 = [3, 9, 15];
    const stars13 = [3, 6, 9];
    const stars9  = [2, 4, 6];
    const stars = size === 19 ? stars19 : size === 13 ? stars13 : stars9;
    return stars.includes(x) && stars.includes(y);
  }

  function handleCanvasMouseMove(e: MouseEvent) {
    if (!game || game.result !== '*' || game.current_player_id !== 'user') {
      hoverPoint = null;
      return;
    }
    const rect = (e.currentTarget as HTMLCanvasElement).getBoundingClientRect();
    const x = pixelToCoord(e.clientX - rect.left);
    const y = pixelToCoord(e.clientY - rect.top);
    if (x >= 0 && x < game.size && y >= 0 && y < game.size) {
      hoverPoint = { x, y };
    } else {
      hoverPoint = null;
    }
  }

  async function handleCanvasClick(e: MouseEvent) {
    if (!game || game.result !== '*' || game.current_player_id !== 'user') return;
    const rect = (e.currentTarget as HTMLCanvasElement).getBoundingClientRect();
    const x = pixelToCoord(e.clientX - rect.left);
    const y = pixelToCoord(e.clientY - rect.top);
    if (x < 0 || x >= game.size || y < 0 || y >= game.size) return;

    // Convert to GTP: x is column (A-T skipping I), y is row from bottom (1-indexed)
    const colChar = String.fromCharCode('A'.charCodeAt(0) + x + (x >= 8 ? 1 : 0));
    const row = game.size - y;
    const gtp = `${colChar}${row}`;
    await playMove(gtp);
  }

  async function playMove(gtp: string) {
    if (!game) return;
    loading = true;
    error = '';
    try {
      game = await invoke<GoGameView>('make_go_move', { gameId: game.id, gtpCoord: gtp });
      hoverPoint = null;
      updateStatus();
      dispatch('moveMade', { game });
    } catch (e: any) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function pass() {
    await playMove('pass');
  }

  async function resign() {
    if (!game) return;
    try {
      game = await invoke<GoGameView>('resign_go_game', { gameId: game.id });
      updateStatus();
    } catch (e: any) {
      error = String(e);
    }
  }

  // Build stone lookup
  $: stoneMap = new Map<string, string>(
    (game?.stones ?? []).map(s => [`${s.x},${s.y}`, s.color])
  );

  function drawBoard(canvas: HTMLCanvasElement) {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const size = game?.size ?? boardSize;

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Board background
    ctx.fillStyle = '#dcb468';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Grid lines
    ctx.strokeStyle = '#8b6914';
    ctx.lineWidth = 1;
    for (let i = 0; i < size; i++) {
      const px = coordToPixel(i);
      ctx.beginPath();
      ctx.moveTo(px, MARGIN);
      ctx.lineTo(px, MARGIN + (size - 1) * CELL);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(MARGIN, px);
      ctx.lineTo(MARGIN + (size - 1) * CELL, px);
      ctx.stroke();
    }

    // Star points
    for (let x = 0; x < size; x++) {
      for (let y = 0; y < size; y++) {
        if (isStarPoint(x, y, size)) {
          ctx.beginPath();
          ctx.arc(coordToPixel(x), coordToPixel(y), 4, 0, Math.PI * 2);
          ctx.fillStyle = '#8b6914';
          ctx.fill();
        }
      }
    }

    // Stones
    const stoneR = CELL * 0.46;
    for (const [key, color] of stoneMap) {
      const [sx, sy] = key.split(',').map(Number);
      const px = coordToPixel(sx);
      const py = coordToPixel(sy);
      ctx.beginPath();
      ctx.arc(px, py, stoneR, 0, Math.PI * 2);
      if (color === 'black') {
        const grad = ctx.createRadialGradient(px - 3, py - 3, 1, px, py, stoneR);
        grad.addColorStop(0, '#666');
        grad.addColorStop(1, '#000');
        ctx.fillStyle = grad;
      } else {
        const grad = ctx.createRadialGradient(px - 3, py - 3, 1, px, py, stoneR);
        grad.addColorStop(0, '#fff');
        grad.addColorStop(1, '#ccc');
        ctx.fillStyle = grad;
      }
      ctx.fill();
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 0.5;
      ctx.stroke();
    }

    // Hover ghost stone
    if (hoverPoint && !stoneMap.has(`${hoverPoint.x},${hoverPoint.y}`)) {
      const px = coordToPixel(hoverPoint.x);
      const py = coordToPixel(hoverPoint.y);
      ctx.beginPath();
      ctx.arc(px, py, stoneR, 0, Math.PI * 2);
      ctx.fillStyle = humanColor === 'black'
        ? 'rgba(0,0,0,0.35)'
        : 'rgba(255,255,255,0.55)';
      ctx.fill();
    }

    // Coordinate labels
    ctx.fillStyle = '#5c3d0a';
    ctx.font = '11px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (let i = 0; i < size; i++) {
      const col = String.fromCharCode('A'.charCodeAt(0) + i + (i >= 8 ? 1 : 0));
      ctx.fillText(col, coordToPixel(i), MARGIN / 2);
      ctx.fillText(col, coordToPixel(i), canvas.height - MARGIN / 2);
      ctx.fillText(String(size - i), MARGIN / 2, coordToPixel(i));
      ctx.fillText(String(size - i), canvas.width - MARGIN / 2, coordToPixel(i));
    }
  }

  // Reactive redraw
  let canvas: HTMLCanvasElement;
  $: if (canvas && game) drawBoard(canvas);
  $: if (canvas && hoverPoint !== undefined) drawBoard(canvas);
</script>

<div class="go-panel">
  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  {#if !game && loading}
    <div class="loading">Starting game…</div>
  {:else if game}
    <div class="game-header">
      <div class="player-info">
        <span class="stone-icon black-icon">●</span>
        <span class="player-name">{game.black_name}</span>
        <span class="captures">caps: {game.black_captures}</span>
      </div>
      <div class="score-estimate">
        Score est: {game.score_estimate > 0 ? `B+${game.score_estimate.toFixed(1)}` : `W+${(-game.score_estimate).toFixed(1)}`}
      </div>
      <div class="player-info">
        <span class="stone-icon white-icon">○</span>
        <span class="player-name">{game.white_name}</span>
        <span class="captures">caps: {game.white_captures}</span>
      </div>
    </div>

    <canvas
      bind:this={canvas}
      width={CANVAS_SIZE}
      height={CANVAS_SIZE}
      class="go-canvas"
      class:interactive={game.result === '*' && game.current_player_id === 'user'}
      on:mousemove={handleCanvasMouseMove}
      on:mouseleave={() => { hoverPoint = null; drawBoard(canvas); }}
      on:click={handleCanvasClick}
    />

    <div class="game-footer">
      <span class="status" class:thinking={loading}>{status}</span>
      <div class="actions">
        {#if game.result === '*'}
          <button class="action-btn" on:click={pass} disabled={loading}>Pass</button>
          <button class="resign-btn action-btn" on:click={resign} disabled={loading}>Resign</button>
        {:else}
          <button class="action-btn" on:click={startGame}>New Game</button>
        {/if}
      </div>
    </div>

    <div class="game-meta">
      Komi: {game.komi} · Moves: {game.move_count} · {game.size}×{game.size}
    </div>
  {/if}
</div>

<style>
  .go-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px;
    background: var(--surface-2, #1e1e2e);
    border-radius: 12px;
    user-select: none;
  }

  .error-banner {
    background: var(--red, #f38ba8);
    color: #000;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 13px;
  }

  .loading { color: var(--subtext, #888); font-size: 14px; }

  .game-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    font-size: 13px;
    gap: 8px;
  }

  .player-info { display: flex; align-items: center; gap: 4px; }
  .player-name { font-weight: 600; color: var(--text, #cdd6f4); }
  .captures { color: var(--subtext, #888); font-size: 11px; }
  .stone-icon { font-size: 18px; line-height: 1; }
  .black-icon { color: #111; text-shadow: 0 0 3px #fff; }
  .white-icon { color: #eee; text-shadow: 0 0 3px #000; }

  .score-estimate { font-size: 12px; color: var(--mauve, #cba6f7); font-style: italic; }

  .go-canvas {
    border-radius: 4px;
    border: 2px solid #8b6914;
    cursor: default;
  }
  .go-canvas.interactive { cursor: crosshair; }

  .game-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    font-size: 13px;
  }

  .status { color: var(--subtext, #a6adc8); }
  .status.thinking { color: var(--yellow, #f9e2af); }

  .actions { display: flex; gap: 6px; }

  .action-btn {
    padding: 4px 14px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid var(--surface-3, #45475a);
    background: var(--surface-3, #313244);
    color: var(--text, #cdd6f4);
    transition: background 0.15s;
  }
  .action-btn:hover { background: var(--green, #a6e3a1); color: #000; }
  .resign-btn:hover { background: var(--red, #f38ba8) !important; color: #000; }

  .game-meta { font-size: 11px; color: var(--subtext, #585b70); }
</style>
