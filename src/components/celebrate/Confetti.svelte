<script lang="ts">
  let { active = false, onDone }: { active?: boolean; onDone?: () => void } = $props();

  const PARTICLE_COUNT = 16;
  const COLORS = ["var(--accent)", "var(--accent-hi)", "var(--success)", "var(--info)"];
  const DURATION = 1200;

  let particles = $state<Array<{ id: number; x: number; y: number; rot: number; color: string; delay: number }>>([]);
  let nextId = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let reduced = $state(false);

  $effect(() => {
    if (typeof window !== "undefined") {
      reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    }
  });

  $effect(() => {
    if (!active) return;
    if (reduced) {
      timer = setTimeout(() => onDone?.(), 400);
      return () => {
        if (timer) clearTimeout(timer);
      };
    }
    const fresh: typeof particles = [];
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const angle = (Math.random() * 2 - 1) * 80;
      const distance = 80 + Math.random() * 80;
      const x = Math.sin((angle * Math.PI) / 180) * distance;
      const y = -Math.abs(Math.cos((angle * Math.PI) / 180)) * distance;
      fresh.push({
        id: nextId++,
        x,
        y,
        rot: (Math.random() - 0.5) * 720,
        color: COLORS[i % COLORS.length],
        delay: Math.random() * 80,
      });
    }
    particles = fresh;
    timer = setTimeout(() => {
      particles = [];
      onDone?.();
    }, DURATION);
    return () => {
      if (timer) clearTimeout(timer);
    };
  });
</script>

{#if active}
  <div class="confetti-root" aria-hidden="true">
    {#if reduced}
      <span class="reduced-pulse"></span>
    {:else}
      {#each particles as p (p.id)}
        <span
          class="particle"
          style="--tx: {p.x}px; --ty: {p.y}px; --rot: {p.rot}deg; --color: {p.color}; --delay: {p.delay}ms;"
        ></span>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .confetti-root {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 0;
    height: 0;
    pointer-events: none;
    z-index: 50;
  }

  .particle {
    position: absolute;
    width: 7px;
    height: 7px;
    background: var(--color);
    border-radius: 1px;
    transform: translate(-50%, -50%);
    animation: confetti 1200ms cubic-bezier(0.2, 0.8, 0.2, 1) var(--delay) forwards;
    opacity: 0;
  }

  @keyframes confetti {
    0% {
      transform: translate(-50%, -50%) rotate(0deg);
      opacity: 1;
    }
    100% {
      transform: translate(calc(-50% + var(--tx)), calc(-50% + var(--ty))) rotate(var(--rot));
      opacity: 0;
    }
  }

  .reduced-pulse {
    position: absolute;
    width: 32px;
    height: 32px;
    transform: translate(-50%, -50%);
    background: var(--accent);
    border-radius: var(--radius-full);
    opacity: 0.4;
    animation: reduced-bounce 400ms ease-out forwards;
  }

  @keyframes reduced-bounce {
    0% { transform: translate(-50%, -50%) scale(0.4); opacity: 0.6; }
    100% { transform: translate(-50%, -50%) scale(1.6); opacity: 0; }
  }
</style>
