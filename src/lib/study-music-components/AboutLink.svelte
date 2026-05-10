<script lang="ts">
  import { t } from "$lib/i18n";

  type Props = {
    variant?: "footer" | "inline" | "card";
    text?: string;
  };

  let { variant = "inline", text }: Props = $props();

  const url = "https://github.com/tonhowtf/omniget";

  async function open() {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch {
      try {
        window.open(url, "_blank");
      } catch {
        /* ignore */
      }
    }
  }

  const label = $derived(text ?? ($t("study.about.short_pitch") as string));
</script>

{#if variant === "card"}
  <button type="button" class="about-card" onclick={open}>
    <span class="about-eyebrow">{$t("study.about.eyebrow")}</span>
    <span class="about-headline">{$t("study.about.long_pitch")}</span>
    <span class="about-cta">
      {$t("study.about.cta_github")}
      <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="7" y1="17" x2="17" y2="7"/>
        <polyline points="7 7 17 7 17 17"/>
      </svg>
    </span>
  </button>
{:else if variant === "footer"}
  <button type="button" class="about-footer" onclick={open} title={url}>
    <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
      <path d="M12 2C6.48 2 2 6.58 2 12.25c0 4.51 2.87 8.34 6.84 9.69.5.1.68-.22.68-.49 0-.24-.01-.88-.01-1.73-2.78.62-3.37-1.36-3.37-1.36-.45-1.18-1.11-1.49-1.11-1.49-.91-.64.07-.62.07-.62 1 .07 1.53 1.05 1.53 1.05.89 1.56 2.34 1.11 2.91.85.09-.66.35-1.11.63-1.37-2.22-.26-4.55-1.14-4.55-5.07 0-1.12.39-2.03 1.03-2.75-.1-.26-.45-1.31.1-2.72 0 0 .84-.27 2.75 1.05A9.4 9.4 0 0 1 12 7.07c.85.01 1.71.12 2.51.34 1.91-1.32 2.75-1.05 2.75-1.05.55 1.41.2 2.46.1 2.72.64.72 1.03 1.63 1.03 2.75 0 3.94-2.34 4.81-4.57 5.06.36.32.68.94.68 1.9 0 1.37-.01 2.48-.01 2.81 0 .27.18.6.69.49A10.04 10.04 0 0 0 22 12.25C22 6.58 17.52 2 12 2z"/>
    </svg>
    <span>{label}</span>
  </button>
{:else}
  <button type="button" class="about-inline" onclick={open}>
    <span>{label}</span>
    <svg viewBox="0 0 24 24" width="11" height="11" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <line x1="7" y1="17" x2="17" y2="7"/>
      <polyline points="7 7 17 7 17 17"/>
    </svg>
  </button>
{/if}

<style>
  .about-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    padding: 18px 20px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .about-card:hover {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 6%, rgba(255, 255, 255, 0.03));
  }
  .about-eyebrow {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--accent);
  }
  .about-headline {
    font-size: 14px;
    font-weight: 600;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.9);
  }
  .about-cta {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }
  .about-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 14px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.45);
    font-family: inherit;
    font-size: 11px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: color 120ms ease, background 120ms ease;
  }
  .about-footer:hover {
    color: rgba(255, 255, 255, 0.95);
    background: color-mix(in oklab, white 6%, transparent);
  }
  .about-inline {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.5);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    padding: 2px 4px;
  }
  .about-inline:hover {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
</style>
