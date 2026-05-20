<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
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
  import { YTDLP_PRESETS, matchActivePreset, type YtdlpPresetId } from "$lib/ytdlp-presets";

  let settings = $derived(getSettings());
  let activePreset = $derived<YtdlpPresetId | null>(matchActivePreset(settings));

  type PathLimitInfo = { limit: number; current: number; reserve: number; ok: boolean };
  let pathInfo = $state<PathLimitInfo | null>(null);

  $effect(() => {
    const dir = settings?.download.default_output_dir;
    if (!dir) {
      pathInfo = null;
      return;
    }
    let cancelled = false;
    invoke<PathLimitInfo>("validate_output_path", { outputDir: dir })
      .then((info) => {
        if (!cancelled) pathInfo = info;
      })
      .catch(() => {
        if (!cancelled) pathInfo = null;
      });
    return () => {
      cancelled = true;
    };
  });

  async function applyPreset(id: YtdlpPresetId) {
    const preset = YTDLP_PRESETS.find((p) => p.id === id);
    if (!preset) return;
    await updateSettings({ download: preset.download });
  }

  let templateInput = $state("");
  let templateTimer: ReturnType<typeof setTimeout> | null = null;
  let hotkeyInput = $state("");
  let hotkeyTimer: ReturnType<typeof setTimeout> | null = null;
  let hotkeyMode = $state<"record" | "type">("record");
  let hotkeyRecording = $state(false);
  let musicHotkeyInput = $state("");
  let musicHotkeyTimer: ReturnType<typeof setTimeout> | null = null;
  let musicHotkeyMode = $state<"record" | "type">("record");
  let musicHotkeyRecording = $state(false);

  $effect(() => {
    if (settings) {
      templateInput = settings.download.filename_template;
      hotkeyInput = settings.download.hotkey_binding;
      musicHotkeyInput = settings.download.music_hotkey_binding;
    }
  });

  let speedNum = $state<number | null>(null);
  let speedUnit = $state<"K" | "M">("M");

  $effect(() => {
    const raw = settings?.download.speed_limit?.trim() ?? "";
    const m = raw.match(/^(\d+(?:\.\d+)?)([KM])?$/i);
    if (m) {
      speedNum = Number(m[1]);
      speedUnit = (m[2]?.toUpperCase() as "K" | "M") ?? "M";
    } else {
      speedNum = null;
    }
  });

  function applySpeedLimit() {
    const value = speedNum && speedNum > 0 ? `${speedNum}${speedUnit}` : "";
    updateSettings({ download: { speed_limit: value } });
  }

  const SB_CATEGORIES = [
    "sponsor",
    "selfpromo",
    "interaction",
    "intro",
    "outro",
    "preview",
    "filler",
    "music_offtopic",
  ] as const;

  function sbHas(cat: string): boolean {
    return settings?.download.sponsorblock_categories?.includes(cat) ?? false;
  }

  function toggleSbCategory(cat: string) {
    const current = settings?.download.sponsorblock_categories ?? [];
    const next = current.includes(cat)
      ? current.filter((c) => c !== cat)
      : [...current, cat];
    updateSettings({ download: { sponsorblock_categories: next } });
  }

  function setSbMode(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    updateSettings({ download: { sponsorblock_mode: value } });
  }

  function setBilibiliDanmakuFormat(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    updateSettings({ download: { bilibili_danmaku_format: value } });
  }

  function setBilibiliContainer(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    updateSettings({ download: { bilibili_container: value } });
  }

  function setBilibiliCoverFormat(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    updateSettings({ download: { bilibili_cover_format: value } });
  }

  let namingTemplatesOpen = $state(false);
  let cdnOpen = $state(false);

  async function setBilibiliNamingTemplate(field: string, value: string) {
    await updateSettings({ download: { [field]: value } });
  }

  async function setBilibiliCdnHosts(e: Event) {
    const value = (e.target as HTMLTextAreaElement).value;
    await updateSettings({ download: { bilibili_cdn_hosts: value } });
  }

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

  function handleMusicHotkeyInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    musicHotkeyInput = value;
    if (musicHotkeyTimer) clearTimeout(musicHotkeyTimer);
    musicHotkeyTimer = setTimeout(async () => {
      if (value.trim()) {
        await updateSettings({ download: { music_hotkey_binding: value } });
      }
    }, 800);
  }

  function handleMusicHotkeyKeyDown(e: KeyboardEvent) {
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
    musicHotkeyInput = value;
    musicHotkeyRecording = false;
    updateSettings({ download: { music_hotkey_binding: value } });
  }

  function changeMusicAudioFormat(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    updateSettings({ download: { music_audio_format: value } });
  }

  function changeCaptionLocale(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    updateSettings({ download: { caption_locale: value } });
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
  <h5 class="section-title">{$t('settings.download.music_hotkey_enabled')}</h5>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.music_hotkey_enabled')}</span>
        <span class="setting-path">{$t('settings.download.music_hotkey_enabled_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.music_hotkey_enabled} onclick={() => toggleBool("download", "music_hotkey_enabled", settings.download.music_hotkey_enabled)} role="switch" aria-checked={settings.download.music_hotkey_enabled} aria-label={$t('settings.download.music_hotkey_enabled') as string}><span class="toggle-knob"></span></button>
    </div>
    {#if settings.download.music_hotkey_enabled}
      <div class="divider"></div>
      <div class="setting-row hotkey-row">
        <span class="setting-label">{$t('settings.download.music_hotkey_binding')}</span>
        <div class="hotkey-controls">
          <div class="hotkey-mode-switch">
            <button class="hotkey-mode-btn" class:active={musicHotkeyMode === 'record'} onclick={() => { musicHotkeyMode = 'record'; musicHotkeyRecording = false; }}>{$t('settings.download.hotkey_record')}</button>
            <button class="hotkey-mode-btn" class:active={musicHotkeyMode === 'type'} onclick={() => { musicHotkeyMode = 'type'; musicHotkeyRecording = false; }}>{$t('settings.download.hotkey_type')}</button>
          </div>
          {#if musicHotkeyMode === 'type'}
            <input type="text" class="input-hotkey" value={musicHotkeyInput} oninput={handleMusicHotkeyInput} spellcheck="false" />
          {:else}
            <button class="input-hotkey hotkey-record-btn" class:recording={musicHotkeyRecording} onclick={() => { musicHotkeyRecording = true; }} onkeydown={musicHotkeyRecording ? handleMusicHotkeyKeyDown : undefined} onblur={() => { musicHotkeyRecording = false; }}>
              {musicHotkeyRecording ? $t('settings.download.hotkey_press') : (musicHotkeyInput || $t('settings.download.hotkey_press'))}
            </button>
          {/if}
        </div>
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.music_audio_format')}</span>
        <span class="setting-path">{$t('settings.download.music_audio_format_desc')}</span>
      </div>
      <select class="select" value={settings.download.music_audio_format} onchange={changeMusicAudioFormat}>
        <option value="m4a">M4A (AAC)</option>
        <option value="mp3">MP3</option>
        <option value="flac">FLAC (lossless)</option>
        <option value="opus">Opus</option>
        <option value="wav">WAV</option>
      </select>
    </div>
  </div>
</section>

<section class="section">
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.default_output_dir')}</span>
        <span class="setting-path">{settings.download.default_output_dir}</span>
        {#if pathInfo && !pathInfo.ok}
          <span class="path-warning" role="status">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z" />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            {$t('settings.download.path_too_long', { current: pathInfo.current, limit: pathInfo.limit })}
          </span>
        {/if}
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
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.speed_limit')}</span>
        <span class="setting-path">{$t('settings.download.speed_limit_desc')}</span>
      </div>
      <div class="speed-limit">
        <input
          type="number"
          class="input-number"
          min="0"
          step="1"
          inputmode="numeric"
          placeholder={$t('settings.download.speed_limit_unlimited') as string}
          bind:value={speedNum}
          onchange={applySpeedLimit}
          aria-label={$t('settings.download.speed_limit') as string}
        />
        <select class="select" bind:value={speedUnit} onchange={applySpeedLimit} aria-label="unit">
          <option value="K">KB/s</option>
          <option value="M">MB/s</option>
        </select>
      </div>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.download.always_ask_path')}</span>
      <button class="toggle" class:on={settings.download.always_ask_path} onclick={() => toggleBool("download", "always_ask_path", settings.download.always_ask_path)} role="switch" aria-checked={settings.download.always_ask_path} aria-label={$t('settings.download.always_ask_path') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</section>

<section class="section">
  <h5 class="section-title">{$t('settings.download.presets')}</h5>
  <p class="setting-path" style="margin: 0 0 8px 4px">{$t('settings.download.presets_desc')}</p>
  <div class="preset-grid">
    {#each YTDLP_PRESETS as preset (preset.id)}
      <button
        class="preset-card"
        class:active={activePreset === preset.id}
        onclick={() => applyPreset(preset.id)}
        type="button"
      >
        <span class="preset-label">{$t(preset.labelKey)}</span>
        <span class="preset-desc">{$t(preset.descKey)}</span>
      </button>
    {/each}
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
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.caption_locale')}</span>
          <span class="setting-path">{$t('settings.download.caption_locale_desc')}</span>
        </div>
        <select class="select" value={settings.download.caption_locale} onchange={changeCaptionLocale}>
          <option value="en">English</option>
          <option value="pt">Português</option>
          <option value="es">Español</option>
          <option value="fr">Français</option>
          <option value="it">Italiano</option>
          <option value="ja">日本語</option>
          <option value="zh-Hans">简体中文</option>
          <option value="zh-Hant">繁體中文</option>
          <option value="el">Ελληνικά</option>
        </select>
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.keep_vtt')}</span>
          <span class="setting-path">{$t('settings.download.keep_vtt_desc')}</span>
        </div>
        <button class="toggle" class:on={settings.download.keep_vtt} onclick={() => toggleBool("download", "keep_vtt", settings.download.keep_vtt)} role="switch" aria-checked={settings.download.keep_vtt} aria-label={$t('settings.download.keep_vtt') as string}><span class="toggle-knob"></span></button>
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
    {#if settings.download.youtube_sponsorblock}
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.sb_mode')}</span>
          <span class="setting-path">{$t('settings.download.sb_mode_desc')}</span>
        </div>
        <select class="select" value={settings.download.sponsorblock_mode} onchange={setSbMode}>
          <option value="remove">{$t('settings.download.sb_mode_remove')}</option>
          <option value="mark">{$t('settings.download.sb_mode_mark')}</option>
        </select>
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.sb_categories')}</span>
          <span class="setting-path">{$t('settings.download.sb_categories_desc')}</span>
        </div>
      </div>
      <div class="sb-chips">
        {#each SB_CATEGORIES as cat (cat)}
          <button
            type="button"
            class="sb-chip"
            class:on={sbHas(cat)}
            onclick={() => toggleSbCategory(cat)}
            aria-pressed={sbHas(cat)}
          >
            {$t(`settings.download.sb_cat_${cat}`)}
          </button>
        {/each}
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.split_by_chapters')}</span>
        <span class="setting-path">{$t('settings.download.split_by_chapters_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.split_by_chapters} onclick={() => toggleBool("download", "split_by_chapters", settings.download.split_by_chapters)} role="switch" aria-checked={settings.download.split_by_chapters} aria-label={$t('settings.download.split_by_chapters') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.live_from_start')}</span>
        <span class="setting-path">{$t('settings.download.live_from_start_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.live_from_start} onclick={() => toggleBool("download", "live_from_start", settings.download.live_from_start)} role="switch" aria-checked={settings.download.live_from_start} aria-label={$t('settings.download.live_from_start') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</details>

<details class="section">
  <summary class="section-title">{$t('settings.download.courses_section')}</summary>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.continuous_lecture_numbers')}</span>
        <span class="setting-path">{$t('settings.download.continuous_lecture_numbers_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.continuous_lecture_numbers} onclick={() => toggleBool("download", "continuous_lecture_numbers", settings.download.continuous_lecture_numbers)} role="switch" aria-checked={settings.download.continuous_lecture_numbers} aria-label={$t('settings.download.continuous_lecture_numbers') as string}><span class="toggle-knob"></span></button>
    </div>
  </div>
</details>

<details class="section">
  <summary class="section-title">{$t('settings.download.bilibili_section')}</summary>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.bilibili_container_label')}</span>
        <span class="setting-path">{$t('settings.download.bilibili_container_desc')}</span>
      </div>
      <select class="select" value={settings.download.bilibili_container} onchange={setBilibiliContainer}>
        <option value="mp4">MP4</option>
        <option value="mkv">MKV</option>
      </select>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.bilibili_danmaku_label')}</span>
        <span class="setting-path">{$t('settings.download.bilibili_danmaku_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.bilibili_danmaku_enabled} onclick={() => toggleBool("download", "bilibili_danmaku_enabled", settings.download.bilibili_danmaku_enabled)} role="switch" aria-checked={settings.download.bilibili_danmaku_enabled} aria-label={$t('settings.download.bilibili_danmaku_label') as string}><span class="toggle-knob"></span></button>
    </div>
    {#if settings.download.bilibili_danmaku_enabled}
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.bilibili_danmaku_format_label')}</span>
          <span class="setting-path">{$t('settings.download.bilibili_danmaku_format_desc')}</span>
        </div>
        <select class="select" value={settings.download.bilibili_danmaku_format} onchange={setBilibiliDanmakuFormat}>
          <option value="xml">XML</option>
          <option value="ass">ASS</option>
          <option value="json">JSON</option>
        </select>
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.bilibili_nfo_label')}</span>
        <span class="setting-path">{$t('settings.download.bilibili_nfo_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.bilibili_nfo_enabled} onclick={() => toggleBool("download", "bilibili_nfo_enabled", settings.download.bilibili_nfo_enabled)} role="switch" aria-checked={settings.download.bilibili_nfo_enabled} aria-label={$t('settings.download.bilibili_nfo_label') as string}><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.bilibili_cover_sidecar_label')}</span>
        <span class="setting-path">{$t('settings.download.bilibili_cover_sidecar_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.bilibili_cover_sidecar} onclick={() => toggleBool("download", "bilibili_cover_sidecar", settings.download.bilibili_cover_sidecar)} role="switch" aria-checked={settings.download.bilibili_cover_sidecar} aria-label={$t('settings.download.bilibili_cover_sidecar_label') as string}><span class="toggle-knob"></span></button>
    </div>
    {#if settings.download.bilibili_cover_sidecar}
      <div class="divider"></div>
      <div class="setting-row">
        <div class="setting-col">
          <span class="setting-label">{$t('settings.download.bilibili_cover_format_label')}</span>
          <span class="setting-path">{$t('settings.download.bilibili_cover_format_desc')}</span>
        </div>
        <select class="select" value={settings.download.bilibili_cover_format} onchange={setBilibiliCoverFormat}>
          <option value="jpg">JPG</option>
          <option value="png">PNG</option>
          <option value="webp">WebP</option>
          <option value="avif">AVIF</option>
        </select>
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.bilibili_naming_label')}</span>
        <span class="setting-path">{$t('settings.download.bilibili_naming_desc')}</span>
      </div>
      <button type="button" class="ghost-btn" onclick={() => (namingTemplatesOpen = !namingTemplatesOpen)}>
        {namingTemplatesOpen ? $t('settings.download.bilibili_naming_hide') : $t('settings.download.bilibili_naming_edit')}
      </button>
    </div>
    {#if namingTemplatesOpen}
      <div class="naming-block">
        <label class="naming-row">
          <span class="naming-label">{$t('settings.download.bilibili_naming_video_label')}</span>
          <input class="naming-input" type="text" value={settings.download.bilibili_naming_video} oninput={(e) => setBilibiliNamingTemplate('bilibili_naming_video', (e.target as HTMLInputElement).value)} />
        </label>
        <label class="naming-row">
          <span class="naming-label">{$t('settings.download.bilibili_naming_multi_part_label')}</span>
          <input class="naming-input" type="text" value={settings.download.bilibili_naming_multi_part} oninput={(e) => setBilibiliNamingTemplate('bilibili_naming_multi_part', (e.target as HTMLInputElement).value)} />
        </label>
        <label class="naming-row">
          <span class="naming-label">{$t('settings.download.bilibili_naming_bangumi_label')}</span>
          <input class="naming-input" type="text" value={settings.download.bilibili_naming_bangumi} oninput={(e) => setBilibiliNamingTemplate('bilibili_naming_bangumi', (e.target as HTMLInputElement).value)} />
        </label>
        <label class="naming-row">
          <span class="naming-label">{$t('settings.download.bilibili_naming_cheese_label')}</span>
          <input class="naming-input" type="text" value={settings.download.bilibili_naming_cheese} oninput={(e) => setBilibiliNamingTemplate('bilibili_naming_cheese', (e.target as HTMLInputElement).value)} />
        </label>
        <label class="naming-row">
          <span class="naming-label">{$t('settings.download.bilibili_naming_collection_label')}</span>
          <input class="naming-input" type="text" value={settings.download.bilibili_naming_collection} oninput={(e) => setBilibiliNamingTemplate('bilibili_naming_collection', (e.target as HTMLInputElement).value)} />
        </label>
        <p class="naming-help">{$t('settings.download.bilibili_naming_help')}</p>
      </div>
    {/if}
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.bilibili_cdn_label')}</span>
        <span class="setting-path">{$t('settings.download.bilibili_cdn_desc')}</span>
      </div>
      <button type="button" class="ghost-btn" onclick={() => (cdnOpen = !cdnOpen)}>
        {cdnOpen ? $t('settings.download.bilibili_naming_hide') : $t('settings.download.bilibili_naming_edit')}
      </button>
    </div>
    {#if cdnOpen}
      <div class="naming-block">
        <label class="naming-row">
          <span class="naming-label">{$t('settings.download.bilibili_cdn_hosts_label')}</span>
          <textarea class="naming-input" rows="3" value={settings.download.bilibili_cdn_hosts} oninput={setBilibiliCdnHosts} placeholder={$t('settings.download.bilibili_cdn_hosts_placeholder') as string}></textarea>
        </label>
        <div class="setting-row" style="padding: 4px 0;">
          <div class="setting-col">
            <span class="setting-label">{$t('settings.download.bilibili_cdn_prefer_label')}</span>
            <span class="setting-path">{$t('settings.download.bilibili_cdn_prefer_desc')}</span>
          </div>
          <button class="toggle" class:on={settings.download.bilibili_cdn_prefer_alternatives} onclick={() => toggleBool("download", "bilibili_cdn_prefer_alternatives", settings.download.bilibili_cdn_prefer_alternatives)} role="switch" aria-checked={settings.download.bilibili_cdn_prefer_alternatives} aria-label={$t('settings.download.bilibili_cdn_prefer_label') as string}><span class="toggle-knob"></span></button>
        </div>
      </div>
    {/if}
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
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.download.auto_download_on_paste')}</span>
        <span class="setting-path">{$t('settings.download.auto_download_on_paste_desc')}</span>
      </div>
      <button class="toggle" class:on={settings.download.auto_download_on_paste} onclick={() => toggleBool("download", "auto_download_on_paste", settings.download.auto_download_on_paste)} role="switch" aria-checked={settings.download.auto_download_on_paste} aria-label={$t('settings.download.auto_download_on_paste') as string}><span class="toggle-knob"></span></button>
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

<style>
  .path-warning {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin-top: 6px;
    color: var(--warning);
    font-size: 12px;
    line-height: 1.4;
  }
  .path-warning svg {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .speed-limit {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .input-number {
    width: 88px;
    padding: 6px 8px;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--border-radius, 6px);
    font: inherit;
    text-align: right;
  }
  .input-number:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .sb-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 0 8px;
  }
  .sb-chip {
    padding: 5px 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-muted, var(--text));
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: border-color 0.12s ease, background 0.12s ease, color 0.12s ease;
  }
  .sb-chip:hover {
    border-color: var(--accent);
  }
  .sb-chip.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, var(--surface));
    color: var(--text);
  }
  .sb-chip:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .preset-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 8px;
  }
  .preset-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius, 6px);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.12s ease, background 0.12s ease;
  }
  .preset-card:hover {
    border-color: var(--accent);
  }
  .preset-card.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }
  .preset-label {
    font-weight: 600;
    font-size: 0.9rem;
  }
  .preset-desc {
    font-size: 0.78rem;
    color: var(--text-muted, var(--text));
    opacity: 0.75;
  }
  .ghost-btn {
    padding: 6px 12px;
    border-radius: 8px;
    border: 1px solid var(--content-border, color-mix(in oklab, var(--text) 15%, transparent));
    background: transparent;
    color: var(--text);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .ghost-btn:hover {
    background: color-mix(in oklab, var(--text) 6%, transparent);
  }
  .naming-block {
    padding: 14px 16px 4px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .naming-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .naming-label {
    font-size: 0.78rem;
    color: var(--text-muted, var(--text));
    opacity: 0.85;
  }
  .naming-input {
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    background: var(--surface);
    color: var(--text);
    font-family: ui-monospace, "JetBrains Mono", Menlo, Consolas, monospace;
    font-size: 0.8rem;
  }
  .naming-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .naming-help {
    font-size: 0.72rem;
    color: var(--text-muted, var(--text));
    opacity: 0.7;
    margin: 4px 0 0;
    line-height: 1.4;
  }
</style>
