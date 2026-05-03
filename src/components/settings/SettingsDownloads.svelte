<script lang="ts">
  import { t } from "$lib/i18n";
  import ContextHint from "$components/hints/ContextHint.svelte";
  import {
    getSettings,
    updateSettings,
    chooseFolder,
    chooseCookieFile,
    toggleBool,
    changeQuality,
  } from "./settings-helpers";

  let settings = $derived(getSettings());

  let templateInput = $state("");
  let templateTimer: ReturnType<typeof setTimeout> | null = null;
  let hotkeyInput = $state("");
  let hotkeyTimer: ReturnType<typeof setTimeout> | null = null;
  let hotkeyMode = $state<"record" | "type">("record");
  let hotkeyRecording = $state(false);

  $effect(() => {
    if (settings) {
      templateInput = settings.download.filename_template;
      hotkeyInput = settings.download.hotkey_binding;
    }
  });

  function previewTemplate(template: string): string {
    return template
      .replace("%(title).200s", "My Video Title")
      .replace("%(title)s", "My Video Title")
      .replace("%(id)s", "dQw4w9WgXcQ")
      .replace("%(ext)s", "mp4")
      .replace("%(uploader)s", "Channel Name")
      .replace("%(upload_date)s", "20260217")
      .replace("%(resolution)s", "1920x1080")
      .replace("%(fps)s", "30")
      .replace("%(duration)s", "212");
  }

  function handleTemplateInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    templateInput = value;
    if (templateTimer) clearTimeout(templateTimer);
    templateTimer = setTimeout(async () => {
      if (value.trim() && value.includes("%(ext)s")) {
        await updateSettings({ download: { filename_template: value } });
      }
    }, 800);
  }

  function handleHotkeyInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    hotkeyInput = value;
    if (hotkeyTimer) clearTimeout(hotkeyTimer);
    hotkeyTimer = setTimeout(async () => {
      if (value.trim()) {
        await updateSettings({ download: { hotkey_binding: value } });
      }
    }, 800);
  }

  function mapKeyName(key: string): string | null {
    if (key.length === 1 && /[a-zA-Z]/.test(key)) return key.toUpperCase();
    if (key.length === 1 && /[0-9]/.test(key)) return key;
    if (/^F([1-9]|1[0-2])$/.test(key)) return key;
    const map: Record<string, string> = {
      " ": "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
      Enter: "Enter", Tab: "Tab", Escape: "Escape", Backspace: "Backspace", Delete: "Delete",
      Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown", Insert: "Insert",
    };
    return map[key] ?? null;
  }

  function handleHotkeyKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    const keyName = mapKeyName(e.key);
    if (!keyName) return;
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push("CmdOrCtrl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    parts.push(keyName);
    const value = parts.join("+");
    hotkeyInput = value;
    hotkeyRecording = false;
    updateSettings({ download: { hotkey_binding: value } });
  }
</script>

