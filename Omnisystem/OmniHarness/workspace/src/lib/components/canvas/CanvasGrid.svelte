<script lang="ts">
  export let spacing = 40;
</script>

<div
  class="grid"
  style="background-size: {spacing}px {spacing}px, {spacing}px {spacing}px;"
/>

<style>
  .grid {
    position: absolute;
    inset: -4000px;
    pointer-events: none;
    background-image:
      linear-gradient(to right, rgba(255, 255, 255, 0.045) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(255, 255, 255, 0.045) 1px, transparent 1px);
  }
</style>
