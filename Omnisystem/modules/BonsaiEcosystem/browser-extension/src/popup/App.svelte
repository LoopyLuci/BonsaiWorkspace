<script lang="ts">
  import browser from '../lib/browser';
  import BigButton from '../lib/components/BigButton.svelte';

  let prompt = '';
  let response = '';
  let status: 'checking' | 'connected' | 'disconnected' = 'checking';
  let busy = false;

  async function connect() {
    busy = true;
    const res = await browser.runtime.sendMessage({ type: 'CONNECT' });
    status = res.ok ? 'connected' : 'disconnected';
    busy = false;
  }

  async function summarizePage() {
    busy = true;
    response = '';
    const res = await browser.runtime.sendMessage({ type: 'SUMMARIZE_CURRENT_PAGE' });
    if (!res.ok) {
      response = res.error;
      busy = false;
      return;
    }

    const text = res.data?.choices?.[0]?.message?.content ?? JSON.stringify(res.data, null, 2);
    response = text;
    busy = false;
  }

  async function askPage() {
    if (!prompt.trim()) return;
    busy = true;
    response = '';

    const snapshotRes = await browser.runtime.sendMessage({ type: 'GET_PAGE_SNAPSHOT' });
    if (!snapshotRes.ok) {
      response = snapshotRes.error;
      busy = false;
      return;
    }

    const page = snapshotRes.data;
    const chatRes = await browser.runtime.sendMessage({
      type: 'CHAT',
      messages: [
        {
          role: 'user',
          content: `${prompt}\n\nContext:\nTitle: ${page.title}\nURL: ${page.url}\n${page.visibleText.slice(0, 14000)}`
        }
      ]
    });

    if (!chatRes.ok) {
      response = chatRes.error;
      busy = false;
      return;
    }

    response = chatRes.data?.choices?.[0]?.message?.content ?? JSON.stringify(chatRes.data, null, 2);
    busy = false;
  }

  async function openWorkspace() {
    await browser.runtime.sendMessage({ type: 'OPEN_WORKSPACE' });
    window.close();
  }

  void (async () => {
    const result = await browser.runtime.sendMessage({ type: 'GET_STATUS' });
    status = result.ok && result.data?.connected ? 'connected' : 'disconnected';
  })();
</script>

<main style="width: 360px; padding: 12px; display: grid; gap: 10px;">
  <section class="card" style="display: flex; justify-content: space-between; align-items: center;">
    <strong>Bonsai Buddy</strong>
    <span class={`badge ${status === 'connected' ? 'ok' : 'offline'}`}>
      {status === 'checking' ? 'Checking...' : status}
    </span>
  </section>

  <section class="card" style="display: grid; gap: 8px;">
    <BigButton on:click={connect} disabled={busy}>Connect To Bonsai</BigButton>
    <BigButton variant="secondary" on:click={summarizePage} disabled={busy}>Summarize This Page</BigButton>
    <BigButton variant="secondary" on:click={openWorkspace}>Open Workspace</BigButton>
  </section>

  <section class="card" style="display: grid; gap: 8px;">
    <label for="ask">Ask about current page</label>
    <textarea id="ask" bind:value={prompt} rows="3" placeholder="What is this page about?"></textarea>
    <button on:click={askPage} disabled={busy}>Ask Buddy</button>
  </section>

  {#if response}
    <section class="card" style="max-height: 220px; overflow: auto; white-space: pre-wrap;">
      {response}
    </section>
  {/if}
</main>
