// OmniHarness AI panel — webview controller.
// Renders a Claude-Code-style chat + agent surface: sessions/history, rich
// markdown + diffs, checkpoints/undo, slash commands, keyboard shortcuts, a
// context/usage indicator, and a settings view for providers, local models,
// custom agents, MCP servers, server profiles, and live logs. Talks to the
// extension host via postMessage; the host proxies to the OmniHarness
// orchestrator.
(function () {
  const vscode = acquireVsCodeApi();
  const $ = (sel, root) => (root || document).querySelector(sel);
  const $all = (sel, root) => Array.from((root || document).querySelectorAll(sel));
  const el = (tag, cls, html) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; };
  const esc = (s) => String(s == null ? '' : s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

  // ── Rich-ish markdown renderer (no external libs; CSP forbids remote scripts) ──
  // Handles: fenced code blocks (+ copy button, language label), inline code,
  // bold/italic, links, headers, blockquotes, unordered/ordered lists, tables.
  function md(text) {
    const raw = String(text == null ? '' : text);
    const codeBlocks = [];
    // Pull out fenced code blocks first so their contents are never touched by
    // the line-oriented rules below.
    let s = raw.replace(/```([a-zA-Z0-9_+-]*)\n?([\s\S]*?)```/g, (_, lang, code) => {
      const idx = codeBlocks.length;
      codeBlocks.push({ lang: lang || '', code: code.replace(/\n$/, '') });
      return `@@CODEBLOCK${idx}@@`;
    });
    s = esc(s); // esc() leaves the @@CODEBLOCK@@ markers untouched (no &<>" in them)

    // Tables: header row, separator row, body rows (GFM-lite).
    s = s.replace(/^\|(.+)\|\s*\n\|[\s:|-]+\|\s*\n((?:\|.*\|\s*\n?)*)/gm, (block, headerRow, bodyRows) => {
      const cells = (row) => row.split('|').slice(1, -1).map((c) => c.trim());
      const head = cells(headerRow);
      const rows = bodyRows.trim().split('\n').filter(Boolean).map(cells);
      let out = '<table><thead><tr>' + head.map((h) => `<th>${h}</th>`).join('') + '</tr></thead><tbody>';
      for (const r of rows) { out += '<tr>' + r.map((c) => `<td>${c}</td>`).join('') + '</tr>'; }
      return out + '</tbody></table>\n';
    });

    const lines = s.split('\n');
    const out = [];
    let listType = null; // 'ul' | 'ol' | null
    const closeList = () => { if (listType) { out.push(`</${listType}>`); listType = null; } };
    for (let line of lines) {
      const h = line.match(/^(#{1,3})\s+(.*)$/);
      if (h) { closeList(); out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`); continue; }
      const bq = line.match(/^&gt;\s?(.*)$/);
      if (bq) { closeList(); out.push(`<blockquote>${inline(bq[1])}</blockquote>`); continue; }
      const ul = line.match(/^\s*[-*]\s+(.*)$/);
      if (ul) { if (listType !== 'ul') { closeList(); out.push('<ul>'); listType = 'ul'; } out.push(`<li>${inline(ul[1])}</li>`); continue; }
      const ol = line.match(/^\s*\d+\.\s+(.*)$/);
      if (ol) { if (listType !== 'ol') { closeList(); out.push('<ol>'); listType = 'ol'; } out.push(`<li>${inline(ol[1])}</li>`); continue; }
      closeList();
      out.push(line === '' ? '' : inline(line));
    }
    closeList();
    let html = out.join('\n');

    // Re-insert code blocks as real elements with copy buttons.
    html = html.replace(/@@CODEBLOCK(\d+)@@/g, (_, i) => {
      const { lang, code } = codeBlocks[Number(i)];
      const escCode = esc(code);
      return `<div class="codeblock">${lang ? `<span class="cb-lang">${esc(lang)}</span>` : ''}` +
        `<button class="cb-copy" data-copy-code="${Number(i)}">Copy</button><pre><code>${escCode}</code></pre></div>`;
    });
    // Stash raw code for the copy buttons via a side table (closures aren't
    // serializable across the innerHTML boundary, so we keep a lookup map).
    lastCodeBlocks = codeBlocks;
    return html;
  }
  let lastCodeBlocks = [];

  function inline(s) {
    let t = s.replace(/`([^`\n]+)`/g, (_, c) => `<code>${c}</code>`);
    t = t.replace(/\*\*([^*]+)\*\*/g, (_, b) => `<strong>${b}</strong>`);
    t = t.replace(/(?<![*_\w])[*_]([^*_\n]+)[*_](?![*_\w])/g, (_, i) => `<em>${i}</em>`);
    t = t.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label, url) => `<a data-href="${esc(url)}">${label}</a>`);
    return t;
  }

  // Delegate copy-button clicks (code blocks are re-created often; one listener covers all).
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('.cb-copy');
    if (btn) {
      const idx = Number(btn.getAttribute('data-copy-code'));
      const block = lastCodeBlocks[idx];
      if (block) { navigator.clipboard.writeText(block.code).then(() => { btn.textContent = 'Copied!'; setTimeout(() => { btn.textContent = 'Copy'; }, 1200); }); }
      return;
    }
    const link = e.target.closest('a[data-href]');
    if (link) { vscode.postMessage({ type: 'openExternal', url: link.getAttribute('data-href') }); }
  });

  // ── Line-level diff (simple LCS) ───────────────────────────────────────────
  function lcsDiff(beforeText, afterText) {
    const a = beforeText.split('\n');
    const b = afterText.split('\n');
    const n = a.length, m = b.length;
    // Cap cost for pathological huge files — fall back to a whole-block replace.
    if (n * m > 400000) {
      return [
        ...a.map((l) => ({ type: 'del', text: l })),
        ...b.map((l) => ({ type: 'add', text: l })),
      ];
    }
    const dp = Array.from({ length: n + 1 }, () => new Int32Array(m + 1));
    for (let i = n - 1; i >= 0; i--) {
      for (let j = m - 1; j >= 0; j--) {
        dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
      }
    }
    const ops = [];
    let i = 0, j = 0;
    while (i < n && j < m) {
      if (a[i] === b[j]) { ops.push({ type: 'ctx', text: a[i] }); i++; j++; }
      else if (dp[i + 1][j] >= dp[i][j + 1]) { ops.push({ type: 'del', text: a[i] }); i++; }
      else { ops.push({ type: 'add', text: b[j] }); j++; }
    }
    while (i < n) { ops.push({ type: 'del', text: a[i++] }); }
    while (j < m) { ops.push({ type: 'add', text: b[j++] }); }
    return ops;
  }

  function renderDiff(before, after, contextLines) {
    const ctx = contextLines == null ? 3 : contextLines;
    const ops = lcsDiff(before || '', after || '');
    let added = 0, removed = 0;
    for (const o of ops) { if (o.type === 'add') added++; else if (o.type === 'del') removed++; }
    // Collapse long unchanged runs to keep huge-file diffs readable.
    const rows = [];
    let run = [];
    const flushRun = (isEdge) => {
      if (!run.length) { return; }
      if (run.length <= ctx * 2 || isEdge) { rows.push(...run); }
      else {
        rows.push(...run.slice(0, ctx));
        rows.push({ type: 'skip', count: run.length - ctx * 2 });
        rows.push(...run.slice(run.length - ctx));
      }
      run = [];
    };
    for (const o of ops) {
      if (o.type === 'ctx') { run.push(o); }
      else { flushRun(false); rows.push(o); }
    }
    flushRun(true);

    let ln1 = 1, ln2 = 1;
    const lines = rows.map((r) => {
      if (r.type === 'skip') { return `<div class="dl ctx" style="opacity:.5">⋯ ${r.count} unchanged line(s) ⋯</div>`; }
      if (r.type === 'add') { const l = `<div class="dl add"><span class="dln">${ln2++}</span>+ ${esc(r.text)}</div>`; return l; }
      if (r.type === 'del') { const l = `<div class="dl del"><span class="dln">${ln1++}</span>- ${esc(r.text)}</div>`; return l; }
      ln1++; ln2++;
      return `<div class="dl ctx"><span class="dln">${ln2 - 1}</span>&nbsp; ${esc(r.text)}</div>`;
    });
    return {
      html: `<div class="diffstat"><span class="add">+${added}</span> <span class="del">-${removed}</span></div><div class="diffbox">${lines.join('')}</div>`,
      added, removed,
    };
  }

  // ── State ────────────────────────────────────────────────────────────────
  let state = {
    activeModel: '', activeAgent: 'coder', agents: [], providers: [], models: [],
    serverUrl: '', approvalMode: 'always', serverAlive: false,
    mcpServers: [], mcpStatus: [], mcpTools: [],
    favorites: [], serverProfiles: [], activeProfileId: 'default',
    sessions: [], activeSessionId: '',
  };
  let contextChips = [];       // {label, text}
  let curAssistant = null;     // {wrap, body, raw} for streaming
  let editingAgent = null;
  let editingMcp = null;
  let editingProfile = null;
  let lastUserText = '';
  let lastAssistantText = '';
  let contextWindow = 0;       // active model's context window, if known
  let liveTurns = [];          // mirrors session.turns for a live-updating usage bar

  const ALL_TOOLS = ['read_file', 'list_dir', 'search', 'write_file', 'edit_file', 'run_command', 'open_file', 'get_diagnostics', 'get_selection'];
  const SLASH_COMMANDS = [
    { name: '/model', desc: 'Switch model' },
    { name: '/agent', desc: 'Switch agent' },
    { name: '/clear', desc: 'Start a new chat' },
    { name: '/undo', desc: 'Undo the last file change' },
    { name: '/compact', desc: 'Compact conversation context now' },
    { name: '/copy', desc: 'Copy the last response' },
    { name: '/help', desc: 'List available commands' },
  ];

  // ── Shell ──────────────────────────────────────────────────────────────────
  const app = $('#app');
  app.innerHTML = `
    <div class="hdr">
      <button class="iconbtn" id="btnHistory" title="Chat history (Ctrl+H)">&#128337;</button>
      <button class="sel" id="modelBtn" title="Model (Ctrl+K)" style="text-align:left;max-width:38%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap"></button>
      <select class="sel" id="agentSel" title="Agent"></select>
      <button class="iconbtn" id="btnRefresh" title="Refresh model list">&#8635;</button>
      <span class="grow"></span>
      <span class="dot down" id="srvDot" title="Server status"></span>
      <button class="iconbtn" id="btnNew" title="New session (Ctrl+N)">&#128459;</button>
      <button class="iconbtn" id="btnSettings" title="Settings">&#9881;</button>
    </div>
    <div id="banner"></div>
    <div class="msgs" id="msgs"></div>
    <div class="usagebar" id="usagebar" style="display:none">
      <span id="ub-tokens">~0 tokens</span>
      <div class="ctxmeter"><div class="ctxfill" id="ub-fill" style="width:0%"></div></div>
      <span id="ub-msgs"></span>
    </div>
    <div class="status" id="status"></div>
    <div class="composer">
      <div class="chips" id="chips"></div>
      <div class="slashmenu" id="slashMenu"></div>
      <textarea id="input" placeholder="Ask OmniHarness to build, fix, explain… ( / for commands, Enter to send, Shift+Enter for newline)"></textarea>
      <div class="crow">
        <button class="iconbtn" id="btnCtx" title="Add current selection as context">&#128206;</button>
        <span class="grow"></span>
        <button class="btn sec sm" id="btnStop" style="display:none">Stop</button>
        <button class="btn" id="btnSend">Send</button>
      </div>
    </div>
    <div class="sessions-drawer" id="sessionsDrawer">
      <div class="sd-hdr">
        <h2>Chat History</h2>
        <button class="btn sm" id="sdNew">+ New Chat</button>
        <button class="btn sec sm" id="sdClose">Close</button>
      </div>
      <input class="field" id="sdSearch" placeholder="Search chats…">
      <div class="sessions-list" id="sessionsList"></div>
    </div>
    <div class="modelmenu" id="modelMenu" style="display:none;position:absolute;z-index:60;background:var(--vscode-dropdown-background);border:1px solid var(--vscode-dropdown-border,rgba(128,128,128,.3));border-radius:8px;box-shadow:0 8px 32px rgba(0,0,0,.4);max-height:340px;overflow-y:auto;min-width:260px">
      <div style="padding:6px"><input class="field" id="mmSearch" placeholder="Search models…"></div>
      <div id="mmList"></div>
    </div>
    <div class="settings" id="settings"></div>
    <div class="toast" id="toast"></div>
  `;

  const msgs = $('#msgs'), statusEl = $('#status'), input = $('#input');
  const agentSel = $('#agentSel'), modelBtn = $('#modelBtn'), srvDot = $('#srvDot');

  $('#btnSend').onclick = send;
  $('#btnStop').onclick = () => vscode.postMessage({ type: 'stop' });
  $('#btnNew').onclick = () => vscode.postMessage({ type: 'newSession' });
  $('#btnSettings').onclick = () => toggleSettings(true);
  $('#btnCtx').onclick = () => vscode.postMessage({ type: 'addSelection' });
  $('#btnRefresh').onclick = () => { vscode.postMessage({ type: 'refreshModels' }); toast('Refreshing models…'); };
  $('#btnHistory').onclick = () => toggleSessions(true);
  agentSel.onchange = () => vscode.postMessage({ type: 'setAgent', agent: agentSel.value });

  input.addEventListener('input', () => { updateSlashMenu(); });
  input.addEventListener('keydown', (e) => {
    if (slashMenuVisible()) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); moveSlashSelection(e.key === 'ArrowDown' ? 1 : -1); return; }
      if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); applySlashSelection(); return; }
      if (e.key === 'Escape') { closeSlashMenu(); return; }
    }
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); return; }
    if (e.key === 'ArrowUp' && input.value === '' && lastUserText) { e.preventDefault(); input.value = lastUserText; input.selectionStart = input.selectionEnd = input.value.length; return; }
    if (e.key === 'Escape') { closeSessionsAndMenus(); }
  });

  document.addEventListener('keydown', (e) => {
    const mod = e.ctrlKey || e.metaKey;
    if (!mod) { return; }
    if (e.key.toLowerCase() === 'n') { e.preventDefault(); vscode.postMessage({ type: 'newSession' }); }
    else if (e.key.toLowerCase() === 'k') { e.preventDefault(); openModelMenu(); }
    else if (e.key.toLowerCase() === 'h') { e.preventDefault(); toggleSessions(true); }
  });

  function closeSessionsAndMenus() {
    toggleSessions(false); closeSlashMenu(); closeModelMenu();
    if ($('#settings').classList.contains('show')) { toggleSettings(false); }
  }

  function showEmpty() {
    if (msgs.children.length === 0) {
      msgs.appendChild(el('div', 'empty',
        '&#129504; <strong>OmniHarness</strong><br>Pick an agent and model, then ask me to work on your project.<br><br>' +
        'I can read, edit, search, and run — using any local or API model.' +
        '<div class="hint">Type <kbd>/</kbd> for commands · <kbd>Ctrl+N</kbd> new chat · <kbd>Ctrl+H</kbd> history · <kbd>Ctrl+K</kbd> switch model</div>'));
    }
  }

  function send() {
    let text = input.value.trim();
    if (!text) return;
    for (const c of contextChips) { text += c.text; }
    contextChips = []; renderChips();
    input.value = '';
    closeSlashMenu();
    vscode.postMessage({ type: 'send', text });
  }

  function renderChips() {
    const box = $('#chips'); box.innerHTML = '';
    contextChips.forEach((c, i) => {
      const chip = el('span', 'chip', `&#128206; ${esc(c.label)} <button title="Remove">&times;</button>`);
      chip.querySelector('button').onclick = () => { contextChips.splice(i, 1); renderChips(); };
      box.appendChild(chip);
    });
  }

  // ── Slash commands ──────────────────────────────────────────────────────────
  let slashSel = 0;
  function slashMenuVisible() { return $('#slashMenu').classList.contains('show'); }
  function currentSlashMatches() {
    const v = input.value;
    if (!v.startsWith('/')) { return []; }
    const q = v.slice(1).split(' ')[0].toLowerCase();
    return SLASH_COMMANDS.filter((c) => c.name.slice(1).startsWith(q));
  }
  function updateSlashMenu() {
    const matches = currentSlashMatches();
    const menu = $('#slashMenu');
    if (!matches.length) { closeSlashMenu(); return; }
    slashSel = 0;
    menu.innerHTML = matches.map((c, i) => `<div class="slash-item${i === 0 ? ' sel' : ''}" data-cmd="${c.name}"><span class="sname">${c.name}</span><span class="sdesc">${esc(c.desc)}</span></div>`).join('');
    menu.classList.add('show');
    $all('.slash-item', menu).forEach((it) => { it.onclick = () => { input.value = it.getAttribute('data-cmd') + ' '; closeSlashMenu(); input.focus(); runSlashIfComplete(); }; });
  }
  function moveSlashSelection(dir) {
    const items = $all('.slash-item', $('#slashMenu'));
    if (!items.length) { return; }
    items[slashSel]?.classList.remove('sel');
    slashSel = (slashSel + dir + items.length) % items.length;
    items[slashSel].classList.add('sel');
  }
  function applySlashSelection() {
    const items = $all('.slash-item', $('#slashMenu'));
    const it = items[slashSel];
    if (it) { input.value = it.getAttribute('data-cmd') + ' '; }
    closeSlashMenu(); input.focus();
  }
  function closeSlashMenu() { $('#slashMenu').classList.remove('show'); }
  function runSlashIfComplete() {
    const v = input.value.trim();
    if (v === '/clear') { input.value = ''; vscode.postMessage({ type: 'newSession' }); }
    else if (v === '/undo') { input.value = ''; undoLastCheckpoint(); }
    else if (v === '/compact') { input.value = ''; vscode.postMessage({ type: 'compactNow' }); }
    else if (v === '/copy') { input.value = ''; if (lastAssistantText) { navigator.clipboard.writeText(lastAssistantText); toast('Copied last response.'); } else { toast('Nothing to copy yet.'); } }
    else if (v === '/help') { input.value = ''; toast(SLASH_COMMANDS.map((c) => c.name + ' — ' + c.desc).join('  ·  ')); }
    else if (v === '/model') { input.value = ''; openModelMenu(); }
    else if (v === '/agent') { input.value = ''; agentSel.focus(); }
  }
  function undoLastCheckpoint() {
    const cards = $all('.tool[data-checkpoint]:not(.undone)');
    const last = cards[cards.length - 1];
    if (!last) { toast('No undoable change in this session.'); return; }
    vscode.postMessage({ type: 'undoToolCall', checkpointId: last.getAttribute('data-checkpoint') });
  }

  // ── Rendering messages ──────────────────────────────────────────────────────
  function addUser(text) {
    const emptyEl = msgs.querySelector('.empty'); if (emptyEl) emptyEl.remove();
    const wrap = el('div', 'msg user');
    wrap.appendChild(el('div', 'who', 'You'));
    wrap.appendChild(el('div', 'body', md(text)));
    msgs.appendChild(wrap); scroll();
    lastUserText = text;
  }

  function startAssistant() {
    const wrap = el('div', 'msg assistant');
    wrap.appendChild(el('div', 'who', 'OmniHarness'));
    const body = el('div', 'body cursor', '');
    wrap.appendChild(body);
    msgs.appendChild(wrap); scroll();
    curAssistant = { wrap, body, raw: '' };
  }

  function assistantDelta(t) {
    if (!curAssistant) startAssistant();
    curAssistant.raw += t;
    curAssistant.body.innerHTML = md(curAssistant.raw);
    curAssistant.body.classList.add('cursor');
    scroll();
  }

  function addMsgActions(wrap, text) {
    const row = el('div', 'msg-actions');
    const copyBtn = el('button', null, 'Copy');
    copyBtn.onclick = () => { navigator.clipboard.writeText(text); copyBtn.textContent = 'Copied!'; setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1200); };
    row.appendChild(copyBtn);
    wrap.appendChild(row);
  }

  function assistantDone(full) {
    if (!curAssistant) return;
    // Strip the trailing tool/final JSON block from the visible bubble; the tool
    // card (or final text) represents it more clearly.
    let visible = (full || curAssistant.raw).replace(/```(?:tool|json)[\s\S]*?```\s*$/i, '').trim();
    if (!visible) { curAssistant.wrap.remove(); } else {
      curAssistant.body.innerHTML = md(visible);
      addMsgActions(curAssistant.wrap, visible);
      lastAssistantText = visible;
    }
    curAssistant.body.classList.remove('cursor');
    curAssistant = null;
  }

  function addToolCard(id, tool, args, diff) {
    const card = el('div', 'tool');
    card.id = 'tool-' + id;
    const head = el('div', 'thead',
      `<span class="tname">${esc(tool)}</span><span class="tsum">${esc(JSON.stringify(args).slice(0, 80))}</span><span class="tstat run" data-stat>&#9679; running</span>`);
    let bodyHtml;
    if (diff) { bodyHtml = renderDiff(diff.before, diff.after).html; }
    else { bodyHtml = `<pre>${esc(JSON.stringify(args, null, 2))}</pre>`; }
    const body = el('div', 'tbody', bodyHtml);
    head.onclick = () => card.classList.toggle('open');
    card.appendChild(head); card.appendChild(body);
    msgs.appendChild(card); scroll();
  }

  function updateToolCard(id, ok, summary, content, diff, checkpointId) {
    const card = $('#tool-' + id); if (!card) return;
    const stat = card.querySelector('[data-stat]');
    stat.className = 'tstat ' + (ok ? 'ok' : 'err');
    stat.innerHTML = ok ? '&#10003; done' : '&#10007; failed';
    card.querySelector('.tsum').textContent = summary || '';
    if (diff) {
      card.querySelector('.tbody').innerHTML = renderDiff(diff.before, diff.after).html;
    } else if (!card.querySelector('.diffbox')) {
      card.querySelector('.tbody').innerHTML = `<pre>${esc(content || summary || '')}</pre>`;
    }
    if (checkpointId) {
      card.setAttribute('data-checkpoint', checkpointId);
      const undoBtn = el('button', 'btn sec sm undo-btn', 'Undo');
      undoBtn.onclick = (e) => { e.stopPropagation(); vscode.postMessage({ type: 'undoToolCall', checkpointId }); };
      stat.appendChild(undoBtn);
    }
  }

  function markCardUndone(checkpointId) {
    const card = $all('.tool[data-checkpoint="' + checkpointId + '"]')[0];
    if (!card) return;
    card.classList.add('undone');
    const stat = card.querySelector('[data-stat]');
    if (stat) { stat.classList.add('undone'); const b = stat.querySelector('.undo-btn'); if (b) { b.remove(); } stat.innerHTML += ' (reverted)'; }
  }

  function addApproval(id, tool, args, preview, diff) {
    const card = el('div', 'approve');
    card.id = 'ap-' + id;
    card.appendChild(el('div', 'atitle', `Approve ${esc(tool)}?`));
    card.appendChild(el('div', 'apreview', esc(preview)));
    if (diff) {
      const d = renderDiff(diff.before, diff.after);
      card.appendChild(el('div', null, d.html));
    } else if (tool === 'run_command') {
      card.appendChild(el('pre', null, esc(JSON.stringify(args, null, 2))));
    }
    const row = el('div', 'arow');
    const yes = el('button', 'btn sm', 'Approve');
    const no = el('button', 'btn sec sm', 'Reject');
    yes.onclick = () => { vscode.postMessage({ type: 'approve', id, approved: true }); card.remove(); };
    no.onclick = () => { vscode.postMessage({ type: 'approve', id, approved: false }); card.remove(); };
    row.appendChild(yes); row.appendChild(no); card.appendChild(row);
    msgs.appendChild(card); scroll();
  }

  function addError(message, needsServer) {
    const card = el('div', 'msg assistant');
    card.appendChild(el('div', 'who', 'Error'));
    const body = el('div', 'body', esc(message));
    if (needsServer) {
      const b = el('button', 'btn sm', 'Start Server');
      b.style.marginTop = '8px';
      b.onclick = () => vscode.postMessage({ type: 'startServer' });
      body.appendChild(el('br')); body.appendChild(b);
    }
    card.appendChild(body); msgs.appendChild(card); scroll();
    setBusy(false);
  }

  function addCompactionDivider(text) {
    const wrap = el('div', 'compaction-divider');
    wrap.appendChild(el('span', 'compaction-line'));
    wrap.appendChild(el('span', 'compaction-label', esc(text)));
    wrap.appendChild(el('span', 'compaction-line'));
    msgs.appendChild(wrap); scroll();
  }

  function scroll() { msgs.scrollTop = msgs.scrollHeight; }
  function setBusy(b) { $('#btnStop').style.display = b ? '' : 'none'; $('#btnSend').style.display = b ? 'none' : ''; }
  function toast(text, err) { const t = $('#toast'); t.textContent = text; t.className = 'toast show' + (err ? ' err' : ''); setTimeout(() => t.className = 'toast', 3500); }

  // ── Re-render a persisted session's full transcript (on load/switch) ───────
  function renderSessionTurns(turns) {
    msgs.innerHTML = '';
    for (const t of turns || []) {
      if (t.role === 'user') { addUser(t.text || ''); }
      else if (t.role === 'assistant') {
        startAssistant();
        curAssistant.raw = t.text || '';
        assistantDone(t.text || '');
      } else if (t.role === 'tool') {
        addToolCard(t.id, t.toolName || '?', t.toolArgs || {}, t.diff || null);
        updateToolCard(t.id, !!t.toolOk, t.toolSummary || '', t.toolContent || '', t.diff || null, t.checkpointId);
      } else if (t.role === 'error') {
        addError(t.text || 'Error');
      } else if (t.role === 'compaction') {
        addCompactionDivider(t.text || 'Context compacted.');
      }
    }
    if (!turns || !turns.length) { showEmpty(); }
    liveTurns = (turns || []).slice();
    updateUsageBar(liveTurns);
    scroll();
  }

  // ── Usage / context indicator ───────────────────────────────────────────────
  function estimateTokens(turns) {
    let chars = 0;
    for (const t of turns) {
      chars += (t.text || '').length + (t.toolContent || '').length + (t.toolSummary || '').length;
    }
    return Math.round(chars / 4);
  }
  function updateUsageBar(turns) {
    const bar = $('#usagebar');
    if (!turns.length) { bar.style.display = 'none'; return; }
    const tokens = estimateTokens(turns);
    bar.style.display = 'flex';
    $('#ub-tokens').textContent = `~${tokens.toLocaleString()} tokens`;
    $('#ub-msgs').textContent = `${turns.length} turn(s)`;
    const fill = $('#ub-fill');
    if (contextWindow > 0) {
      const pct = Math.min(100, Math.round((tokens / contextWindow) * 100));
      fill.style.width = pct + '%';
      fill.className = 'ctxfill' + (pct > 90 ? ' danger' : pct > 70 ? ' warn' : '');
    } else {
      fill.style.width = '0%';
    }
  }

  // ── Header selects ──────────────────────────────────────────────────────────
  function renderAgents() {
    agentSel.innerHTML = '';
    state.agents.forEach((a) => {
      const o = el('option'); o.value = a.id; o.textContent = a.name; if (a.id === state.activeAgent) o.selected = true;
      agentSel.appendChild(o);
    });
  }

  function modelLabel(id) {
    const m = state.models.find((x) => (x.provider && !String(x.id).includes('/') ? `${x.provider}/${x.id}` : x.id) === id);
    return m ? m.id : id;
  }

  function renderModelButton() {
    modelBtn.textContent = (state.favorites.includes(state.activeModel) ? '★ ' : '') + (modelLabel(state.activeModel) || state.activeModel || 'Select model…');
    const m = state.models.find((x) => (x.provider && !String(x.id).includes('/') ? `${x.provider}/${x.id}` : x.id) === state.activeModel);
    contextWindow = (m && m.context_window) || 0;
  }

  function allModelIds() {
    const seen = new Set(); const list = [];
    const add = (id, provider, meta) => { if (seen.has(id)) return; seen.add(id); list.push({ id, provider, meta: meta || null }); };
    state.models.forEach((m) => { const id = m.provider && !String(m.id).includes('/') ? `${m.provider}/${m.id}` : m.id; add(id, m.provider || '', m); });
    if (state.activeModel) add(state.activeModel, (state.activeModel.split('/')[0] || ''), null);
    if (state.models.length === 0 && list.length === 0) add('anthropic/claude-sonnet-4-6', 'anthropic', null);
    return list;
  }
  // Local, no-key runtimes sort first — they're the zero-setup happy path.
  const LOCAL_PROVIDERS = new Set(['ollama', 'local']);
  function providerRank(p) { return LOCAL_PROVIDERS.has(p) ? 0 : 1; }
  function providerLabel(p) {
    return ({ ollama: 'Local · Ollama', local: 'Local · OpenAI-compatible', anthropic: 'Anthropic', openai: 'OpenAI',
      google: 'Google', gemini: 'Google', groq: 'Groq', mistral: 'Mistral', cohere: 'Cohere',
      openrouter: 'OpenRouter', together: 'Together', fireworks: 'Fireworks' })[p] || (p || 'Other');
  }
  function modelBadges(meta) {
    if (!meta) return '';
    const b = [];
    if (meta.context_window && meta.context_window > 0) {
      const k = meta.context_window >= 1000 ? Math.round(meta.context_window / 1000) + 'K' : String(meta.context_window);
      b.push(`<span class="mm-badge" title="Context window">${k}</span>`);
    }
    if (meta.supports_tools) b.push('<span class="mm-badge" title="Supports tools">🔧</span>');
    if (meta.supports_vision) b.push('<span class="mm-badge" title="Supports vision">👁</span>');
    return b.join('');
  }

  modelBtn.onclick = () => openModelMenu();
  function openModelMenu() {
    const menu = $('#modelMenu');
    const r = modelBtn.getBoundingClientRect();
    menu.style.left = r.left + 'px'; menu.style.top = (r.bottom + 4) + 'px';
    menu.style.display = 'block';
    $('#mmSearch').value = '';
    renderModelMenuList('');
    $('#mmSearch').focus();
  }
  function closeModelMenu() { $('#modelMenu').style.display = 'none'; }
  $('#mmSearch').addEventListener('input', () => renderModelMenuList($('#mmSearch').value.toLowerCase()));
  document.addEventListener('click', (e) => {
    const menu = $('#modelMenu');
    if (menu.style.display === 'block' && !menu.contains(e.target) && e.target !== modelBtn) { closeModelMenu(); }
  });
  function renderModelMenuList(filter) {
    const all = allModelIds().filter((m) => !filter || m.id.toLowerCase().includes(filter));
    const rowHtml = (m) => {
      const isFav = state.favorites.includes(m.id);
      return `<div class="sm-app-btn" style="flex-direction:row;justify-content:flex-start;gap:8px;padding:6px 10px" data-model="${esc(m.id)}">` +
        `<span class="modelpick-star${isFav ? ' fav' : ''}" data-star="${esc(m.id)}">${isFav ? '★' : '☆'}</span>` +
        `<span style="flex:1;font-size:12px">${esc(m.id)}</span>` +
        `<span class="mm-badges">${modelBadges(m.meta)}</span></div>`;
    };
    const list = $('#mmList');
    let html = '';

    // Favorites first (across all providers).
    const favs = all.filter((m) => state.favorites.includes(m.id));
    if (favs.length) {
      html += '<div class="sm-section-label" style="padding:4px 10px">★ Favorites</div>' + favs.map(rowHtml).join('');
    }

    // Then group the rest by provider, with local/no-key runtimes first.
    const rest = all.filter((m) => !state.favorites.includes(m.id));
    const groups = {};
    rest.forEach((m) => { (groups[m.provider] = groups[m.provider] || []).push(m); });
    const provs = Object.keys(groups).sort((a, b) => providerRank(a) - providerRank(b) || providerLabel(a).localeCompare(providerLabel(b)));
    provs.forEach((p) => {
      const isLocal = LOCAL_PROVIDERS.has(p);
      html += `<div class="sm-section-label" style="padding:4px 10px">${isLocal ? '💻 ' : ''}${esc(providerLabel(p))}${isLocal ? ' · no key needed' : ''}</div>`;
      html += groups[p].map(rowHtml).join('');
    });

    if (!all.length) {
      html = '<div class="mm-empty">' +
        '<div style="font-weight:600;margin-bottom:6px">No models available yet</div>' +
        '<div class="sub" style="margin-bottom:10px">Install a local runtime like <b>Ollama</b> (ollama.com) or <b>LM Studio</b> and it will be discovered automatically — no key required. Or add an API provider key in Settings.</div>' +
        '<button class="btn sm" data-mm-refresh style="margin-right:6px">🔄 Discover models</button>' +
        '<button class="btn sec sm" data-mm-settings>Open Settings</button></div>';
    }
    list.innerHTML = html;

    const refreshBtn = list.querySelector('[data-mm-refresh]');
    if (refreshBtn) refreshBtn.onclick = () => { vscode.postMessage({ type: 'refreshModels' }); toast('Scanning for local & API models...'); };
    const setBtn = list.querySelector('[data-mm-settings]');
    if (setBtn) setBtn.onclick = () => { closeModelMenu(); toggleSettings(true); };

    $all('[data-star]', list).forEach((star) => {
      star.onclick = (e) => { e.stopPropagation(); vscode.postMessage({ type: 'toggleFavoriteModel', id: star.getAttribute('data-star') }); };
    });
    $all('[data-model]', list).forEach((row) => {
      row.onclick = () => { const id = row.getAttribute('data-model'); vscode.postMessage({ type: 'setModel', model: id }); state.activeModel = id; renderModelButton(); closeModelMenu(); };
    });
  }

  function renderBanner() {
    const b = $('#banner'); b.innerHTML = '';
    if (!state.serverAlive) {
      const banner = el('div', 'banner', 'Starting the OmniHarness orchestrator automatically… If it does not come up, start it manually.');
      const btn = el('button', 'btn sm', 'Start Server');
      btn.onclick = () => vscode.postMessage({ type: 'startServer' });
      banner.appendChild(el('br')); banner.appendChild(btn);
      b.appendChild(banner);
    }
  }

  // ── Sessions drawer ──────────────────────────────────────────────────────────
  function toggleSessions(show) {
    const d = $('#sessionsDrawer');
    if (show) { vscode.postMessage({ type: 'listSessions' }); d.classList.add('show'); $('#sdSearch').value = ''; $('#sdSearch').focus(); }
    else { d.classList.remove('show'); }
  }
  $('#sdClose').onclick = () => toggleSessions(false);
  $('#sdNew').onclick = () => { vscode.postMessage({ type: 'newSession' }); toggleSessions(false); };
  $('#sdSearch').addEventListener('input', () => renderSessionsList());

  function renderSessionsList() {
    const box = $('#sessionsList'); box.innerHTML = '';
    const q = ($('#sdSearch').value || '').toLowerCase();
    const items = state.sessions.filter((s) => !q || s.title.toLowerCase().includes(q));
    if (!items.length) { box.appendChild(el('div', 'sub', 'No matching chats.')); return; }
    items.forEach((s) => {
      const item = el('div', 'session-item' + (s.id === state.activeSessionId ? ' active' : ''));
      const main = el('div', 'si-main');
      main.appendChild(el('div', 'si-title', esc(s.title)));
      main.appendChild(el('div', 'si-meta', `${s.messageCount} turn(s) · ${new Date(s.updatedAt).toLocaleString()}`));
      const actions = el('div', 'si-actions');
      const ren = el('button', null, '✏️'); ren.title = 'Rename';
      ren.onclick = (e) => { e.stopPropagation(); const t = prompt('Rename chat', s.title); if (t) { vscode.postMessage({ type: 'renameSession', id: s.id, title: t }); } };
      const del = el('button', null, '🗑️'); del.title = 'Delete';
      del.onclick = (e) => { e.stopPropagation(); if (confirm('Delete "' + s.title + '"?')) { vscode.postMessage({ type: 'deleteSession', id: s.id }); } };
      actions.appendChild(ren); actions.appendChild(del);
      item.appendChild(main); item.appendChild(actions);
      item.onclick = () => { vscode.postMessage({ type: 'switchSession', id: s.id }); toggleSessions(false); };
      box.appendChild(item);
    });
  }

  // ── Settings view ───────────────────────────────────────────────────────────
  function toggleSettings(show) {
    const s = $('#settings');
    if (show) { renderSettings(); s.classList.add('show'); } else { s.classList.remove('show'); }
  }

  function renderSettings() {
    const s = $('#settings');
    s.innerHTML = `
      <div class="hdr" style="padding:0 0 8px;border:none">
        <h2 style="flex:1">OmniHarness Settings</h2>
        <button class="btn sec sm" id="setClose">Close</button>
      </div>
      <input class="field" id="setSearch" placeholder="Search settings (providers, agents, MCP servers)…">

      <h3>Server Profiles</h3>
      <p class="sub">Multiple orchestrator endpoints — local dev, a remote box, Docker, etc. The active profile is what the panel talks to.</p>
      <div id="profilesList"></div>
      <button class="btn sm" id="newProfile">+ Add Profile</button>
      <div id="profileEditor"></div>
      <p class="sub" style="margin-top:8px">Status: <span id="setSrv">${state.serverAlive ? 'running' : 'stopped'}</span>
        <a class="link" id="setStart">Start</a> · <a class="link" id="setStop">Stop</a> · <a class="link" id="setOpen">VS Code settings</a> · <a class="link" id="setLogsToggle">Show logs</a></p>
      <div class="logbox" id="setLogs" style="display:none"></div>

      <h3 data-sec="providers">AI Providers &amp; API Keys</h3>
      <p class="sub" data-sec="providers">Keys are stored securely in VS Code. Click <em>Apply to server</em> to write them to OmniHarness/.env and restart.</p>
      <div id="provs" data-sec="providers"></div>
      <button class="btn sm" id="applyEnv" data-sec="providers">Apply keys to server (.env)</button>

      <h3 data-sec="local">Local Models</h3>
      <p class="sub" data-sec="local"><strong>Ollama:</strong> run <code>ollama serve</code> and <code>ollama pull &lt;model&gt;</code> → use as <code>ollama/&lt;name&gt;</code>.<br>
      <strong>llama.cpp / LM Studio / Jan:</strong> serve a GGUF on an OpenAI-compatible port, set <code>LOCAL_OPENAI_BASE_URL</code>, then use <code>local/&lt;model&gt;</code>.<br>
      e.g. <code>llama-server -m D:\\Models\\general\\gemma-4-31B-it-UD-Q2_K_XL\\gemma-4-31B-it-UD-Q2_K_XL.gguf --port 8081</code></p>

      <h3 data-sec="mcp">MCP Servers</h3>
      <p class="sub" data-sec="mcp">Model Context Protocol servers extend the agent with external tools. Enabled servers' tools appear to agents that allow all tools, namespaced <code>mcp__server__tool</code>.
        <a class="link" id="mcpRefresh">Refresh</a></p>
      <div id="mcpList" data-sec="mcp"></div>
      <button class="btn sm" id="newMcp" data-sec="mcp">+ Add MCP Server</button>
      <div id="mcpEditor" data-sec="mcp"></div>

      <h3 data-sec="agents">Custom Agents</h3>
      <p class="sub" data-sec="agents">Presets that control the system prompt, model, temperature, and which tools an agent may use.</p>
      <div id="agentsList" data-sec="agents"></div>
      <button class="btn sm" id="newAgent" data-sec="agents">+ New Agent</button>
      <div id="agentEditor" data-sec="agents"></div>

      <h3>Backup</h3>
      <p class="sub">Export/import your agents, MCP servers, and server profiles as a JSON file. API keys are never included.</p>
      <div class="row2">
        <button class="btn sec sm" id="exportCfg">Export Configuration…</button>
        <button class="btn sec sm" id="importCfg">Import Configuration…</button>
      </div>
      <div style="height:40px"></div>
    `;
    $('#setClose').onclick = () => toggleSettings(false);
    $('#setStart').onclick = () => vscode.postMessage({ type: 'startServer' });
    $('#setStop').onclick = () => vscode.postMessage({ type: 'stopServer' });
    $('#setOpen').onclick = () => vscode.postMessage({ type: 'openSettings' });
    $('#setLogsToggle').onclick = () => {
      const box = $('#setLogs');
      const show = box.style.display === 'none';
      box.style.display = show ? 'block' : 'none';
      if (show) { vscode.postMessage({ type: 'getLogs' }); }
    };
    $('#applyEnv').onclick = () => vscode.postMessage({ type: 'applyEnv' });
    $('#newAgent').onclick = () => { editingAgent = blankAgent(); renderAgentEditor(); };
    $('#mcpRefresh').onclick = () => vscode.postMessage({ type: 'syncMcp' });
    $('#newMcp').onclick = () => { editingMcp = blankMcp(); renderMcpEditor(); };
    $('#newProfile').onclick = () => { editingProfile = { id: 'profile-' + Date.now().toString(36), name: 'New Server', url: 'http://localhost:8080' }; renderProfileEditor(); };
    $('#exportCfg').onclick = () => vscode.postMessage({ type: 'exportConfig' });
    $('#importCfg').onclick = () => vscode.postMessage({ type: 'importConfig' });
    $('#setSearch').addEventListener('input', applySettingsFilter);

    renderProviders();
    renderMcpList();
    renderAgentsList();
    renderProfilesList();
    if (editingAgent) renderAgentEditor();
    if (editingMcp) renderMcpEditor();
    if (editingProfile) renderProfileEditor();
    vscode.postMessage({ type: 'getMcp' });
    vscode.postMessage({ type: 'getServerProfiles' });
  }

  function applySettingsFilter() {
    const q = ($('#setSearch').value || '').toLowerCase();
    const sections = {};
    $all('[data-sec]').forEach((elm) => { (sections[elm.getAttribute('data-sec')] = sections[elm.getAttribute('data-sec')] || []).push(elm); });
    if (!q) { $all('[data-sec]').forEach((elm) => elm.classList.remove('settings-hidden')); return; }
    for (const sec of Object.keys(sections)) {
      const cards = sections[sec].filter((e2) => e2.id && /List$|list$/.test(e2.id) === false && e2.children.length && e2.id !== ('' + sec));
      const matchAny = sections[sec].some((e2) => e2.textContent.toLowerCase().includes(q));
      sections[sec].forEach((e2) => e2.classList.toggle('settings-hidden', !matchAny));
    }
  }

  // ── Server profiles ──────────────────────────────────────────────────────────
  function renderProfilesList() {
    const box = $('#profilesList'); if (!box) return; box.innerHTML = '';
    (state.serverProfiles || []).forEach((p) => {
      const row = el('div', 'profile-row' + (p.id === state.activeProfileId ? ' active' : ''));
      const label = el('div', null, `<strong>${esc(p.name)}</strong> <span class="sub" style="margin:0">${esc(p.url)}</span>`);
      label.style.flex = '1';
      const use = el('button', 'btn sec sm', p.id === state.activeProfileId ? 'Active' : 'Use');
      use.disabled = p.id === state.activeProfileId;
      use.onclick = () => vscode.postMessage({ type: 'switchServerProfile', id: p.id });
      const edit = el('button', 'btn sec sm', 'Edit');
      edit.onclick = () => { editingProfile = { ...p }; renderProfileEditor(); };
      const del = el('button', 'btn sec sm', 'Delete');
      del.style.display = state.serverProfiles.length > 1 ? '' : 'none';
      del.onclick = () => vscode.postMessage({ type: 'deleteServerProfile', id: p.id });
      row.appendChild(label); row.appendChild(use); row.appendChild(edit); row.appendChild(del);
      box.appendChild(row);
    });
  }
  function renderProfileEditor() {
    const box = $('#profileEditor'); if (!box) return;
    if (!editingProfile) { box.innerHTML = ''; return; }
    const p = editingProfile;
    box.innerHTML = `
      <div class="agent-item" style="margin-top:10px">
        <label class="lbl">Name</label><input class="field" id="pfName" value="${esc(p.name)}">
        <label class="lbl">URL</label><input class="field" id="pfUrl" value="${esc(p.url)}" placeholder="http://localhost:8080">
        <div class="arow" style="margin-top:10px;display:flex;gap:6px">
          <button class="btn sm" id="pfSave">Save Profile</button>
          <button class="btn sec sm" id="pfCancel">Cancel</button>
        </div>
      </div>`;
    $('#pfCancel').onclick = () => { editingProfile = null; box.innerHTML = ''; };
    $('#pfSave').onclick = () => {
      const profile = { id: p.id, name: $('#pfName').value || 'Server', url: $('#pfUrl').value || 'http://localhost:8080' };
      vscode.postMessage({ type: 'saveServerProfile', profile });
      editingProfile = null; box.innerHTML = '';
    };
  }

  function statusFor(id) { return (state.mcpStatus || []).find((s) => s.id === id) || {}; }

  function renderMcpList() {
    const box = $('#mcpList'); if (!box) return; box.innerHTML = '';
    (state.mcpServers || []).forEach((srv) => {
      const st = statusFor(srv.id);
      const dotCls = !srv.enabled ? 'off' : (st.connected ? 'on' : 'off');
      const stTxt = !srv.enabled ? 'disabled' : (st.connected ? `${st.toolCount} tool(s)` : (st.error ? 'error' : 'connecting…'));
      const item = el('div', 'agent-item');
      const row = el('div', 'arow',
        `<span class="aname">${esc(srv.name)}</span><span class="badge ${dotCls}">${esc(stTxt)}</span>`);
      const toggle = el('button', 'btn sec sm', srv.enabled ? 'Disable' : 'Enable');
      toggle.onclick = () => vscode.postMessage({ type: 'toggleMcp', id: srv.id, enabled: !srv.enabled });
      const edit = el('button', 'btn sec sm', 'Edit');
      edit.onclick = () => { editingMcp = JSON.parse(JSON.stringify(srv)); renderMcpEditor(); };
      const del = el('button', 'btn sec sm', 'Delete');
      del.onclick = () => vscode.postMessage({ type: 'deleteMcpServer', id: srv.id });
      row.appendChild(toggle); row.appendChild(edit); row.appendChild(del);
      item.appendChild(row);
      const detail = srv.transport === 'http' ? esc(srv.url || '') : esc(`${srv.command || ''} ${(srv.args || []).join(' ')}`);
      item.appendChild(el('div', 'sub', `${srv.transport} · ${detail}`));
      if (st.error && srv.enabled) item.appendChild(el('div', 'sub', `⚠ ${esc(st.error)}`));
      box.appendChild(item);
    });
    if (!state.mcpServers || state.mcpServers.length === 0) {
      box.appendChild(el('div', 'sub', 'No MCP servers configured.'));
    }
    if ((state.mcpTools || []).length > 0) {
      const tools = state.mcpTools.map((t) => `<code title="${esc(t.description)}">${esc(t.name)}</code>`).join(' ');
      box.appendChild(el('div', 'mcp-tools', `${state.mcpTools.length} tool(s) available to agents: ${tools}`));
    }
  }

  function blankMcp() {
    return { id: 'mcp-' + Date.now().toString(36), name: 'New Server', transport: 'stdio', enabled: false, command: 'npx', args: [], url: '' };
  }

  function renderMcpEditor() {
    const box = $('#mcpEditor'); if (!box) return;
    if (!editingMcp) { box.innerHTML = ''; return; }
    const m = editingMcp;
    box.innerHTML = `
      <div class="agent-item" style="margin-top:10px">
        <label class="lbl">Name</label><input class="field" id="mcName" value="${esc(m.name)}">
        <label class="lbl">Transport</label>
        <select class="field" id="mcTransport">
          <option value="stdio" ${m.transport === 'stdio' ? 'selected' : ''}>stdio (spawn a command)</option>
          <option value="http" ${m.transport === 'http' ? 'selected' : ''}>http (Streamable HTTP URL)</option>
        </select>
        <div id="mcStdio" ${m.transport === 'http' ? 'style="display:none"' : ''}>
          <label class="lbl">Command</label><input class="field" id="mcCmd" value="${esc(m.command || '')}" placeholder="npx">
          <label class="lbl">Arguments (one per line)</label><textarea class="field" id="mcArgs" rows="3">${esc((m.args || []).join('\n'))}</textarea>
          <label class="lbl">Env (KEY=value per line)</label><textarea class="field" id="mcEnv" rows="2">${esc(Object.entries(m.env || {}).map(([k, v]) => k + '=' + v).join('\n'))}</textarea>
        </div>
        <div id="mcHttp" ${m.transport !== 'http' ? 'style="display:none"' : ''}>
          <label class="lbl">URL</label><input class="field" id="mcUrl" value="${esc(m.url || '')}" placeholder="http://localhost:3000/mcp">
          <label class="lbl">Headers (KEY=value per line)</label><textarea class="field" id="mcHeaders" rows="2">${esc(Object.entries(m.headers || {}).map(([k, v]) => k + '=' + v).join('\n'))}</textarea>
        </div>
        <label class="lbl"><input type="checkbox" id="mcEnabled" ${m.enabled ? 'checked' : ''}> Enabled</label>
        <div class="arow" style="margin-top:10px;display:flex;gap:6px">
          <button class="btn sm" id="mcSave">Save Server</button>
          <button class="btn sec sm" id="mcCancel">Cancel</button>
        </div>
      </div>`;
    $('#mcTransport').onchange = () => {
      const t = $('#mcTransport').value;
      $('#mcStdio').style.display = t === 'http' ? 'none' : '';
      $('#mcHttp').style.display = t === 'http' ? '' : 'none';
    };
    $('#mcCancel').onclick = () => { editingMcp = null; box.innerHTML = ''; };
    $('#mcSave').onclick = () => {
      const parseKv = (s) => { const o = {}; s.split('\n').map((l) => l.trim()).filter(Boolean).forEach((l) => { const i = l.indexOf('='); if (i > 0) o[l.slice(0, i)] = l.slice(i + 1); }); return o; };
      const t = $('#mcTransport').value;
      const server = {
        id: m.id, name: $('#mcName').value || 'Server', transport: t, enabled: $('#mcEnabled').checked,
        command: $('#mcCmd') ? $('#mcCmd').value : '',
        args: $('#mcArgs') ? $('#mcArgs').value.split('\n').map((s) => s.trim()).filter(Boolean) : [],
        env: $('#mcEnv') ? parseKv($('#mcEnv').value) : {},
        url: $('#mcUrl') ? $('#mcUrl').value : '',
        headers: $('#mcHeaders') ? parseKv($('#mcHeaders').value) : {},
      };
      vscode.postMessage({ type: 'saveMcpServer', server });
      editingMcp = null; box.innerHTML = '';
    };
  }

  function renderProviders() {
    const box = $('#provs'); if (!box) return; box.innerHTML = '';
    state.providers.forEach((p) => {
      const card = el('div', 'prov');
      if (p.kind === 'local') {
        card.innerHTML = `<div class="prow"><span class="pname">${esc(p.label)}</span><span class="badge on">enabled</span></div>
          <div class="local">Base URL: <code>${esc(p.defaultBaseUrl || '')}</code> · example: <code>${esc(p.example || '')}</code></div>`;
      } else {
        card.innerHTML = `<div class="prow"><span class="pname">${esc(p.label)}</span>
          <span class="badge ${p.configured ? 'on' : 'off'}">${p.configured ? 'key set' : 'no key'}</span></div>`;
        const row = el('div', 'row2');
        const inp = el('input', 'field'); inp.type = 'password'; inp.placeholder = p.configured ? '•••••••• (saved)' : `Enter ${p.envVar}`;
        const save = el('button', 'btn sm', 'Save');
        save.onclick = () => { vscode.postMessage({ type: 'saveKey', provider: p.id, key: inp.value }); inp.value = ''; };
        row.appendChild(inp); row.appendChild(save);
        card.appendChild(row);
        if (p.keyUrl) {
          const link = el('a', 'link', 'Get an API key →');
          link.onclick = () => vscode.postMessage({ type: 'openExternal', url: p.keyUrl });
          card.appendChild(link);
        }
      }
      box.appendChild(card);
    });
  }

  function renderAgentsList() {
    const box = $('#agentsList'); if (!box) return; box.innerHTML = '';
    state.agents.forEach((a) => {
      const item = el('div', 'agent-item');
      const row = el('div', 'arow',
        `<span class="aname">${esc(a.name)}</span><span class="badge ${a.autoApprove ? 'on' : 'off'}">${a.autoApprove ? 'auto' : 'ask'}</span>`);
      const edit = el('button', 'btn sec sm', 'Edit');
      edit.onclick = () => { editingAgent = JSON.parse(JSON.stringify(a)); renderAgentEditor(); };
      row.appendChild(edit);
      if (!a.builtin) {
        const del = el('button', 'btn sec sm', 'Delete');
        del.onclick = () => vscode.postMessage({ type: 'deleteAgent', id: a.id });
        row.appendChild(del);
      }
      item.appendChild(row);
      item.appendChild(el('div', 'sub', esc(a.description)));
      box.appendChild(item);
    });
  }

  function blankAgent() {
    return { id: 'agent-' + Date.now().toString(36), name: 'New Agent', description: '', systemPrompt: 'You are a helpful coding assistant.', model: state.activeModel, temperature: 0.2, maxTokens: 8192, tools: ['*'], autoApprove: false, builtin: false };
  }

  function renderAgentEditor() {
    const box = $('#agentEditor'); if (!box) return;
    if (!editingAgent) { box.innerHTML = ''; return; }
    const a = editingAgent;
    const allTools = a.tools.length === 1 && a.tools[0] === '*';
    box.innerHTML = `
      <div class="agent-item" style="margin-top:10px">
        <label class="lbl">Name</label><input class="field" id="agName" value="${esc(a.name)}">
        <label class="lbl">Description</label><input class="field" id="agDesc" value="${esc(a.description)}">
        <label class="lbl">Model (blank = use panel's active model)</label><input class="field" id="agModel" value="${esc(a.model)}" placeholder="anthropic/claude-sonnet-4-6">
        <label class="lbl">System prompt</label><textarea class="field" id="agPrompt" rows="4">${esc(a.systemPrompt)}</textarea>
        <div class="row2">
          <div><label class="lbl">Temperature</label><input class="field" id="agTemp" type="number" step="0.1" min="0" max="2" value="${a.temperature}"></div>
          <div><label class="lbl">Max tokens</label><input class="field" id="agMax" type="number" step="256" value="${a.maxTokens}"></div>
        </div>
        <label class="lbl"><input type="checkbox" id="agAll" ${allTools ? 'checked' : ''}> Allow all tools</label>
        <div class="toolgrid" id="agToolGrid"></div>
        <label class="lbl"><input type="checkbox" id="agAuto" ${a.autoApprove ? 'checked' : ''}> Auto-approve tool actions (no prompts)</label>
        <div class="arow" style="margin-top:10px;display:flex;gap:6px">
          <button class="btn sm" id="agSave">Save Agent</button>
          <button class="btn sec sm" id="agCancel">Cancel</button>
        </div>
      </div>`;
    const grid = $('#agToolGrid');
    const selected = new Set(allTools ? ALL_TOOLS : a.tools);
    ALL_TOOLS.forEach((t) => {
      const lab = el('label', null, `<input type="checkbox" data-tool="${t}" ${selected.has(t) ? 'checked' : ''}> ${t}`);
      grid.appendChild(lab);
    });
    const syncGridDisabled = () => { const all = $('#agAll').checked; grid.querySelectorAll('input').forEach((i) => i.disabled = all); };
    $('#agAll').onchange = syncGridDisabled; syncGridDisabled();
    $('#agCancel').onclick = () => { editingAgent = null; box.innerHTML = ''; };
    $('#agSave').onclick = () => {
      const allSel = $('#agAll').checked;
      const tools = allSel ? ['*'] : Array.from(grid.querySelectorAll('input:checked')).map((i) => i.getAttribute('data-tool'));
      const agent = {
        id: a.id, name: $('#agName').value || 'Agent', description: $('#agDesc').value,
        systemPrompt: $('#agPrompt').value, model: $('#agModel').value,
        temperature: parseFloat($('#agTemp').value) || 0.2, maxTokens: parseInt($('#agMax').value) || 8192,
        tools, autoApprove: $('#agAuto').checked, builtin: false,
      };
      vscode.postMessage({ type: 'saveAgent', agent });
      editingAgent = null; box.innerHTML = '';
    };
  }

  // ── Extension → webview ─────────────────────────────────────────────────────
  window.addEventListener('message', (ev) => {
    const m = ev.data;
    switch (m.type) {
      case 'state':
        state.activeModel = m.activeModel; state.activeAgent = m.activeAgent;
        state.agents = m.agents || []; state.providers = m.providers || [];
        state.approvalMode = m.approvalMode; state.serverUrl = m.serverUrl;
        renderAgents(); renderModelButton();
        if ($('#settings').classList.contains('show')) renderSettings();
        break;
      case 'models':
        state.models = m.models || []; renderModelButton();
        break;
      case 'favorites':
        state.favorites = m.favorites || []; renderModelButton();
        break;
      case 'serverProfiles':
        state.serverProfiles = m.profiles || []; state.activeProfileId = m.activeId;
        if ($('#settings').classList.contains('show')) renderProfilesList();
        break;
      case 'sessionsList':
        state.sessions = m.sessions || []; state.activeSessionId = m.activeId;
        if ($('#sessionsDrawer').classList.contains('show')) renderSessionsList();
        break;
      case 'sessionLoaded':
        state.activeSessionId = m.session.id;
        renderSessionTurns(m.session.turns);
        setBusy(false); statusEl.textContent = '';
        break;
      case 'logs':
        { const box = $('#setLogs'); if (box) { box.textContent = (m.lines || []).join('\n'); box.scrollTop = box.scrollHeight; } }
        break;
      case 'serverStatus':
        state.serverAlive = !!m.alive; srvDot.className = 'dot ' + (m.alive ? 'up' : 'down');
        srvDot.title = m.alive ? 'Orchestrator online' : 'Orchestrator offline';
        renderBanner();
        if ($('#settings').classList.contains('show')) { const e = $('#setSrv'); if (e) e.textContent = m.alive ? 'running' : 'stopped'; }
        break;
      case 'userMessage':
        addUser(m.text); setBusy(true); statusEl.textContent = 'Thinking…';
        liveTurns.push({ role: 'user', text: m.text }); updateUsageBar(liveTurns);
        break;
      case 'assistantStart': startAssistant(); break;
      case 'assistantDelta': assistantDelta(m.text); break;
      case 'assistantDone':
        assistantDone(m.text);
        liveTurns.push({ role: 'assistant', text: m.text }); updateUsageBar(liveTurns);
        break;
      case 'toolCall': addToolCard(m.id, m.tool, m.args); break;
      case 'toolResult':
        updateToolCard(m.id, m.ok, m.summary, m.content, m.diff, m.checkpointId);
        liveTurns.push({ role: 'tool', toolContent: m.content, toolSummary: m.summary }); updateUsageBar(liveTurns);
        break;
      case 'undoDone': markCardUndone(m.checkpointId); break;
      case 'compaction':
        addCompactionDivider(`Context compacted — ${m.messagesCompacted} earlier message(s) summarized (~${m.tokensSaved} tokens saved).`);
        liveTurns.push({ role: 'compaction', text: 'compacted' }); updateUsageBar(liveTurns);
        break;
      case 'approvalRequest': addApproval(m.id, m.tool, m.args, m.preview, m.diff); break;
      case 'status': statusEl.textContent = m.text || ''; break;
      case 'final':
        assistantDone(m.text); setBusy(false); statusEl.textContent = '';
        vscode.postMessage({ type: 'listSessions' });
        break;
      case 'error':
        addError(m.message, m.needsServer); statusEl.textContent = '';
        liveTurns.push({ role: 'error', text: m.message }); updateUsageBar(liveTurns);
        break;
      case 'sessionCleared': msgs.innerHTML = ''; showEmpty(); liveTurns = []; updateUsageBar(liveTurns); break;
      case 'toast': toast(m.text, m.error); if ($('#settings').classList.contains('show')) renderSettings(); break;
      case 'insertContext': contextChips.push({ label: m.label, text: m.text }); renderChips(); break;
      case 'mcpState':
        state.mcpServers = m.servers || []; state.mcpStatus = m.status || []; state.mcpTools = m.tools || [];
        if ($('#settings').classList.contains('show')) renderMcpList();
        break;
    }
  });

  showEmpty();
  vscode.postMessage({ type: 'ready' });
})();
