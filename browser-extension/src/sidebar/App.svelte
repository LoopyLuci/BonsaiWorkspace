<script lang="ts">
  import browser from '../lib/browser';
  import type { AuditEntry } from '../lib/types';

  let input = '';
  let streamText = '';
  let busy = false;
  let audit: AuditEntry[] = [];
  let streamId = '';

  const runtimeListener = (msg: any) => {
    if (msg?.type !== 'EXTENSION_EVENT') return;
    const event = msg.event;
    if (event.type === 'CHAT_TOKEN' && event.streamId === streamId) {
      streamText += event.token;
    }
    if (event.type === 'CHAT_DONE' && event.streamId === streamId) {
      busy = false;
    }
    if (event.type === 'AUDIT_UPDATED') {
      void refreshAudit();
    }
  };

  async function refreshAudit() {
    const res = await browser.runtime.sendMessage({ type: 'GET_AUDIT_LOG' });
    if (res.ok) {
      audit = (res.data ?? []) as AuditEntry[];
    }
  }

  async function ask() {
    if (!input.trim()) return;
    busy = true;
    streamText = '';
    streamId = crypto.randomUUID();

    const res = await browser.runtime.sendMessage({
      type: 'CHAT_STREAM',
      streamId,
      messages: [{ role: 'user', content: input }]
    });

    if (!res.ok) {
      streamText = res.error;
      busy = false;
    }

  }

  async function clearAudit() {
    await browser.runtime.sendMessage({ type: 'CLEAR_AUDIT_LOG' });
    await refreshAudit();
  }

  browser.runtime.onMessage.addListener(runtimeListener);
  void refreshAudit();

  window.addEventListener('beforeunload', () => {
    browser.runtime.onMessage.removeListener(runtimeListener);
  });
</script>

<main style="padding: 14px; display: grid; gap: 12px;">
  <section class="card" style="display: grid; gap: 8px;">
    <h2 style="margin: 0;">Bonsai Buddy Sidebar</h2>
    <textarea bind:value={input} rows="3" placeholder="Ask Buddy anything"></textarea>
    <button on:click={ask} disabled={busy}>Send</button>
  </section>

  <section class="card" style="min-height: 120px; white-space: pre-wrap;">
    {#if streamText}
      {streamText}
    {:else}
      Streaming response appears here...
    {/if}
  </section>

  <section class="card">
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
      <strong>Action History</strong>
      <button class="secondary" on:click={clearAudit}>Clear</button>
    </div>
    <div style="max-height: 260px; overflow: auto; display: grid; gap: 8px;">
      {#if audit.length === 0}
        <div>No actions logged yet.</div>
      {:else}
        {#each audit as item}
          <article style="border: 1px solid #d7c7ae; border-radius: 10px; padding: 8px;">
            <div style="font-weight: 700;">{item.action} · {item.result}</div>
            <div style="font-size: 12px; color: var(--muted);">{item.url}</div>
            {#if item.message}
              <div style="font-size: 12px;">{item.message}</div>
            {/if}
          </article>
        {/each}
      {/if}
    </div>
  </section>
</main>
