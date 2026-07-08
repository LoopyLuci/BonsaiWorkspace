<script lang="ts">
  export let label = 'OmniAI is thinking…';
</script>

<div class="typing" role="status" aria-label={label}>
  <span class="dot"></span>
  <span class="dot"></span>
  <span class="dot"></span>
</div>

<style>
  .typing {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 10px 14px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 18px 18px 18px 4px;
    width: fit-content;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-secondary);
    animation: bounce 1.2s ease-in-out infinite;
  }
  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }
  @keyframes bounce {
    0%, 60%, 100% { transform: translateY(0);    opacity: 0.5; }
    30%            { transform: translateY(-5px); opacity: 1;   }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot { animation: none; opacity: 0.7; }
  }
</style>
