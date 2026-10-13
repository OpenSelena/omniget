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
            <span class="badge-chip">
              <span class="badge-dot"></span>
              {item.badge}
            </span>
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
          <div class="topic-chips">
            {#each item.highlights as highlight}
              <span class="chip">{highlight}</span>
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
    padding: 14px 14px 16px 14px;
  }

  .banner-wrapper {
    position: relative;
    width: 100%;
    height: 180px;
    background: var(--surface-2, rgba(255, 255, 255, 0.05));
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
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

  .close-btn {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(18, 19, 22, 0.7);
    backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--text-muted, #9aa0a6);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .close-btn:hover {
    background: rgba(18, 19, 22, 0.95);
    color: #ffffff;
  }

  .header-no-thumb {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 4px 6px 0 6px;
  }

  .close-btn-inline {
    background: transparent;
    border: none;
    color: var(--text-muted, #9aa0a6);
    font-size: 14px;
    cursor: pointer;
    padding: 6px;
    border-radius: var(--radius-full, 9999px);
    transition: color 0.15s ease, background 0.15s ease;
  }

  .close-btn-inline:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.06);
  }

  .content-body {
    padding: 18px 8px 0 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .title {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--text, #f1f3f4);
    line-height: 1.25;
    letter-spacing: -0.01em;
  }

  .description {
    margin: 0;
    font-size: 14px;
    line-height: 1.55;
    color: var(--text-muted, #bdc1c6);
  }

  .topic-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }

  .chip {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.07);
    color: var(--text-muted, #a8aaad);
    font-size: 12px;
    font-weight: 400;
    padding: 4px 10px;
    border-radius: 8px;
  }

  .footer-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 8px;
  }

  .btn-dismiss {
    background: transparent;
    border: none;
    color: #9aa0a6;
    padding: 9px 18px;
    border-radius: var(--radius-full, 9999px);
    font-size: 13.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .btn-dismiss:hover {
    color: #e8eaed;
    background: rgba(255, 255, 255, 0.06);
  }

  .btn-cta {
    background: #8ab4f8;
    border: none;
    color: #041e49;
    padding: 9px 24px;
    border-radius: var(--radius-full, 9999px);
    font-size: 13.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease, opacity 0.15s ease;
  }

  .btn-cta:hover {
    background: #a8c7fa;
  }
</style>
