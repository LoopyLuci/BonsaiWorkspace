<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';

  // ── Types ────────────────────────────────────────────────────────────────��───

  interface WalletInfo { balance: number; community_pool_balance: number; }
  interface DeviceInfo {
    urv: number; device_class: string; credits_per_minute_full: number;
    free_tier_pct: number; paid_pct: number; paid_bonus_multiplier: number;
    live_cpu_utilization: number; live_ram_utilization: number;
  }
  interface DeviceListing {
    device_id: string; display_name: string; urv: number; device_class_str: string;
    free_tier_pct: number; paid_pct: number; price_per_minute: number;
    reliability_score: number; is_online: boolean; live_utilization: number;
  }
  interface FreePoolStatus {
    total_cpu_urv: number; total_gpu_urv: number; active_projects: number;
    per_project_cpu_urv: number; per_project_gpu_urv: number; device_count: number;
  }
  interface TaskProfile { task_type: string; unit_name: string; urv_minutes_per_unit: number; }
  interface ProjectEstimateResponse {
    total_urv_minutes: number; eta_minutes: number; eta_low_minutes: number;
    eta_high_minutes: number; credits_per_minute: number; estimated_total_credits: number;
    confidence: number; free_tier_eta_minutes: number | null; paid_savings_minutes: number | null;
  }
  interface RentalContract {
    contract_id: string; device_id: string; reserved_minutes: number;
    price_per_minute: number; status: string;
  }

  // ── State ────────────────────────────────────────────────────────────────────

  type Tab = 'overview' | 'contribute' | 'marketplace' | 'estimate' | 'wallet';
  let activeTab: Tab = 'overview';

  let wallet: WalletInfo | null = null;
  let myDevice: DeviceInfo | null = null;
  let freePool: FreePoolStatus | null = null;
  let listings: DeviceListing[] = [];
  let profiles: TaskProfile[] = [];
  let activeContracts: RentalContract[] = [];

  // Contribution form
  let freeTierPct = 15;
  let paidPct = 80;
  let displayName = '';
  let priceMultiplier = 1.0;
  let savingContribution = false;
  let contributionMsg = '';

  // Estimator
  let estTaskType = '';
  let estUnits = 1000;
  let estPaidDevices = 1;
  let estPaidUrv = 100;
  let estimate: ProjectEstimateResponse | null = null;
  let estimating = false;

  // Marketplace
  let filterQuery = '';
  let filterRequireGpu = false;
  let reservingId = '';

  const ALL_TABS: Tab[] = ['overview', 'contribute', 'marketplace', 'estimate', 'wallet'];

  let loading = false;
  let error = '';
  let refreshInterval: ReturnType<typeof setInterval>;

  // ── Lifecycle ────────────────────────────────────────────────────────────────

  onMount(async () => {
    await refresh();
    refreshInterval = setInterval(refresh, 15000);
  });

  onDestroy(() => clearInterval(refreshInterval));

  async function refresh() {
    try {
      [wallet, myDevice, freePool, profiles] = await Promise.all([
        invoke<WalletInfo>('credits_balance'),
        invoke<DeviceInfo>('my_device_info'),
        invoke<FreePoolStatus>('free_pool_status'),
        invoke<TaskProfile[]>('list_task_profiles'),
      ]);
      listings = await invoke<DeviceListing[]>('marketplace_list', {
        filter: filterRequireGpu ? { requires_gpu: true, min_urv: null, max_price_per_min: null, min_reliability: null } : null,
      });
      error = '';
    } catch (e) {
      error = `Refresh failed: ${e}`;
    }
  }

  async function saveContribution() {
    savingContribution = true;
    contributionMsg = '';
    try {
      myDevice = await invoke<DeviceInfo>('set_contribution', {
        freeTierPct, paidPct, displayName: displayName || 'My Device', priceMultiplier,
      });
      contributionMsg = '✓ Contribution settings saved';
    } catch (e) {
      contributionMsg = `Error: ${e}`;
    } finally {
      savingContribution = false;
    }
  }

  async function reserveDevice(deviceId: string, minutes: number) {
    reservingId = deviceId;
    try {
      const contract = await invoke<RentalContract>('reserve_device', { deviceId, minutes });
      activeContracts = [...activeContracts, contract];
    } catch (e) {
      error = `Reserve failed: ${e}`;
    } finally {
      reservingId = '';
    }
  }

  async function cancelContract(contractId: string) {
    try {
      await invoke('cancel_reservation', { contractId });
      activeContracts = activeContracts.filter(c => c.contract_id !== contractId);
    } catch (e) {
      error = `Cancel failed: ${e}`;
    }
  }

  async function runEstimate() {
    estimating = true;
    estimate = null;
    try {
      estimate = await invoke<ProjectEstimateResponse>('estimate_project', {
        taskType: estTaskType,
        units: estUnits,
        numPaidDevices: estPaidDevices,
        totalPaidUrvPerMin: estPaidUrv,
      });
    } catch (e) {
      error = `Estimate failed: ${e}`;
    } finally {
      estimating = false;
    }
  }

  // ── Helpers ───────────────────────────────────────────────────────────────────

  function fmtUrv(v: number) { return v.toFixed(1); }
  function fmtCr(v: number)  { return v.toFixed(4); }
  function fmtMin(v: number) {
    if (v < 60) return `${Math.round(v)}m`;
    return `${Math.floor(v/60)}h ${Math.round(v%60)}m`;
  }
  function classColor(cls: string): string {
    const map: Record<string,string> = {
      Phone: '#60a5fa', Tablet: '#818cf8', Laptop: '#34d399',
      Desktop: '#fbbf24', HighEndDesktop: '#f97316', Server: '#ef4444', Supercomputer: '#a855f7',
    };
    return map[cls] ?? '#6b7280';
  }

  $: filteredListings = listings.filter(l =>
    !filterQuery || l.display_name.toLowerCase().includes(filterQuery.toLowerCase())
  );
