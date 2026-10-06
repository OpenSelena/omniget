<script lang="ts">
  import { open } from "@tauri-apps/plugin-shell";
  import { getVersion } from "@tauri-apps/api/app";
  import { t } from "$lib/i18n";

  const GITHUB_URL = "https://github.com/OpenSelena/omniget";
  let appVersion = $state("");

  $effect(() => {
    getVersion().then((v) => { appVersion = v; }).catch(() => {});
  });

  async function openGitHub() {
    await open(GITHUB_URL);
  }
</script>

<section class="star-section">
  <p class="star-description">{$t('about.star_description')}</p>
  <div class="star-iframe-wrapper">
    <iframe
      src="https://ghbtns.com/github-btn.html?user=OpenSelena&repo=omniget&type=star&count=true&size=large"
      frameborder="0"
      scrolling="0"
      width="170"
      height="30"
      title="GitHub Stars"
    ></iframe>
  </div>
</section>

<section class="project-header">
  <img src="/loop.png" alt="Loop" class="project-logo" width="64" height="64" />
  <p class="project-description">{$t('about.description')}</p>
</section>

<section class="card">
  <h5 class="card-title">{$t('about.features_title')}</h5>
  <div class="features-list">
    <p class="feature-item">{$t('about.feature_platforms')}</p>
    <p class="feature-item">{$t('about.feature_hotmart')}</p>
    <p class="feature-item">{$t('about.feature_progress')}</p>
    <p class="feature-item">{$t('about.feature_mascot')}</p>
    <p class="feature-item">{$t('about.feature_themes')}</p>
    <p class="feature-item">{$t('about.feature_i18n')}</p>
    <p class="feature-item">{$t('about.feature_tech')}</p>
  </div>
</section>

{#if appVersion}
  <p class="version">{$t('about.version')} {appVersion}</p>
{/if}

<style>
  .star-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--padding);
    width: 100%;
  }

  .star-description {
    font-size: var(--text-base);
    font-weight: 500;
    color: var(--secondary);
  }

  .star-iframe-wrapper {
    display: flex;
    justify-content: center;
  }

  .star-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: calc(var(--padding) / 2);
    margin: 0 auto;
    padding: calc(var(--padding) / 2) calc(var(--padding) * 1.5);
    font-size: var(--text-base);
    font-weight: 500;
    color: var(--text);
    background: var(--surface);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .star-button svg {
    pointer-events: none;
    flex-shrink: 0;
    color: var(--warning);
  }

  @media (hover: hover) {
    .star-button:hover {
      background: var(--surface-hi);
    }
  }

  .star-button:active {
    background: var(--fill-2);
  }

  .star-button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .project-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) / 2);
    text-align: center;
  }

  .project-logo {
    border-radius: calc(var(--border-radius) + 4px);
    pointer-events: none;
  }

  .project-description {
    font-size: var(--text-base);
    font-weight: 400;
    line-height: 1.8;
    color: var(--gray);
    max-width: 320px;
  }

  .card {
    width: 100%;
    background: var(--button);
    box-shadow: var(--button-box-shadow);
    border-radius: var(--border-radius);
    padding: calc(var(--padding) + 4px);
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) / 2);
  }

  .card-title {
    color: var(--secondary);
  }

  .features-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .feature-item {
    font-size: 13px;
    font-weight: 400;
    line-height: 1.6;
    color: var(--gray);
  }

  .feature-item::before {
    content: "• ";
    color: var(--accent);
  }

  .version {
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--gray);
    text-align: center;
  }
</style>
