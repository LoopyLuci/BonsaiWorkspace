<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  // ── Types ────────────────────────────────────────────────────────────────────

  interface ExtractedRule {
    condition: string;
    action: string;
    confidence: number;
  }

  interface SecurityReport {
    passed: boolean;
    concerns: string[];
    content_hash: string;
  }

  interface CompiledSkill {
    id: string;
    name: string;
    description: string;
    tags: string[];
    wasm_hash: string;
    security_report: SecurityReport;
    requires_permissions: string[];
    rules: ExtractedRule[];
  }

  interface MarketplaceAsset {
    id: string;
    name: string;
    description: string;
    author: string;
    version: string;
    tags: string[];
    cid: string;
  }

  // ── State ────────────────────────────────────────────────────────────────────

  let installedSkills: CompiledSkill[] = [];
  let peerSkills: MarketplaceAsset[] = [];
  let loadingInstalled = false;
  let loadingPeers = false;

  let activeTab: 'installed' | 'marketplace' | 'import' = 'installed';

  // Marketplace search
  let searchQuery = '';
  let searchResults: MarketplaceAsset[] = [];
  let searching = false;

  // Import from content
  let importContent = '';
  let importAllowUnsafe = false;
  let importing = false;
  let importResult: CompiledSkill | null = null;
  let importError = '';

  // Per-skill actions
  let busySkills = new Set<string>();
  let verifyResults: Record<string, boolean> = {};
  let distillJobs: Record<string, string> = {};
  let invokeResults: Record<string, string> = {};
  let expandedSkill: string | null = null;

  // Toast
  let toast = '';
  let toastTimeout: ReturnType<typeof setTimeout>;

  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => (toast = ''), 3500);
  }

  // ── Data loading ─────────────────────────────────────────────────────────────

  async function loadInstalled() {
    loadingInstalled = true;
    try {
      installedSkills = await invoke<CompiledSkill[]>('list_compiled_skills');
    } catch (e) {
      showToast(`Load error: ${e}`);
    } finally {
      loadingInstalled = false;
    }
  }

  async function loadPeerSkills() {
    loadingPeers = true;
    try {
      peerSkills = await invoke<MarketplaceAsset[]>('discover_peer_skills');
    } catch (e) {
      showToast(`Peer discovery error: ${e}`);
    } finally {
      loadingPeers = false;
    }
  }

  onMount(loadInstalled);

  // ── Skill actions ─────────────────────────────────────────────────────────────

  async function verifySkill(id: string) {
    busySkills = new Set([...busySkills, id]);
    try {
      const ok = await invoke<boolean>('verify_compiled_skill', { id });
      verifyResults = { ...verifyResults, [id]: ok };
      showToast(ok ? `✅ ${id} integrity verified` : `⚠ ${id} integrity FAILED`);
    } catch (e) {
      showToast(`Verify error: ${e}`);
    } finally {
      busySkills.delete(id);
      busySkills = new Set(busySkills);
    }
  }

  async function uninstallSkill(id: string) {
    if (!confirm(`Uninstall "${id}"?`)) return;
    busySkills = new Set([...busySkills, id]);
    try {
      await invoke('uninstall_compiled_skill', { id });
      showToast(`Uninstalled: ${id}`);
      await loadInstalled();
    } catch (e) {
      showToast(`Uninstall error: ${e}`);
    } finally {
      busySkills.delete(id);
      busySkills = new Set(busySkills);
    }
  }

  async function distillSkill(skill: CompiledSkill) {
    busySkills = new Set([...busySkills, skill.id]);
    distillJobs = { ...distillJobs, [skill.id]: 'running' };
    try {
      const job = await invoke<any>('distill_skill_to_lora', { skillId: skill.id });
      const status = job.training_job_id
        ? `Submitted (job ${job.training_job_id})`
        : `Dataset ready — ${job.dpo_examples} DPO pairs`;
      distillJobs = { ...distillJobs, [skill.id]: status };
      showToast(`Distill started: ${status}`);
    } catch (e) {
      distillJobs = { ...distillJobs, [skill.id]: `Error: ${e}` };
      showToast(`Distill error: ${e}`);
    } finally {
      busySkills.delete(skill.id);
      busySkills = new Set(busySkills);
    }
  }

  async function publishSkill(skill: CompiledSkill) {
    busySkills = new Set([...busySkills, skill.id]);
    try {
      await invoke('publish_compiled_skill_to_marketplace', { skillId: skill.id });
      showToast(`Published: ${skill.name}`);
    } catch (e) {
      showToast(`Publish error: ${e}`);
    } finally {
      busySkills.delete(skill.id);
      busySkills = new Set(busySkills);
    }
  }

  async function testInvokeSkill(skill: CompiledSkill) {
    busySkills = new Set([...busySkills, skill.id]);
    try {
      const result = await invoke<string>('invoke_skill', {
        skillName: skill.name,
        args: { test: true },
      });
      invokeResults = { ...invokeResults, [skill.id]: result || '(empty output)' };
    } catch (e) {
      invokeResults = { ...invokeResults, [skill.id]: `Error: ${e}` };
    } finally {
      busySkills.delete(skill.id);
      busySkills = new Set(busySkills);
    }
  }

  // ── Import tab ───────────────────────────────────────────────────────────────

  async function importFromContent() {
    if (!importContent.trim()) return;
    importing = true;
    importError = '';
    importResult = null;
    try {
      importResult = await invoke<CompiledSkill>('compile_skill_from_content', {
        content: importContent,
        allowSecurityConcerns: importAllowUnsafe,
      });
      showToast(`✅ Installed: ${importResult.name}`);
      importContent = '';
      await loadInstalled();
      activeTab = 'installed';
    } catch (e) {
      importError = String(e);
    } finally {
      importing = false;
    }
  }

  // ── Marketplace tab ───────────────────────────────────────────────────────────

  async function searchMarketplace() {
    if (!searchQuery.trim()) {
      await loadPeerSkills();
      searchResults = peerSkills;
      return;
    }
    searching = true;
    try {
      const all = await invoke<MarketplaceAsset[]>('discover_peer_skills');
      const q = searchQuery.toLowerCase();
      searchResults = all.filter(
        (a) =>
          a.name.toLowerCase().includes(q) ||
          a.description.toLowerCase().includes(q) ||
          a.tags.some((t) => t.toLowerCase().includes(q))
      );
    } catch (e) {
      showToast(`Search error: ${e}`);
    } finally {
      searching = false;
    }
  }

  async function installFromMarketplace(assetId: string) {
    busySkills = new Set([...busySkills, assetId]);
    try {
      const skill = await invoke<CompiledSkill>('install_skill_from_marketplace', {
        assetId,
      });
      showToast(`✅ Installed from marketplace: ${skill.name}`);
      await loadInstalled();
      activeTab = 'installed';
    } catch (e) {
      showToast(`Install error: ${e}`);
    } finally {
      busySkills.delete(assetId);
      busySkills = new Set(busySkills);
    }
  }

  // ── Severity colour helper ────────────────────────────────────────────────────
  function permBadgeClass(perm: string) {
    if (perm === 'run_shell' || perm === 'network') return 'bg-red-800 text-red-200';
    if (perm === 'write_fs') return 'bg-yellow-800 text-yellow-200';
    return 'bg-gray-700 text-gray-300';
  }
