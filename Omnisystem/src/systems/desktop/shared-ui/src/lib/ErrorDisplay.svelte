<script>
  export let errorCode = '';
  export let errorMessage = '';
  export let onDocClick = () => {};

  // Error registry (E001–E599)
  const errorRegistry = {
    'E001': { simple: 'Could not download a file', developer: 'Network timeout or invalid source URL. Check your connection.' },
    'E002': { simple: 'File was damaged', developer: 'BLAKE3 hash mismatch during download. File corrupted or incomplete.' },
    'E003': { simple: 'Not enough disk space', developer: 'Install requires at least 50MB for base system (excluding models).' },
    'E101': { simple: 'Application failed to start', developer: 'Binary exited with non-zero code. Check logs for details.' },
    'E102': { simple: 'Port already in use', developer: 'Another application is using the required port. Try changing ports in settings.' },
    'E201': { simple: 'Service stopped unexpectedly', developer: 'Service supervisor detected exit. Check service logs and restart.' },
    'E202': { simple: 'Capability denied', developer: 'The requested capability token was rejected by the kernel. Grant via Control Panel.' },
    'E301': { simple: 'Network error during download', developer: 'Connection lost or server unreachable. Download will resume from last verified chunk.' },
    'E401': { simple: 'Identity verification failed', developer: 'Invalid signature or malformed identity key. Recover from BIP-39 seed or FIDO2 device.' },
    'E501': { simple: 'Kernel error', developer: 'Low-level UOSC hypercall failed. System may be unstable. Try "Heal" from Control Panel.' }
  };

  $: errorInfo = errorRegistry[errorCode] || { simple: errorMessage, developer: 'Unknown error. Check documentation.' };
</script>

<style>
  .error-display {
    display: flex;
    gap: 12px;
    padding: 12px;
    background-color: var(--surface2);
    border: 1px solid var(--error);
    border-radius: var(--radius);
    margin-bottom: var(--spacing-md);
  }

  .error-icon {
    font-size: 20px;
    flex-shrink: 0;
  }

  .error-content {
    flex: 1;
  }

  .error-code {
    font-size: var(--font-size-sm);
    color: var(--muted);
    font-family: monospace;
    margin-bottom: 4px;
  }

  .error-message {
    font-size: var(--font-size-base);
    color: var(--text);
    margin-bottom: 8px;
  }

  .error-action {
    display: flex;
    gap: 8px;
  }

  .doc-button {
    padding: 4px 8px;
    background-color: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--accent2);
    cursor: pointer;
    font-size: var(--font-size-sm);
    transition: all var(--duration-fast);
  }

  .doc-button:hover {
    background-color: var(--surface2);
    border-color: var(--accent2);
  }

  .dev-detail {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    font-size: var(--font-size-sm);
    color: var(--muted);
    font-family: monospace;
  }
</style>

<div class="error-display" role="alert">
  <div class="error-icon">❌</div>
  <div class="error-content">
    {#if errorCode}
      <div class="error-code">{errorCode}</div>
    {/if}
    <div class="error-message">{errorInfo.simple}</div>
    <div class="error-action">
      <button class="doc-button" on:click={onDocClick}>
        [What is this?]
      </button>
    </div>
    <div class="dev-detail">
      {errorInfo.developer}
    </div>
  </div>
</div>
