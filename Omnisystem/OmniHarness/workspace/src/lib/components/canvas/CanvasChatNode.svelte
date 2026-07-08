<script lang="ts">
  export let content = '';
</script>

<div class="chat-node">{content}</div>

<style>
  .chat-node {
    width: 100%;
    height: 100%;
    overflow: auto;
    padding: 12px;
    color: var(--text);
    font-size: 12px;
    white-space: pre-wrap;
    line-height: 1.5;
    background: rgba(22, 163, 74, 0.08);
  }
</style>