</script>

<!-- ── Toast ─────────────────────────────────────────────────────────────────── -->
{#if toast}
  <div class="fixed bottom-4 right-4 z-50 bg-gray-800 border border-gray-600 text-white text-sm px-4 py-2 rounded shadow-lg">
    {toast}
  </div>
{/if}

<!-- ── Panel ──────────────────────────────────────────────────────────────────── -->
<div class="flex flex-col h-full bg-gray-900 text-white overflow-hidden">

  <!-- Header -->
  <div class="flex items-center gap-2 px-4 py-3 border-b border-gray-700">
    <span class="text-lg font-bold">🧩 Skills</span>
    <span class="ml-auto text-xs text-gray-500">
      {installedSkills.length} installed
    </span>
    <button
      class="text-xs text-gray-400 hover:text-white ml-2"
      on:click={loadInstalled}
      title="Refresh"
    >⟳</button>
  </div>

  <!-- Tabs -->
  <div class="flex border-b border-gray-700 text-sm">
    {#each ['installed', 'marketplace', 'import'] as tab}
      <button
        class="px-4 py-2 transition-colors {activeTab === tab
          ? 'border-b-2 border-blue-500 text-white'
          : 'text-gray-400 hover:text-white'}"
        on:click={() => { activeTab = tab as any; if (tab === 'marketplace') loadPeerSkills(); }}
      >
        {tab === 'installed' ? '📦 Installed' : tab === 'marketplace' ? '🌐 Marketplace' : '⬆ Import'}
      </button>
    {/each}
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-3 space-y-2">

    <!-- ── INSTALLED TAB ───────────────────────────────────────────────────── -->
    {#if activeTab === 'installed'}
      {#if loadingInstalled}
        <p class="text-gray-500 text-sm">Loading…</p>
      {:else if installedSkills.length === 0}
        <p class="text-gray-500 text-sm">No skills installed yet. Use Import or Marketplace to add one.</p>
      {:else}
        {#each installedSkills as skill (skill.id)}
          {@const busy = busySkills.has(skill.id)}
          {@const expanded = expandedSkill === skill.id}
          <div class="bg-gray-800 rounded-lg border border-gray-700">
            <!-- Skill header row -->
            <button
              class="w-full flex items-start gap-2 p-3 text-left hover:bg-gray-750"
              on:click={() => (expandedSkill = expanded ? null : skill.id)}
            >
              <span class="text-white font-mono font-semibold truncate flex-1">{skill.name}</span>
              <span class="text-gray-400 text-xs mt-0.5">{expanded ? '▲' : '▼'}</span>
            </button>

            {#if expanded}
              <div class="px-3 pb-3 space-y-2 border-t border-gray-700">
                <p class="text-gray-400 text-xs mt-2">{skill.description}</p>

                <!-- Tags -->
                {#if skill.tags.length > 0}
                  <div class="flex flex-wrap gap-1">
                    {#each skill.tags as tag}
                      <span class="text-xs bg-gray-700 text-gray-300 px-2 py-0.5 rounded">{tag}</span>
                    {/each}
                  </div>
                {/if}

                <!-- Permissions -->
                {#if skill.requires_permissions.length > 0}
                  <div class="flex flex-wrap gap-1">
                    {#each skill.requires_permissions as perm}
                      <span class="text-xs px-2 py-0.5 rounded {permBadgeClass(perm)}">{perm}</span>
                    {/each}
                  </div>
                {/if}

                <!-- Security report -->
                <div class="text-xs {skill.security_report.passed ? 'text-green-400' : 'text-red-400'}">
                  Security: {skill.security_report.passed ? '✅ Pass' : '⚠ ' + skill.security_report.concerns.join(', ')}
                </div>

                <!-- WASM hash -->
                <div class="text-xs text-gray-600 font-mono truncate" title={skill.wasm_hash}>
                  WASM: {skill.wasm_hash.slice(0, 16)}…
                </div>

                <!-- Integrity verify result -->
                {#if skill.id in verifyResults}
                  <div class="text-xs {verifyResults[skill.id] ? 'text-green-400' : 'text-red-400'}">
                    Integrity: {verifyResults[skill.id] ? '✅ Valid' : '❌ Tampered'}
                  </div>
                {/if}

                <!-- Distill status -->
                {#if skill.id in distillJobs}
                  <div class="text-xs text-blue-300">LoRA: {distillJobs[skill.id]}</div>
                {/if}

                <!-- Invoke result -->
                {#if skill.id in invokeResults}
                  <pre class="text-xs text-gray-300 bg-gray-900 rounded p-2 overflow-x-auto max-h-24">{invokeResults[skill.id]}</pre>
                {/if}

                <!-- Rules -->
                {#if skill.rules.length > 0}
                  <details class="text-xs">
                    <summary class="text-gray-400 cursor-pointer">
                      {skill.rules.length} extracted rules
                    </summary>
                    <ul class="mt-1 space-y-0.5 pl-3">
                      {#each skill.rules as rule}
                        <li class="text-gray-400">
                          {#if rule.condition !== 'always'}
                            <span class="text-blue-400">if</span> {rule.condition}
                            <span class="text-blue-400">→</span>
                          {/if}
                          {rule.action}
                          <span class="text-gray-600">({(rule.confidence * 100).toFixed(0)}%)</span>
                        </li>
                      {/each}
                    </ul>
                  </details>
                {/if}

                <!-- Action buttons -->
                <div class="flex flex-wrap gap-2 pt-1">
                  <button
                    class="text-xs px-2 py-1 bg-blue-700 hover:bg-blue-600 rounded disabled:opacity-40"
                    disabled={busy}
                    on:click={() => testInvokeSkill(skill)}
                  >▶ Test</button>
                  <button
                    class="text-xs px-2 py-1 bg-indigo-700 hover:bg-indigo-600 rounded disabled:opacity-40"
                    disabled={busy}
                    on:click={() => verifySkill(skill.id)}
                  >🔒 Verify</button>
                  <button
                    class="text-xs px-2 py-1 bg-violet-700 hover:bg-violet-600 rounded disabled:opacity-40"
                    disabled={busy}
                    on:click={() => distillSkill(skill)}
                    title="Distil rules to a LoRA adapter"
                  >🧠 Distil to LoRA</button>
                  <button
                    class="text-xs px-2 py-1 bg-teal-700 hover:bg-teal-600 rounded disabled:opacity-40"
                    disabled={busy}
                    on:click={() => publishSkill(skill)}
                    title="Publish to local P2P marketplace"
                  >📡 Publish</button>
                  <button
                    class="text-xs px-2 py-1 bg-red-800 hover:bg-red-700 rounded disabled:opacity-40"
                    disabled={busy}
                    on:click={() => uninstallSkill(skill.id)}
                  >🗑 Remove</button>
                </div>
              </div>
            {/if}
          </div>
        {/each}
      {/if}

    <!-- ── MARKETPLACE TAB ─────────────────────────────────────────────────── -->
    {:else if activeTab === 'marketplace'}
      <div class="flex gap-2 mb-3">
        <input
          bind:value={searchQuery}
          placeholder="Search peer skills…"
          class="flex-1 bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-white placeholder-gray-500"
          on:keydown={(e) => e.key === 'Enter' && searchMarketplace()}
        />
        <button
          class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-sm disabled:opacity-40"
          disabled={searching}
          on:click={searchMarketplace}
        >{searching ? '…' : 'Search'}</button>
      </div>

      {#if loadingPeers}
        <p class="text-gray-500 text-sm">Discovering peers…</p>
      {:else}
        {@const list = searchResults.length > 0 ? searchResults : peerSkills}
        {#if list.length === 0}
          <p class="text-gray-500 text-sm">No peer skills found. Ensure other Bonsai instances are running on the same LAN.</p>
        {:else}
          {#each list as asset (asset.id)}
            {@const busy = busySkills.has(asset.id)}
            <div class="bg-gray-800 rounded border border-gray-700 p-3 flex items-start gap-3">
              <div class="flex-1 min-w-0">
                <p class="text-white font-mono font-medium truncate">{asset.name}</p>
                <p class="text-gray-400 text-xs mt-0.5">{asset.description}</p>
                <p class="text-gray-600 text-xs">by {asset.author} · v{asset.version}</p>
                <div class="flex flex-wrap gap-1 mt-1">
                  {#each asset.tags as tag}
                    <span class="text-xs bg-gray-700 text-gray-400 px-1.5 py-0.5 rounded">{tag}</span>
                  {/each}
                </div>
              </div>
              <button
                class="shrink-0 text-xs px-3 py-1.5 bg-green-700 hover:bg-green-600 rounded disabled:opacity-40"
                disabled={busy}
                on:click={() => installFromMarketplace(asset.id)}
              >{busy ? '…' : 'Install'}</button>
            </div>
          {/each}
        {/if}
      {/if}

    <!-- ── IMPORT TAB ──────────────────────────────────────────────────────── -->
    {:else}
      <p class="text-gray-400 text-xs mb-2">
        Paste the contents of a <code class="bg-gray-700 px-1 rounded">SKILL.md</code> file from
        <a href="https://skills.sh" target="_blank" rel="noopener" class="text-blue-400 hover:underline">Skills.sh</a>
        or any compatible source.
      </p>
      <textarea
        bind:value={importContent}
        placeholder="---&#10;name: my-skill&#10;description: …&#10;---&#10;&#10;- Always greet users warmly."
        class="w-full h-48 bg-gray-800 border border-gray-600 rounded px-3 py-2 text-sm text-white font-mono placeholder-gray-600 resize-none"
      />
      <label class="flex items-center gap-2 text-xs text-gray-400 mt-1 select-none">
        <input type="checkbox" bind:checked={importAllowUnsafe} class="accent-yellow-500" />
        Allow skills that fail the security scan (use with caution)
      </label>

      {#if importError}
        <div class="text-red-400 text-xs bg-red-900/30 border border-red-700 rounded p-2 mt-2">
          {importError}
        </div>
      {/if}

      <button
        class="mt-3 w-full py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm font-medium disabled:opacity-40"
        disabled={importing || !importContent.trim()}
        on:click={importFromContent}
      >{importing ? 'Compiling…' : 'Compile & Install'}</button>

      {#if importResult}
        <div class="mt-3 bg-green-900/30 border border-green-700 rounded p-3 text-xs">
          <p class="text-green-300 font-semibold">✅ Installed: {importResult.name}</p>
          <p class="text-gray-400 mt-1">{importResult.description}</p>
          <p class="text-gray-500 mt-1">{importResult.rules.length} rules extracted · {importResult.requires_permissions.length} permissions</p>
        </div>
      {/if}
    {/if}

  </div>
</div>
