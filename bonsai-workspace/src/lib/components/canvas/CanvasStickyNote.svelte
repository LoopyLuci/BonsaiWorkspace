<script lang="ts">
  export let text = '';
  export let onChange: (value: string) => void;
</script>

<textarea bind:value={text} on:input={() => onChange(text)} placeholder="Write a note..." />

<style>
  textarea {
    width: 100%;
    height: 100%;
    resize: none;
    border: none;
    outline: none;
    padding: 12px;
    color: var(--text);
    background: rgba(255, 255, 210, 0.14);
    font-size: 13px;
    line-height: 1.45;
  }
</style>
