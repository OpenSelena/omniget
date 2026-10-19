<script lang="ts">
  import { page } from "$app/stores";
  import { t } from "$lib/i18n";
</script>

<div class="error-stage">
  <div class="error-card">
    <div class="error-badge">{$page.status}</div>
    <h1 class="error-title">
      {$page.status === 404 ? ($t("common.not_found") || "Page Not Found") : ($t("common.error") || "An error occurred")}
    </h1>
    <p class="error-msg">{$page.error?.message || "Unexpected application error"}</p>
    <div class="error-actions">
      <a href="/" class="btn-home">{$t("common.home") || "Back to Safety"}</a>
    </div>
  </div>
</div>

<style>
  .error-stage {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: var(--bg, #0b0c0e);
    color: var(--text, #e2e8f0);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
  .error-card {
    max-width: 460px;
    width: 100%;
    padding: 36px 32px;
    background: var(--surface, #14171c);
    border: 1px solid var(--border, rgba(255, 255, 255, 0.08));
    border-radius: 16px;
    text-align: center;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.4);
  }
  .error-badge {
    display: inline-block;
    padding: 4px 14px;
    border-radius: 999px;
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
    font-size: 14px;
    font-weight: 700;
    margin-bottom: 16px;
    letter-spacing: 0.05em;
  }
  .error-title {
    margin: 0 0 12px;
    font-size: 22px;
    font-weight: 650;
    color: var(--text-heading, #f8fafc);
  }
  .error-msg {
    margin: 0 0 24px;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text-dim, #94a3b8);
    word-break: break-word;
  }
  .error-actions {
    display: flex;
    justify-content: center;
  }
  .btn-home {
    padding: 10px 22px;
    border-radius: 10px;
    background: var(--accent, #6366f1);
    color: #fff;
    text-decoration: none;
    font-size: 13px;
    font-weight: 600;
    transition: filter 0.15s ease, transform 0.15s ease;
  }
  .btn-home:hover {
    filter: brightness(1.1);
    transform: translateY(-1px);
  }
</style>
