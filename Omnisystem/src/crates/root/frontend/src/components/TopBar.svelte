<script>
  export let onTimeline = () => {};
  export let onHealth = () => {};
  export let onSettings = () => {};
</script>

<header class="topbar">
  <div class="brand">Bonsai Root</div>
  <div class="actions">
    <button on:click={onTimeline} title="Timeline">🕐</button>
    <button on:click={onHealth} title="Health">🛡️</button>
    <button on:click={onSettings} title="Settings">⚙️</button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border: 1px solid #233042;
    border-radius: 0.75rem;
    background: linear-gradient(90deg, #0f1622, #131c29);
  }

  .brand {
    font-weight: 700;
    letter-spacing: 0.2px;
  }

  .actions button {
    margin-left: 0.5rem;
    background: #1b2636;
    border: 1px solid #2a3a52;
    color: #e6eef9;
    border-radius: 0.5rem;
    width: 2.2rem;
    height: 2.2rem;
    cursor: pointer;
  }
</style>
