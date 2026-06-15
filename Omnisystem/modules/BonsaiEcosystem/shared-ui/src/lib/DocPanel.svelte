<script>
  export let topic = '';
  export let isOpen = false;
  export let onClose = () => {};
  export let devMode = false;

  const docData = {
    'service': {
      simple: 'A "service" is a program running quietly in the background, doing one job for Bonsai — like a worker at a factory.',
      developer: 'Services are managed by the SLM (Service Lifecycle Manager). Each has a ServiceManifest, resource quota, and health check. See: SLM Docs.'
    },
    'capability': {
      simple: 'A "capability" is permission to do something — like a key. Only apps with the right key can access your files or microphone.',
      developer: 'Capabilities are cryptographic tokens signed by UOSC kernel. Granted by the capability-broker service. Revocable at any time.'
    },
    'vault': {
      simple: 'A "vault" is a locked box that runs a program safely, isolated from everything else on your computer.',
      developer: 'Vaults are Sanctum hardware-isolated execution environments. Created via UOSC syscall_create_vault, with TLB/cache partitioning.'
    },
    'snapshot': {
      simple: 'A "snapshot" is a saved picture of an app at one moment — all its data and state. You can restore it anytime.',
      developer: 'Snapshots are content-addressed (BLAKE3) vault memory/register dumps stored in CAS. Restore via UOSC vault_restore syscall.'
    }
  };

  $: currentDoc = docData[topic] || { simple: 'No documentation available for this topic.', developer: 'Topic not found in registry.' };
</script>

<style>
  .doc-panel {
    position: fixed;
    right: 0;
    top: 0;
    width: 320px;
    height: 100vh;
    background-color: var(--surface);
    border-left: 1px solid var(--border);
    transform: translateX(100%);
    transition: transform var(--duration-normal) var(--easing-ease-in-out);
    z-index: var(--z-modal);
    overflow-y: auto;
  }

  .doc-panel.open {
    transform: translateX(0);
  }

  .doc-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-md);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    background-color: var(--surface);
  }

  .doc-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    margin: 0;
  }

  .doc-close {
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    padding: 4px;
    font-size: 20px;
    line-height: 1;
  }

  .doc-close:hover {
    color: var(--accent2);
  }

  .doc-content {
    padding: var(--spacing-md);
  }

  .doc-section {
    margin-bottom: var(--spacing-lg);
  }

  .doc-section-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: var(--spacing-sm);
  }

  .doc-text {
    font-size: var(--font-size-base);
    line-height: 1.6;
    color: var(--text);
    margin: 0;
  }

  .doc-divider {
    height: 1px;
    background-color: var(--border);
    margin: var(--spacing-md) 0;
  }

  .dev-toggle-inline {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm);
    background-color: var(--surface2);
    border-radius: var(--radius);
    margin-top: var(--spacing-md);
    cursor: pointer;
  }

  .dev-toggle-inline:hover {
    background-color: var(--border);
  }

  .toggle-label {
    font-size: var(--font-size-sm);
    color: var(--muted);
  }

  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--duration-normal);
    z-index: calc(var(--z-modal) - 1);
  }

  .overlay.open {
    opacity: 1;
    pointer-events: all;
  }
</style>

<div class="overlay" class:open={isOpen} on:click={onClose} />

<div class="doc-panel" class:open={isOpen}>
  <div class="doc-header">
    <h2 class="doc-title">📖 {topic}</h2>
    <button class="doc-close" on:click={onClose}>✕</button>
  </div>

  <div class="doc-content">
    {#if !devMode}
      <div class="doc-section">
        <div class="doc-section-title">What is this?</div>
        <p class="doc-text">{currentDoc.simple}</p>
        <div class="dev-toggle-inline" on:click={() => (devMode = true)}>
          <span>&lt;/&gt;</span>
          <span class="toggle-label">More detail</span>
        </div>
      </div>
    {:else}
      <div class="doc-section">
        <div class="doc-section-title">Simple</div>
        <p class="doc-text">{currentDoc.simple}</p>
      </div>

      <div class="doc-divider" />

      <div class="doc-section">
        <div class="doc-section-title">Developer</div>
        <p class="doc-text">{currentDoc.developer}</p>
        <div class="dev-toggle-inline" on:click={() => (devMode = false)}>
          <span>&lt;/&gt;</span>
          <span class="toggle-label">Simpler</span>
        </div>
      </div>
    {/if}
  </div>
</div>
