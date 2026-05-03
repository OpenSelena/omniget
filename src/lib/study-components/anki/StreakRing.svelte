<script lang="ts">
  let {
    current,
    longest,
    reviewedToday,
    goal = 50,
  } = $props<{
    current: number;
    longest: number;
    reviewedToday: number;
    goal?: number;
  }>();

  const radius = 52;
  const circumference = 2 * Math.PI * radius;

  const ratio = $derived(Math.min(1, reviewedToday / Math.max(goal, 1)));
  const dashoffset = $derived(circumference * (1 - ratio));
  const flame = $derived(current >= 30 ? "🔥🔥🔥" : current >= 7 ? "🔥🔥" : current >= 3 ? "🔥" : "✨");
</script>

<div class="streak">
  <svg viewBox="0 0 120 120" width="120" height="120" class="streak-ring" aria-hidden="true">
    <circle cx="60" cy="60" r={radius} fill="none" stroke="var(--surface-mut)" stroke-width="8" />
    <circle
      cx="60"
      cy="60"
      r={radius}
      fill="none"
      stroke="var(--accent)"
      stroke-width="8"
      stroke-linecap="round"
      stroke-dasharray={circumference}
      stroke-dashoffset={dashoffset}
      transform="rotate(-90 60 60)"
      class="streak-progress"
    />
  </svg>
  <div class="streak-center">
    <span class="streak-flame">{flame}</span>
    <span class="streak-current">{current}</span>
    <span class="streak-label">{current === 1 ? "dia" : "dias"}</span>
  </div>
</div>

<div class="streak-stats">
  <span><strong>{reviewedToday}</strong> hoje</span>
  <span class="dot">·</span>
  <span><strong>{longest}</strong> recorde</span>
</div>

<style>
  .streak {
    position: relative;
    width: 120px;
    height: 120px;
    margin-inline: auto;
  }

  .streak-ring {
    position: absolute;
    inset: 0;
  }

  .streak-progress {
    transition: stroke-dashoffset var(--duration-base) var(--ease-out);
  }

  .streak-center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
  }

  .streak-flame {
    font-size: 18px;
    line-height: 1;
  }

  .streak-current {
    font-size: 28px;
    font-weight: 700;
    line-height: 1;
    color: var(--accent);
  }

  .streak-label {
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .streak-stats {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .streak-stats strong {
    color: var(--text);
    font-weight: 600;
  }

  .dot {
    opacity: 0.5;
  }
</style>
