<script lang="ts">
  import { onMount } from 'svelte';

  let canvas: HTMLCanvasElement;

  class Particle {
    x: number; y: number;
    vx: number; vy: number;
    size: number; color: string;
    rot: number; rotV: number;
    life: number;

    constructor(w: number, h: number) {
      this.x  = Math.random() * w;
      this.y  = -20 - Math.random() * h * 0.5;
      this.vx = (Math.random() - 0.5) * 7;
      this.vy = Math.random() * 5 + 3;
      this.size = Math.random() * 9 + 5;
      this.color = `hsl(${Math.random() * 360},85%,60%)`;
      this.rot  = Math.random() * Math.PI * 2;
      this.rotV = (Math.random() - 0.5) * 0.2;
      this.life = 1;
    }

    update() {
      this.x   += this.vx;
      this.vy  += 0.18;
      this.y   += this.vy;
      this.rot += this.rotV;
      this.life -= 0.004;
    }

    draw(ctx: CanvasRenderingContext2D) {
      ctx.save();
      ctx.globalAlpha = Math.max(this.life, 0);
      ctx.fillStyle   = this.color;
      ctx.translate(this.x + this.size / 2, this.y + this.size / 2);
      ctx.rotate(this.rot);
      ctx.fillRect(-this.size / 2, -this.size / 2, this.size, this.size * 0.5);
      ctx.restore();
    }
  }

  onMount(() => {
    const ctx = canvas.getContext('2d')!;
    const w = canvas.width  = window.innerWidth;
    const h = canvas.height = window.innerHeight;

    let particles: Particle[] = [];
    for (let i = 0; i < 200; i++) particles.push(new Particle(w, h));

    let raf: number;
    const loop = () => {
      ctx.clearRect(0, 0, w, h);
      for (const p of particles) { p.update(); p.draw(ctx); }
      particles = particles.filter(p => p.life > 0 && p.y < h + 40);
      if (particles.length > 0) raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });
</script>

<canvas bind:this={canvas} class="confetti-canvas"></canvas>

<style>
  .confetti-canvas {
    position: fixed;
    top: 0; left: 0;
    width: 100vw; height: 100vh;
    z-index: 10000;
    pointer-events: none;
  }
</style>