{#if settings}
<section class="section">
  <h5 class="section-title">{$t('settings.download.hotkey_enabled')}</h5>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.hotkey_enabled')} <ContextHint text={$t('hints.hotkey') as string} dismissKey="hotkey" /></span>
        <span class="setting-path">{$t('settings.download.hotkey_enabled_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.hotkey_enabled} onclick={() => toggleBool("download", "hotkey_enabled", settings.download.hotkey_enabled)} role="switch" aria-checked={settings.download.hotkey_enabled} aria-label={$t('settings.download.hotkey_enabled') as string}><span class="toggle-knob"></span></button>
    </div>
    {#if settings.download.hotkey_enabled}
      <div class="divider"></div>
      <div class="setting-row hotkey-row">
        <span class="setting-label">{$t('settings.download.hotkey_binding')}</span>
        <div class="hotkey-controls">
          <div class="hotkey-mode-switch">
            <button class="hotkey-mode-btn" class:active={hotkeyMode === 'record'} onclick={() => { hotkeyMode = 'record'; hotkeyRecording = false; }}>{$t('settings.download.hotkey_record')}</button>
            <button class="hotkey-mode-btn" class:active={hotkeyMode === 'type'} onclick={() => { hotkeyMode = 'type'; hotkeyRecording = false; }}>{$t('settings.download.hotkey_type')}</button>
          </div>
          {#if hotkeyMode === 'type'}
            <input type="text" class="input-hotkey" value={hotkeyInput} oninput={handleHotkeyInput} spellcheck="false" />
          {:else}
            <button class="input-hotkey hotkey-record-btn" class:recording={hotkeyRecording} onclick={() => { hotkeyRecording = true; }} onkeydown={hotkeyRecording ? handleHotkeyKeyDown : undefined} onblur={() => { hotkeyRecording = false; }}>
              {hotkeyRecording ? $t('settings.download.hotkey_press') : (hotkeyInput || $t('settings.download.hotkey_press'))}
            </button>
          {/if}
        </div>
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.copy_to_clipboard_on_hotkey')}</span>
          <span class="setting-path">{$t('settings.download.copy_to_clipboard_on_hotkey_desc')}</span>
        </div>
        <button class="toggle" class:on={settings.download.copy_to_clipboard_on_hotkey} onclick={() => toggleBool("download", "copy_to_clipboard_on_hotkey", settings.download.copy_to_clipboard_on_hotkey)} role="switch" aria-checked={settings.download.copy_to_clipboard_on_hotkey} aria-label={$t('settings.download.copy_to_clipboard_on_hotkey') as string}><span class="toggle-knob"></span></button>
      </div>
    {/if}
  </div>
</section>

<section class="section">
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.default_output_dir')}</span>
        <span class="setting-path">{settings.download.default_output_dir}</span>
      </div>
      <button class="button" onclick={chooseFolder}>{$t('settings.download.choose_folder')}</button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('common.cookie_file_label')}</span>
        <span class="setting-path">{settings.download.cookie_file || $t('common.cookie_file_hint')}</span>
      </div>
      <button class="button" onclick={chooseCookieFile}>{$t('common.cookie_file_choose')}</button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.download.video_quality')}</span>
      <select class="select" value={settings.download.video_quality} onchange={changeQuality}>
        <option value="best">{$t('omnibox.quality_best')}</option>
        <option value="1080p">{$t('omnibox.quality_1080p')}</option>
        <option value="720p">{$t('omnibox.quality_720p')}</option>
        <option value="480p">{$t('omnibox.quality_480p')}</option>
        <option value="360p">{$t('omnibox.quality_360p')}</option>
      </select>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.download.always_ask_path')}</span>
      <button class="toggle" class:on={settings.download.always_ask_path} onclick={() => toggleBool("download", "always_ask_path", settings.download.always_ask_path)} role="switch" aria-checked={settings.download.always_ask_path} aria-label={$t('settings.download.always_ask_path') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</section>

<details class="section">
  <summary class="section-title">{$t('settings.download.organize_by_platform')}</summary>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.organize_by_platform')}</span>
        <span class="setting-path">{$t('settings.download.organize_by_platform_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.organize_by_platform} onclick={() => toggleBool("download", "organize_by_platform", settings.download.organize_by_platform)} role="switch" aria-checked={settings.download.organize_by_platform} aria-label={$t('settings.download.organize_by_platform') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.download.skip_existing')}</span>
      <button class="toggle" class:on={settings.download.skip_existing} onclick={() => toggleBool("download", "skip_existing", settings.download.skip_existing)} role="switch" aria-checked={settings.download.skip_existing} aria-label={$t('settings.download.skip_existing') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</details>

<details class="section">
  <summary class="section-title">{$t('settings.download.what_to_also_save')}</summary>
  <div class="card">
    <div class="setting-row">
      <span class="setting-label">{$t('settings.download.download_attachments')}</span>
      <button class="toggle" class:on={settings.download.download_attachments} onclick={() => toggleBool("download", "download_attachments", settings.download.download_attachments)} role="switch" aria-checked={settings.download.download_attachments} aria-label={$t('settings.download.download_attachments') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.download.download_descriptions')}</span>
      <button class="toggle" class:on={settings.download.download_descriptions} onclick={() => toggleBool("download", "download_descriptions", settings.download.download_descriptions)} role="switch" aria-checked={settings.download.download_descriptions} aria-label={$t('settings.download.download_descriptions') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.download_subtitles')}</span>
        <span class="setting-path">{$t('settings.download.download_subtitles_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.download_subtitles} onclick={() => toggleBool("download", "download_subtitles", settings.download.download_subtitles)} role="switch" aria-checked={settings.download.download_subtitles} aria-label={$t('settings.download.download_subtitles') as string}><span class="toggle-knob"></span></button>
    </div>
    {#if settings.download.download_subtitles}
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.include_auto_subtitles')}</span>
          <span class="setting-path">{$t('settings.download.include_auto_subtitles_desc')}</span>
        </div>
        <button class="toggle" class:on={settings.download.include_auto_subtitles} onclick={() => toggleBool("download", "include_auto_subtitles", settings.download.include_auto_subtitles)} role="switch" aria-checked={settings.download.include_auto_subtitles} aria-label={$t('settings.download.include_auto_subtitles') as string}><span class="toggle-knob"></span></button>
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.translate_metadata')}</span>
        <span class="setting-path">{$t('settings.download.translate_metadata_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.translate_metadata} onclick={() => toggleBool("download", "translate_metadata", settings.download.translate_metadata)} role="switch" aria-checked={settings.download.translate_metadata} aria-label={$t('settings.download.translate_metadata') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.embed_metadata')}</span>
        <span class="setting-path">{$t('settings.download.embed_metadata_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.embed_metadata} onclick={() => toggleBool("download", "embed_metadata", settings.download.embed_metadata)} role="switch" aria-checked={settings.download.embed_metadata} aria-label={$t('settings.download.embed_metadata') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.embed_thumbnail')}</span>
        <span class="setting-path">{$t('settings.download.embed_thumbnail_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.embed_thumbnail} onclick={() => toggleBool("download", "embed_thumbnail", settings.download.embed_thumbnail)} role="switch" aria-checked={settings.download.embed_thumbnail} aria-label={$t('settings.download.embed_thumbnail') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</details>

<details class="section">
  <summary class="section-title">{$t('settings.download.youtube_specific')}</summary>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.youtube_sponsorblock')}</span>
        <span class="setting-path">{$t('settings.download.youtube_sponsorblock_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.youtube_sponsorblock} onclick={() => toggleBool("download", "youtube_sponsorblock", settings.download.youtube_sponsorblock)} role="switch" aria-checked={settings.download.youtube_sponsorblock} aria-label={$t('settings.download.youtube_sponsorblock') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.split_by_chapters')}</span>
        <span class="setting-path">{$t('settings.download.split_by_chapters_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.split_by_chapters} onclick={() => toggleBool("download", "split_by_chapters", settings.download.split_by_chapters)} role="switch" aria-checked={settings.download.split_by_chapters} aria-label={$t('settings.download.split_by_chapters') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</details>

<details class="section">
  <summary class="section-title">{$t('settings.download.clipboard_detection')}</summary>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.clipboard_detection')} <ContextHint text={$t('hints.clipboard') as string} dismissKey="clipboard" /></span>
        <span class="setting-path">{$t('settings.download.clipboard_detection_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.clipboard_detection} onclick={() => toggleBool("download", "clipboard_detection", settings.download.clipboard_detection)} role="switch" aria-checked={settings.download.clipboard_detection} aria-label={$t('settings.download.clipboard_detection') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</details>

<details class="section">
  <summary class="section-title">{$t('settings.download.filename_template')}</summary>
  <div class="card">
    <div class="setting-row template-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.filename_template')}</span>
        <span class="setting-path">{$t('settings.download.filename_template_desc')}</span>
      </div>
      <input type="text" class="input-template" value={templateInput} oninput={handleTemplateInput} spellcheck="false" />
    </div>
    {#if templateInput}
      <div class="template-preview">
        <span class="setting-path">{$t('settings.download.filename_template_preview', { preview: previewTemplate(templateInput) })}</span>
      </div>
    {/if}
  </div>
</details>
{/if}