</script>

<div class="cmp-root">

  <!-- Header -->
  <div class="cmp-header">
    <h2 class="cmp-title">⚡ Cluster Credits</h2>
    {#if wallet}
      <div class="wallet-badge">
        <span class="wallet-bal">{fmtCr(wallet.balance)} cr</span>
        <span class="wallet-pool" title="Community pool">Pool: {fmtCr(wallet.community_pool_balance)}</span>
      </div>
    {/if}
  </div>

  {#if error}
    <div class="cmp-error" role="alert">{error}</div>
  {/if}

  <!-- Tab bar -->
  <div class="tab-bar" role="tablist">
    {#each ALL_TABS as tab}
      <button
        class="tab-btn" class:active={activeTab === tab}
        role="tab" aria-selected={activeTab === tab}
        on:click={() => activeTab = tab}
      >{tab.charAt(0).toUpperCase() + tab.slice(1)}</button>
    {/each}
  </div>

  <!-- ── OVERVIEW ─────────────────────────────────────────────────────────── -->
  {#if activeTab === 'overview'}
    <div class="tab-content">

      <!-- My device card -->
      {#if myDevice}
        <div class="card">
          <div class="card-header">
            <span>My Device</span>
            <span class="device-class-badge" style="background:{classColor(myDevice.device_class)}22;color:{classColor(myDevice.device_class)}">
              {myDevice.device_class}
            </span>
          </div>
          <div class="stat-grid">
            <div class="stat"><span class="stat-label">URV</span><span class="stat-value">{fmtUrv(myDevice.urv)}</span></div>
            <div class="stat"><span class="stat-label">CPU</span><span class="stat-value">{(myDevice.live_cpu_utilization*100).toFixed(0)}%</span></div>
            <div class="stat"><span class="stat-label">RAM</span><span class="stat-value">{(myDevice.live_ram_utilization*100).toFixed(0)}%</span></div>
            <div class="stat"><span class="stat-label">Free tier</span><span class="stat-value">{myDevice.free_tier_pct}%</span></div>
            <div class="stat"><span class="stat-label">Paid</span><span class="stat-value">{myDevice.paid_pct}%</span></div>
            <div class="stat"><span class="stat-label">Bonus</span><span class="stat-value green">{myDevice.paid_bonus_multiplier.toFixed(2)}×</span></div>
          </div>
          <div class="earn-rate">
            Earning at full load: <strong>{fmtCr(myDevice.credits_per_minute_full)} cr/min</strong>
          </div>
        </div>
      {/if}

      <!-- Free pool card -->
      {#if freePool}
        <div class="card">
          <div class="card-header">Free Cloud Pool</div>
          <div class="stat-grid">
            <div class="stat"><span class="stat-label">Devices</span><span class="stat-value">{freePool.device_count}</span></div>
            <div class="stat"><span class="stat-label">CPU URV</span><span class="stat-value">{fmtUrv(freePool.total_cpu_urv)}</span></div>
            <div class="stat"><span class="stat-label">GPU URV</span><span class="stat-value">{fmtUrv(freePool.total_gpu_urv)}</span></div>
            <div class="stat"><span class="stat-label">Projects</span><span class="stat-value">{freePool.active_projects}</span></div>
          </div>
          {#if freePool.active_projects > 0}
            <div class="pool-share">
              Each project gets <strong>{fmtUrv(freePool.per_project_cpu_urv)} CPU URV/min</strong>
              {#if freePool.per_project_gpu_urv > 0}
                + <strong>{fmtUrv(freePool.per_project_gpu_urv)} GPU URV/min</strong>
              {/if}
            </div>
          {:else}
            <div class="pool-share muted">No active free projects — pool is idle</div>
          {/if}
        </div>
      {/if}
    </div>

  <!-- ── CONTRIBUTE ────────────────────────────────────────────────────────── -->
  {:else if activeTab === 'contribute'}
    <div class="tab-content">
      <div class="card">
        <div class="card-header">Resource Contribution Settings</div>

        <div class="form-row">
          <label for="cc-display-name">Device name</label>
          <input id="cc-display-name" type="text" bind:value={displayName} placeholder="My Desktop" />
        </div>

        <div class="form-row">
          <label for="cc-free-pct">Free tier (<em>max 15%</em>)</label>
          <div class="slider-row">
            <input id="cc-free-pct" type="range" min="0" max="15" step="1" bind:value={freeTierPct} />
            <span class="slider-val">{freeTierPct}%</span>
          </div>
          <p class="hint">
            At 15% free you earn a <strong>{myDevice ? myDevice.paid_bonus_multiplier.toFixed(2) : '1.20'}×</strong>
            paid earnings bonus — worth more than the donated capacity.
          </p>
        </div>

        <div class="form-row">
          <label for="cc-paid-pct">Paid rental</label>
          <div class="slider-row">
            <input id="cc-paid-pct" type="range" min="0" max={100 - freeTierPct} step="5" bind:value={paidPct} />
            <span class="slider-val">{paidPct}%</span>
          </div>
        </div>

        <div class="form-row">
          <label for="cc-price-mult">Price multiplier</label>
          <div class="slider-row">
            <input id="cc-price-mult" type="range" min="1.0" max="3.0" step="0.1" bind:value={priceMultiplier} />
            <span class="slider-val">{priceMultiplier.toFixed(1)}×</span>
          </div>
          {#if myDevice}
            <p class="hint">
              Listed price: <strong>{fmtCr(myDevice.credits_per_minute_full * (paidPct/100) * priceMultiplier)} cr/min</strong>
            </p>
          {/if}
        </div>

        <button class="btn-primary" on:click={saveContribution} disabled={savingContribution}>
          {savingContribution ? 'Saving…' : 'Save Settings'}
        </button>
        {#if contributionMsg}
          <p class="msg" class:msg-ok={contributionMsg.startsWith('✓')}>{contributionMsg}</p>
        {/if}
      </div>
    </div>

  <!-- ── MARKETPLACE ───────────────────────────────────────────────────────── -->
  {:else if activeTab === 'marketplace'}
    <div class="tab-content">

      {#if activeContracts.length > 0}
        <div class="card">
          <div class="card-header">Active Rentals</div>
          {#each activeContracts as c (c.contract_id)}
            <div class="contract-row">
              <div>
                <strong>{c.device_id.slice(0,8)}…</strong>
                <span class="muted">{c.reserved_minutes}min @ {fmtCr(c.price_per_minute)} cr/min</span>
              </div>
              <button class="btn-sm btn-danger" on:click={() => cancelContract(c.contract_id)}>Cancel</button>
            </div>
          {/each}
        </div>
      {/if}

      <div class="search-row">
        <input type="text" placeholder="Search devices…" bind:value={filterQuery} class="search-input" />
        <label class="check-label">
          <input type="checkbox" bind:checked={filterRequireGpu} on:change={refresh} />
          GPU only
        </label>
      </div>

      {#if filteredListings.length === 0}
        <div class="empty-state">No devices available. Contribute yours to grow the marketplace.</div>
      {:else}
        <div class="device-grid">
          {#each filteredListings as d (d.device_id)}
            <div class="device-card">
              <div class="device-card-header">
                <span class="device-name">{d.display_name}</span>
                <span class="device-class-badge" style="background:{classColor(d.device_class_str)}22;color:{classColor(d.device_class_str)}">
                  {d.device_class_str}
                </span>
              </div>
              <div class="device-stats">
                <span><strong>{fmtUrv(d.urv)}</strong> URV</span>
                <span>Util: {(d.live_utilization*100).toFixed(0)}%</span>
                <span>Rely: {(d.reliability_score*100).toFixed(0)}%</span>
              </div>
              <div class="device-price">
                <strong>{fmtCr(d.price_per_minute)}</strong> cr/min
                <span class="muted">({fmtCr(d.price_per_minute * d.live_utilization)} live)</span>
              </div>
              <div class="device-actions">
                <button class="btn-sm btn-primary"
                  disabled={reservingId === d.device_id}
                  on:click={() => reserveDevice(d.device_id, 60)}
                >{reservingId === d.device_id ? '…' : 'Rent 1h'}</button>
                <button class="btn-sm btn-secondary"
                  disabled={reservingId === d.device_id}
                  on:click={() => reserveDevice(d.device_id, 10)}
                >10 min</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  <!-- ── ESTIMATE ──────────────────────────────────────────────────────────── -->
  {:else if activeTab === 'estimate'}
    <div class="tab-content">
      <div class="card">
        <div class="card-header">Project Cost Estimator</div>

        <div class="form-row">
          <label for="est-type">Task type</label>
          <select id="est-type" bind:value={estTaskType}>
            <option value="">— choose —</option>
            {#each profiles as p}
              <option value={p.task_type}>{p.task_type.replace(/_/g,' ')} (per {p.unit_name})</option>
            {/each}
          </select>
        </div>

        <div class="form-row">
          <label for="est-units">Units</label>
          <input id="est-units" type="number" min="1" bind:value={estUnits} />
        </div>

        <div class="form-row">
          <label for="est-paid-urv">Paid URV/min (0 = free tier only)</label>
          <div class="slider-row">
            <input id="est-paid-urv" type="range" min="0" max="2000" step="10" bind:value={estPaidUrv} />
            <span class="slider-val">{estPaidUrv}</span>
          </div>
        </div>

        <button class="btn-primary" disabled={!estTaskType || estimating} on:click={runEstimate}>
          {estimating ? 'Estimating…' : 'Estimate'}
        </button>
      </div>

      {#if estimate}
        <div class="card estimate-card">
          <div class="card-header">Estimate Results</div>
          <div class="est-grid">
            <div class="est-row">
              <span>Total work</span>
              <strong>{estimate.total_urv_minutes.toFixed(0)} URV-min</strong>
            </div>
            {#if estimate.free_tier_eta_minutes}
              <div class="est-row muted">
                <span>Free tier ETA</span>
                <strong>{fmtMin(estimate.free_tier_eta_minutes)}</strong>
              </div>
            {/if}
            {#if estPaidUrv > 0}
              <div class="est-row green">
                <span>With paid devices ETA</span>
                <strong>{fmtMin(estimate.eta_minutes)}</strong>
                <span class="hint">(±{fmtMin(estimate.eta_high_minutes - estimate.eta_minutes)})</span>
              </div>
              <div class="est-row">
                <span>Cost/min</span>
                <strong>{fmtCr(estimate.credits_per_minute)} cr/min</strong>
              </div>
              <div class="est-row amber">
                <span>Estimated total</span>
                <strong>{fmtCr(estimate.estimated_total_credits)} credits</strong>
              </div>
              {#if estimate.paid_savings_minutes}
                <div class="est-row green">
                  <span>Time saved vs free</span>
                  <strong>−{fmtMin(estimate.paid_savings_minutes)}</strong>
                </div>
              {/if}
            {/if}
          </div>
          <div class="confidence-bar">
            <span>Confidence</span>
            <div class="bar-track">
              <div class="bar-fill" style="width:{(estimate.confidence*100).toFixed(0)}%"></div>
            </div>
            <span>{(estimate.confidence*100).toFixed(0)}%</span>
          </div>
        </div>
      {/if}
    </div>

  <!-- ── WALLET ────────────────────────────────────────────────────────────── -->
  {:else if activeTab === 'wallet'}
    <div class="tab-content">
      {#if wallet}
        <div class="card">
          <div class="card-header">Balance</div>
          <div class="wallet-detail">
            <div class="wallet-row">
              <span>My balance</span><strong>{fmtCr(wallet.balance)} credits</strong>
            </div>
            <div class="wallet-row muted">
              <span>Community pool</span><strong>{fmtCr(wallet.community_pool_balance)} credits</strong>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .cmp-root {
    display: flex; flex-direction: column; height: 100%;
    background: #0d0f1a; color: #e2e8f0; font-size: 13px; overflow: hidden;
  }
  .cmp-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px; border-bottom: 1px solid #1e2030;
  }
  .cmp-title { font-size: 15px; font-weight: 600; margin: 0; }
  .wallet-badge { display: flex; gap: 10px; align-items: center; }
  .wallet-bal { font-size: 15px; font-weight: 700; color: #fbbf24; }
  .wallet-pool { font-size: 11px; color: #6b7280; }
  .cmp-error { background: #7f1d1d33; border: 1px solid #ef4444; color: #fca5a5;
    padding: 8px 12px; margin: 8px 12px; border-radius: 6px; }
  .tab-bar { display: flex; border-bottom: 1px solid #1e2030; padding: 0 8px; gap: 2px; }
  .tab-btn {
    padding: 8px 14px; background: none; border: none; color: #6b7280;
    cursor: pointer; font-size: 12px; border-bottom: 2px solid transparent;
    transition: all .15s;
  }
  .tab-btn.active { color: #818cf8; border-bottom-color: #818cf8; }
  .tab-btn:hover:not(.active) { color: #94a3b8; }
  .tab-content { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
  .card { background: #111827; border: 1px solid #1e2030; border-radius: 10px; padding: 14px; }
  .card-header { font-weight: 600; margin-bottom: 10px; color: #c7d2fe; font-size: 12px; text-transform: uppercase; letter-spacing: .05em; }
  .stat-grid { display: grid; grid-template-columns: repeat(3,1fr); gap: 8px; }
  .stat { background: #0d1117; border-radius: 6px; padding: 8px; text-align: center; }
  .stat-label { display: block; font-size: 10px; color: #6b7280; margin-bottom: 3px; }
  .stat-value { font-size: 16px; font-weight: 700; color: #f1f5f9; }
  .stat-value.green { color: #34d399; }
  .earn-rate { margin-top: 10px; color: #94a3b8; font-size: 12px; text-align: center; }
  .pool-share { margin-top: 8px; font-size: 12px; color: #94a3b8; text-align: center; }
  .muted { color: #6b7280; }
  .green { color: #34d399; }
  .amber { color: #fbbf24; }
  .device-class-badge {
    font-size: 10px; padding: 2px 8px; border-radius: 10px; font-weight: 600;
  }
  .form-row { display: flex; flex-direction: column; gap: 5px; margin-bottom: 14px; }
  .form-row label { font-size: 11px; color: #94a3b8; font-weight: 500; }
  .form-row input[type=text], .form-row input[type=number], .form-row select {
    background: #0d1117; border: 1px solid #374151; border-radius: 6px;
    color: #e2e8f0; padding: 7px 10px; font-size: 13px;
  }
  .slider-row { display: flex; align-items: center; gap: 10px; }
  .slider-row input[type=range] { flex: 1; accent-color: #818cf8; }
  .slider-val { min-width: 40px; font-size: 13px; font-weight: 600; color: #818cf8; }
  .hint { font-size: 11px; color: #6b7280; margin: 4px 0 0; }
  .btn-primary {
    background: #4f46e5; color: #fff; border: none; border-radius: 7px;
    padding: 9px 18px; cursor: pointer; font-size: 13px; font-weight: 600;
    transition: background .15s;
  }
  .btn-primary:hover:not(:disabled) { background: #6366f1; }
  .btn-primary:disabled { opacity: .5; cursor: not-allowed; }
  .btn-sm { padding: 4px 10px; border-radius: 5px; font-size: 11px; font-weight: 600; cursor: pointer; border: none; }
  .btn-sm.btn-primary { background: #4f46e5; color: #fff; }
  .btn-sm.btn-secondary { background: #1e2030; color: #94a3b8; }
  .btn-sm.btn-danger { background: #7f1d1d; color: #fca5a5; }
  .btn-sm:disabled { opacity: .4; cursor: not-allowed; }
  .msg { font-size: 12px; margin-top: 8px; color: #94a3b8; }
  .msg.msg-ok { color: #34d399; }
  .search-row { display: flex; gap: 10px; align-items: center; margin-bottom: 8px; }
  .search-input { flex: 1; background: #0d1117; border: 1px solid #374151; border-radius: 6px;
    color: #e2e8f0; padding: 7px 10px; font-size: 13px; }
  .check-label { display: flex; align-items: center; gap: 5px; font-size: 12px; color: #94a3b8; cursor: pointer; }
  .device-grid { display: grid; grid-template-columns: repeat(auto-fill,minmax(200px,1fr)); gap: 10px; }
  .device-card { background: #111827; border: 1px solid #1e2030; border-radius: 9px; padding: 12px; }
  .device-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .device-name { font-weight: 600; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 110px; }
  .device-stats { display: flex; gap: 8px; font-size: 11px; color: #94a3b8; margin-bottom: 8px; flex-wrap: wrap; }
  .device-price { font-size: 13px; margin-bottom: 8px; color: #fbbf24; }
  .device-price .muted { font-size: 11px; }
  .device-actions { display: flex; gap: 6px; }
  .contract-row { display: flex; justify-content: space-between; align-items: center;
    padding: 8px; background: #0d1117; border-radius: 6px; margin-bottom: 6px; }
  .empty-state { text-align: center; color: #6b7280; padding: 40px; }
  .est-grid { display: flex; flex-direction: column; gap: 8px; }
  .est-row { display: flex; justify-content: space-between; align-items: baseline;
    padding: 6px 10px; background: #0d1117; border-radius: 6px; font-size: 13px; }
  .est-row.green { border-left: 3px solid #34d399; }
  .est-row.amber { border-left: 3px solid #fbbf24; }
  .est-row.muted { opacity: .7; }
  .confidence-bar { display: flex; gap: 8px; align-items: center; margin-top: 12px; font-size: 11px; color: #94a3b8; }
  .bar-track { flex: 1; height: 6px; background: #1e2030; border-radius: 3px; overflow: hidden; }
  .bar-fill { height: 100%; background: #4f46e5; transition: width .3s; }
  .wallet-detail { display: flex; flex-direction: column; gap: 8px; }
  .wallet-row { display: flex; justify-content: space-between; align-items: center;
    padding: 10px; background: #0d1117; border-radius: 6px; }
</style>
