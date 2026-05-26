<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher, onMount } from 'svelte';

  export let gameId: string | null = null;
  export let humanColor: 'white' | 'black' = 'white';
  export let playerName: string = 'Player';
  export let aiStrength: 'interactive' | 'strong' = 'interactive';

  const dispatch = createEventDispatcher();

  interface ChessGameView {
    id: string;
    fen: string;
    pgn: string;
    legal_moves: string[];
    current_player_id: string;
    result: string;
    white_name: string;
    black_name: string;
    move_count: number;
    opening_name: string | null;
  }

  let game: ChessGameView | null = null;
  let selectedSquare: string | null = null;
  let highlightedMoves: string[] = [];
  let status = '';
  let loading = false;
  let error = '';

  // Board geometry
  const FILES = ['a','b','c','d','e','f','g','h'];
  const RANKS = ['8','7','6','5','4','3','2','1'];

  // Piece unicode symbols
  const PIECE_CHARS: Record<string, string> = {
    K: '♔', Q: '♕', R: '♖', B: '♗', N: '♘', P: '♙',
    k: '♚', q: '♛', r: '♜', b: '♝', n: '♞', p: '♟',
  };

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
      game = await invoke<ChessGameView>('create_chess_game', {
        req: { human_name: playerName, human_color: humanColor, ai_strength: aiStrength }
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
      game = await invoke<ChessGameView>('get_chess_game', { gameId: id });
      updateStatus();
    } catch (e: any) {
      error = String(e);
    }
  }

  function updateStatus() {
    if (!game) return;
    if (game.result === '*') {
      const isMyTurn = game.current_player_id === 'user';
      status = isMyTurn ? 'Your turn' : 'BonsAI is thinking…';
    } else {
      status = `Game over: ${game.result}`;
    }
  }

  // Parse FEN into a board map: square -> piece char
  function parseFen(fen: string): Map<string, string> {
    const board = new Map<string, string>();
    const piecePart = fen.split(' ')[0];
    const rows = piecePart.split('/');
    for (let rank = 0; rank < 8; rank++) {
      let file = 0;
      for (const ch of rows[rank]) {
        if (ch >= '1' && ch <= '8') {
          file += parseInt(ch);
        } else {
          const sq = FILES[file] + RANKS[rank];
          board.set(sq, ch);
          file++;
        }
      }
    }
    return board;
  }

  function squareColor(file: number, rank: number): string {
    return (file + rank) % 2 === 0 ? 'light' : 'dark';
  }

  function handleSquareClick(sq: string) {
    if (!game || game.result !== '*' || game.current_player_id !== 'user') return;

    if (selectedSquare === null) {
      // First click: select if there's a friendly piece
      const board = parseFen(game.fen);
      const piece = board.get(sq);
      if (!piece) return;
      const isWhite = piece === piece.toUpperCase();
      if ((humanColor === 'white' && isWhite) || (humanColor === 'black' && !isWhite)) {
        selectedSquare = sq;
        highlightedMoves = game.legal_moves.filter(m => m.startsWith(sq));
      }
    } else {
      // Second click: attempt move
      const uci = selectedSquare + sq;
      // Check for pawn promotion (simplified: always promote to queen)
      const promotion = isPromotion(selectedSquare, sq) ? 'q' : '';
      selectedSquare = null;
      highlightedMoves = [];
      makeMove(uci + promotion);
    }
  }

  function isPromotion(from: string, to: string): boolean {
    if (!game) return false;
    const board = parseFen(game.fen);
    const piece = board.get(from);
    return (piece === 'P' && to[1] === '8') || (piece === 'p' && to[1] === '1');
  }

  async function makeMove(uci: string) {
    if (!game) return;
    loading = true;
    error = '';
    try {
      game = await invoke<ChessGameView>('make_chess_move', { gameId: game.id, notation: uci });
      updateStatus();
      dispatch('moveMade', { game });
    } catch (e: any) {
      error = `Illegal move: ${String(e)}`;
    } finally {
      loading = false;
    }
  }

  async function resign() {
    if (!game) return;
    try {
      game = await invoke<ChessGameView>('resign_chess_game', { gameId: game.id });
      updateStatus();
    } catch (e: any) {
      error = String(e);
    }
  }

  $: boardMap = game ? parseFen(game.fen) : new Map<string, string>();
  $: displayRanks = humanColor === 'white' ? RANKS : [...RANKS].reverse();
  $: displayFiles = humanColor === 'white' ? FILES : [...FILES].reverse();
</script>

