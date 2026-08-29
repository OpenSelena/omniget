<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import SettingsDrillBack from "$components/settings/SettingsDrillBack.svelte";
  import SettingsDrillItem from "$components/settings/SettingsDrillItem.svelte";
  import VoiceSettings from "$components/omnidisc/VoiceSettings.svelte";
  import ProfileSettings from "$components/omnidisc/settings/ProfileSettings.svelte";
  import SessionsSettings from "$components/omnidisc/settings/SessionsSettings.svelte";
  import DevicesSettings from "$components/omnidisc/settings/DevicesSettings.svelte";

  type Drill = "voice" | "profile" | "sessions" | "devices";

  function initial(): Drill | null {
    const view = page.url.searchParams.get("view");
    return view === "voice" || view === "profile" || view === "sessions" || view === "devices"
      ? view
      : null;
  }

  let subView = $state<Drill | null>(initial());
</script>

<div class="od-settings">
  <header class="bar">
    <h2 class="title">{$t("omnidisc.settings_title")}</h2>
  </header>
  <div class="body settings-content">
    {#if subView === null}
      <div class="settings-drill-list">
        <SettingsDrillItem
          title={$t("omnidisc.profile_settings.title")}
          hint={$t("omnidisc.profile_settings.hint")}
          onclick={() => (subView = "profile")}
        />
        <SettingsDrillItem
          title={$t("omnidisc.voice.title")}
          hint={$t("omnidisc.voice.settings_hint")}
          onclick={() => (subView = "voice")}
        />
        <SettingsDrillItem
          title={$t("omnidisc.sessions.title")}
          hint={$t("omnidisc.sessions.hint")}
          onclick={() => (subView = "sessions")}
        />
        <SettingsDrillItem
          title={$t("omnidisc.devices.title")}
          hint={$t("omnidisc.devices.hint")}
          onclick={() => (subView = "devices")}
        />
      </div>
    {:else if subView === "voice"}
      <SettingsDrillBack title={$t("omnidisc.voice.title")} hint={$t("omnidisc.voice.settings_hint")} onBack={() => (subView = null)} />
      <VoiceSettings />
    {:else if subView === "profile"}
      <SettingsDrillBack title={$t("omnidisc.profile_settings.title")} hint={$t("omnidisc.profile_settings.hint")} onBack={() => (subView = null)} />
      <ProfileSettings />
    {:else if subView === "sessions"}
      <SettingsDrillBack title={$t("omnidisc.sessions.title")} hint={$t("omnidisc.sessions.hint")} onBack={() => (subView = null)} />
      <SessionsSettings />
    {:else if subView === "devices"}
      <SettingsDrillBack title={$t("omnidisc.devices.title")} hint={$t("omnidisc.devices.hint")} onBack={() => (subView = null)} />
      <DevicesSettings />
    {/if}
  </div>
</div>

<style>
  .od-settings {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .bar {
    display: flex;
    align-items: center;
    min-height: 48px;
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }

  .title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4) var(--space-5);
    max-width: 760px;
  }
</style>
