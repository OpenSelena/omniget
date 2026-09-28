<script lang="ts">
  import DialogContainer from "./DialogContainer.svelte";
  import {
    getAnnouncementOpen,
    getActiveAnnouncement,
    dismissAnnouncement,
  } from "$lib/stores/announcement-store.svelte";
  import { open as openExternal } from "@tauri-apps/plugin-shell";

  let isOpen = $derived(getAnnouncementOpen());
  let item = $derived(getActiveAnnouncement());

  async function handleCTA() {
    if (item?.cta?.url) {
      try {
        await openExternal(item.cta.url);
      } catch {
        window.open(item.cta.url, "_blank");
      }
    }
    dismissAnnouncement(true);
  }

  function handleDismiss() {
    dismissAnnouncement(true);
  }
</script>

{#if item}
  <DialogContainer
    isOpen={isOpen}
    onClose={handleDismiss}
    titleId="announcement-modal-title"
  >
    <div class="announcement-card">
      {#if item.thumbnail}
        <div class="banner-wrapper">
          <img
            src={item.thumbnail}
            alt={item.title}
            class="banner-img"
            loading="lazy"
          />
          {#if item.badge}
            <span class="badge-pill">{item.badge}</span>
          {/if}
          <button
            type="button"
            class="close-btn"
            aria-label="Close announcement"
            onclick={handleDismiss}
          >
            ✕
          </button>
        </div>
      {:else}
        <div class="header-no-thumb">
          {#if item.badge}
            <span class="badge-pill-inline">{item.badge}</span>
          {/if}
          <button
            type="button"
            class="close-btn-inline"
            aria-label="Close announcement"
            onclick={handleDismiss}
          >
            ✕
          </button>
        </div>
      {/if}

      <div class="content-body">
        <h2 id="announcement-modal-title" class="title">{item.title}</h2>
        <p class="description">{item.body}</p>

        {#if item.highlights && item.highlights.length > 0}
          <div class="highlights-box">
            {#each item.highlights as highlight}
              <div class="highlight-item">
                <span class="highlight-bullet">•</span>
                <span class="highlight-text">{highlight}</span>
              </div>
            {/each}
          </div>
        {/if}

        <div class="footer-actions">
          <button
            type="button"
            class="btn-dismiss"
            onclick={handleDismiss}
          >
            Dismiss
          </button>

          {#if item.cta}
            <button
              type="button"
              class="btn-cta"
              onclick={handleCTA}
            >
              {item.cta.label}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </DialogContainer>
{/if}

<style>
  .announcement-card {
    display: flex;
    flex-direction: column;
    width: 100%;
    overflow: hidden;
  }

  .banner-wrapper {
    position: relative;
    width: 100%;
    height: 180px;
    background: var(--surface-2, rgba(255, 255, 255, 0.05));
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .banner-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .badge-pill {
    position: absolute;
    top: 12px;
    left: 12px;
    background: var(--accent, #6366f1);
    color: #ffffff;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 3px 10px;
    border-radius: var(--radius-full, 9999px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  }

  .close-btn {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(0, 0, 0, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .close-btn:hover {
    background: rgba(0, 0, 0, 0.8);
  }

  .header-no-thumb {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 0 20px;
  }

  .badge-pill-inline {
    background: var(--accent, #6366f1);
    color: #ffffff;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 3px 10px;
    border-radius: var(--radius-full, 9999px);
  }

  .close-btn-inline {
    background: transparent;
    border: none;
    color: var(--text-muted, #94a3b8);
    font-size: 14px;
    cursor: pointer;
    padding: 4px;
  }

  .close-btn-inline:hover {
    color: var(--text, #ffffff);
  }

  .content-body {
    padding: 18px 22px 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    color: var(--text, #ffffff);
    line-height: 1.3;
  }

  .description {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.5;
    color: var(--text-muted, #94a3b8);
  }

  .highlights-box {
    background: var(--surface-1, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--border, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-md, 8px);
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .highlight-item {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 13px;
    color: var(--text, #e2e8f0);
  }

  .highlight-bullet {
    color: var(--accent, #6366f1);
    font-size: 14px;
    font-weight: bold;
  }

  .footer-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }

  .btn-dismiss {
    background: var(--surface-2, rgba(255, 255, 255, 0.08));
    border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
    color: var(--text, #ffffff);
    padding: 8px 16px;
    border-radius: var(--radius-md, 8px);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .btn-dismiss:hover {
    background: var(--surface-3, rgba(255, 255, 255, 0.12));
  }

  .btn-cta {
    background: var(--accent, #6366f1);
    border: none;
    color: #ffffff;
    padding: 8px 18px;
    border-radius: var(--radius-md, 8px);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s ease, opacity 0.15s ease;
    box-shadow: 0 2px 6px rgba(99, 102, 241, 0.3);
  }

  .btn-cta:hover {
    filter: brightness(1.1);
  }
</style>
