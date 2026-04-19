<script lang="ts">
  import DOMPurify from 'dompurify';

  export let toolName: string;
  export let resultJson: string;

  let parsed: any = null;
  try { parsed = JSON.parse(resultJson); } catch { parsed = null; }

  $: isWeather = toolName === 'get_weather' && parsed;
  $: isChart   = toolName === 'render_chart' && parsed?.svg;
  $: isFiles   = toolName === 'find_files' && parsed?.files;
  $: isStats   = toolName === 'get_system_stats' && parsed;
  $: isEmail   = toolName === 'send_email' && parsed;
  $: isWebFetch = toolName === 'fetch_url' && parsed?.text;

  // Always sanitize any HTML/SVG before rendering with {@html}
  function safeHtml(raw: string): string {
    return DOMPurify.sanitize(raw, {
      USE_PROFILES: { svg: true, html: false },
      FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed', 'form'],
      FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onfocus', 'onmouseover'],
    });
  }
</script>

{#if isWeather}
  <div class="card weather">
    <div class="row">
      <span class="big">{parsed.temperature_c}°C</span>
      <span class="cond">{parsed.condition}</span>
    </div>
    <div class="meta">
      📍 {parsed.location} &nbsp;·&nbsp; 💨 {parsed.wind_kmh} km/h &nbsp;·&nbsp; 💧 {parsed.humidity_pct}%
    </div>
  </div>

{:else if isChart}
  <div class="chart-wrap">
    {@html safeHtml(parsed.svg)}
  </div>

{:else if isWebFetch}
  <div class="card web-fetch">
    <div class="label">🌐 {parsed.url ?? 'Fetched content'}</div>
    <pre class="raw">{String(parsed.text).slice(0, 600)}</pre>
  </div>

{:else if isFiles}
  <div class="card files">
    <div class="label">Found {parsed.count} file{parsed.count !== 1 ? 's' : ''}</div>
    <ul>
      {#each parsed.files.slice(0, 20) as f}
        <li>{f}</li>
      {/each}
      {#if parsed.files.length > 20}
        <li class="more">…and {parsed.files.length - 20} more</li>
      {/if}
    </ul>
  </div>

{:else if isStats}
  <div class="card stats">
    <div class="stat-row">
      <span>CPU</span><span>{parsed.cpu_usage_pct}%</span>
    </div>
    <div class="stat-row">
      <span>RAM</span>
      <span>{parsed.memory_used_mb} / {parsed.memory_total_mb} MB ({parsed.memory_used_pct}%)</span>
    </div>
  </div>

{:else if isEmail}
  <div class="card email">
    ✅ Email sent to <strong>{parsed.to}</strong>
  </div>

{:else}
  <pre class="raw">{resultJson.slice(0, 512)}</pre>
{/if}

<style>
  .card {
    border: 1px solid var(--border, #3e3e42);
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 0.82rem;
    background: var(--bg, #1e1e1e);
    margin: 2px 0;
  }
  .weather .row { display: flex; align-items: baseline; gap: 8px; }
  .weather .big { font-size: 1.4rem; font-weight: 700; }
  .weather .cond { color: var(--fg-dim, #888); }
  .weather .meta { color: var(--fg-dim, #888); margin-top: 4px; font-size: 0.78rem; }
  .files ul { margin: 4px 0 0; padding-left: 16px; }
  .files li { font-family: monospace; font-size: 0.78rem; color: var(--fg, #ccc); }
  .files .more { color: var(--fg-dim, #888); }
  .files .label { font-weight: 600; margin-bottom: 4px; }
  .stats .stat-row { display: flex; justify-content: space-between; padding: 2px 0; }
  .chart-wrap { display: flex; justify-content: center; margin: 4px 0; }
  .raw { font-family: monospace; font-size: 0.78rem; white-space: pre-wrap; word-break: break-all; color: var(--fg-dim, #888); margin: 0; }
</style>
