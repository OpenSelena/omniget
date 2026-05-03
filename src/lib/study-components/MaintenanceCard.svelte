<script lang="ts">
  import { t } from "$lib/i18n";

  type Props = {
    feature: string;
    eta?: string;
    detail?: string;
  };

  let { feature, eta, detail }: Props = $props();
</script>

<div class="shell">
  <article class="card">
    <div class="icon" aria-hidden="true">🔧</div>
    <h1 class="title">{$t("study.maintenance.title", { feature })}</h1>
    <p class="lead">
      {$t("study.maintenance.lead")}
    </p>
    {#if detail}
      <p class="detail">{detail}</p>
    {/if}
    {#if eta}
      <p class="eta">{$t("study.maintenance.eta_label")}: <strong>{eta}</strong></p>
    {/if}
    <div class="actions">
      <a class="btn primary" href="/study">{$t("study.maintenance.back_to_study")}</a>
      <a class="btn ghost" href="/study/library">{$t("study.maintenance.open_library")}</a>
    </div>
  </article>
</div>

<style>
  .shell {
    min-height: calc(100vh - var(--header-height, 64px) - 80px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-5);
  }
  .card {
    width: min(560px, 100%);
    padding: 32px 28px;
    background: var(--surface, var(--bg));
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: 14px;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
  }
  .icon {
    font-size: 40px;
    line-height: 1;
    margin-bottom: 4px;
  }
  .title {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .lead {
    margin: 0;
    font-size: 14px;
    line-height: 1.55;
    color: var(--secondary, color-mix(in oklab, currentColor 75%, transparent));
    max-width: 440px;
  }
  .detail {
    margin: 0;
    padding: 10px 14px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-radius: 8px;
    font-size: 12px;
    color: var(--accent);
    line-height: 1.5;
    max-width: 440px;
  }
  .eta {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary, color-mix(in oklab, currentColor 55%, transparent));
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .eta strong {
    color: var(--text);
    font-weight: 600;
  }
  .actions {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    text-decoration: none;
    border: 1px solid transparent;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
  }
  .btn.primary:hover {
    filter: brightness(1.08);
  }
  .btn.ghost {
    background: transparent;
    border-color: var(--input-border);
    color: var(--text);
  }
  .btn.ghost:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
</style>
