import AndroidUsbLab from './lib/components/AndroidUsbLab.svelte';

const app = new AndroidUsbLab({
  target: document.getElementById('app')!,
});

export default app;