<div class="chess-panel">
  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  {#if !game && loading}
    <div class="loading">Starting game…</div>
  {:else if game}
    <div class="game-header">
      <span class="player-label black-label">{game.black_name}</span>
      {#if game.opening_name}<span class="opening">{game.opening_name}</span>{/if}
      <span class="player-label white-label">{game.white_name}</span>
    </div>

    <div class="board" class:flipped={humanColor === 'black'}>
      {#each displayRanks as rank, ri}
        {#each displayFiles as file, fi}
          {@const sq = file + rank}
          {@const piece = boardMap.get(sq)}
          {@const fileIdx = FILES.indexOf(file)}
          {@const rankIdx = RANKS.indexOf(rank)}
          {@const isLight = squareColor(fileIdx, rankIdx) === 'light'}
          {@const isSelected = selectedSquare === sq}
          {@const isHighlighted = highlightedMoves.some(m => m.startsWith(selectedSquare ?? '') && m.slice(2, 4) === sq)}
          {@const isLastMove = game.move_count > 0}
          <div
            class="square"
            class:light={isLight}
            class:dark={!isLight}
            class:selected={isSelected}
            class:highlighted={isHighlighted}
            on:click={() => handleSquareClick(sq)}
            role="button"
            tabindex="0"
            on:keydown={e => e.key === 'Enter' && handleSquareClick(sq)}
          >
            {#if piece}
              <span class="piece" class:white-piece={piece === piece.toUpperCase()} class:black-piece={piece !== piece.toUpperCase()}>
                {PIECE_CHARS[piece] ?? piece}
              </span>
            {/if}
            {#if fi === 0}
              <span class="rank-label">{rank}</span>
            {/if}
            {#if ri === 7}
              <span class="file-label">{file}</span>
            {/if}
          </div>
        {/each}
      {/each}
    </div>

    <div class="game-footer">
      <span class="status" class:thinking={loading}>{status}</span>
      {#if game.result === '*'}
        <button class="resign-btn" on:click={resign} disabled={loading}>Resign</button>
      {:else}
        <button class="new-game-btn" on:click={startGame}>New Game</button>
      {/if}
    </div>

    {#if game.pgn}
      <details class="pgn-section">
        <summary>PGN</summary>
        <pre class="pgn-text">{game.pgn}</pre>
      </details>
    {/if}
  {/if}
</div>

<style>
  .chess-panel {
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

  .game-header, .game-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    max-width: 448px;
    font-size: 13px;
  }

  .player-label { font-weight: 600; color: var(--text, #cdd6f4); }
  .opening { font-size: 11px; color: var(--subtext, #888); font-style: italic; }
  .status { color: var(--subtext, #a6adc8); font-size: 13px; }
  .status.thinking { color: var(--yellow, #f9e2af); }

  .board {
    display: grid;
    grid-template-columns: repeat(8, 56px);
    grid-template-rows: repeat(8, 56px);
    border: 2px solid var(--surface-3, #313244);
    border-radius: 4px;
    overflow: hidden;
  }

  .square {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    width: 56px;
    height: 56px;
    transition: filter 0.1s;
  }
  .square:hover { filter: brightness(1.15); }
  .square.light  { background: #f0d9b5; }
  .square.dark   { background: #b58863; }
  .square.selected { background: #f6f669 !important; }
  .square.highlighted::after {
    content: '';
    position: absolute;
    width: 30%;
    height: 30%;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.25);
    pointer-events: none;
  }

  .piece { font-size: 36px; line-height: 1; }
  .white-piece { filter: drop-shadow(0 1px 2px rgba(0,0,0,0.6)); }
  .black-piece { filter: drop-shadow(0 1px 2px rgba(255,255,255,0.2)); }

  .rank-label {
    position: absolute;
    top: 2px; left: 3px;
    font-size: 10px;
    font-weight: 600;
    color: rgba(0,0,0,0.45);
    pointer-events: none;
  }
  .file-label {
    position: absolute;
    bottom: 2px; right: 3px;
    font-size: 10px;
    font-weight: 600;
    color: rgba(0,0,0,0.45);
    pointer-events: none;
  }
  .square.dark .rank-label,
  .square.dark .file-label { color: rgba(255,255,255,0.55); }

  .resign-btn, .new-game-btn {
    padding: 4px 14px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid var(--surface-3, #45475a);
    background: var(--surface-3, #313244);
    color: var(--text, #cdd6f4);
    transition: background 0.15s;
  }
  .resign-btn:hover { background: var(--red, #f38ba8); color: #000; }
  .new-game-btn:hover { background: var(--green, #a6e3a1); color: #000; }

  .pgn-section {
    width: 100%;
    max-width: 448px;
    font-size: 12px;
  }
  .pgn-section summary { cursor: pointer; color: var(--subtext, #888); }
  .pgn-text {
    background: var(--surface-1, #181825);
    padding: 8px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 11px;
    white-space: pre-wrap;
    color: var(--text, #cdd6f4);
    max-height: 150px;
    overflow-y: auto;
  }
</style>
