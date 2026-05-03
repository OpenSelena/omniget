<script lang="ts">
  type Props = {
    coverUrl: string | null;
    title: string;
    subtitle?: string | null;
    onTitleClick?: () => void;
  };

  let { coverUrl, title, subtitle, onTitleClick }: Props = $props();
</script>

{#if coverUrl}
  <div class="page-hero">
    <img class="cover" src={coverUrl} alt="" loading="lazy" />
    <div class="overlay">
      <button
        type="button"
        class="title-btn"
        onclick={onTitleClick}
        disabled={!onTitleClick}
      >
        <h1 class="title">{title}</h1>
      </button>
      {#if subtitle}
        <span class="subtitle">{subtitle}</span>
      {/if}
    </div>
  </div>
{/if}

<style>
  .page-hero {
    position: relative;
    width: 100%;
    height: 200px;
    border-radius: var(--border-radius);
    overflow: hidden;
    margin-bottom: var(--space-3);
    background: color-mix(in oklab, var(--input-border) 30%, transparent);
  }
  @media (max-width: 768px) {
    .page-hero {
      height: 80px;
    }
  }
  .cover {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: var(--space-4);
    background: linear-gradient(
      to top,
      color-mix(in oklab, black 75%, transparent) 0%,
      color-mix(in oklab, black 35%, transparent) 50%,
      transparent 100%
    );
  }
  .title-btn {
    align-self: flex-start;
    border: 0;
    background: transparent;
    cursor: pointer;
    padding: 0;
    text-align: left;
  }
  .title-btn:disabled {
    cursor: default;
  }
  .title-btn:hover:not(:disabled) .title {
    text-decoration: underline;
  }
  .title {
    margin: 0;
    color: white;
    font-size: 28px;
    font-weight: 700;
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.6);
    letter-spacing: -0.01em;
  }
  @media (max-width: 768px) {
    .title {
      font-size: 18px;
    }
  }
  .subtitle {
    margin-top: 4px;
    color: rgba(255, 255, 255, 0.85);
    font-size: 12px;
    font-family: var(--font-mono, ui-monospace, monospace);
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
  }
</style>
