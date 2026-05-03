<script lang="ts">
  import { t } from "$lib/i18n";
  import { getSettings, updateSettings, toggleBool, changeNumber } from "./settings-helpers";

  let settings = $derived(getSettings());

  let proxyHost = $state("");
  let proxyUsername = $state("");
  let proxyPassword = $state("");
  let proxyTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (settings) {
      proxyHost = settings.proxy?.host ?? "";
      proxyUsername = settings.proxy?.username ?? "";
      proxyPassword = settings.proxy?.password ?? "";
    }
  });

  async function changeProxyType(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    await updateSettings({ proxy: { proxy_type: value } });
  }

  function handleProxyHost(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    proxyHost = value;
    if (proxyTimer) clearTimeout(proxyTimer);
    proxyTimer = setTimeout(async () => {
      await updateSettings({ proxy: { host: value } });
    }, 800);
  }

  function handleProxyUsername(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    proxyUsername = value;
    if (proxyTimer) clearTimeout(proxyTimer);
    proxyTimer = setTimeout(async () => {
      await updateSettings({ proxy: { username: value } });
    }, 800);
  }

  function handleProxyPassword(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    proxyPassword = value;
    if (proxyTimer) clearTimeout(proxyTimer);
    proxyTimer = setTimeout(async () => {
      await updateSettings({ proxy: { password: value } });
    }, 800);
  }

  const YTDLP_FLAG_CATALOG = [
    { flag: "--embed-subs", label: "Embed subtitles" },
    { flag: "--write-thumbnail", label: "Save thumbnail" },
    { flag: "--write-description", label: "Save description" },
    { flag: "--write-comments", label: "Save comments" },
    { flag: "--restrict-filenames", label: "ASCII filenames" },
    { flag: "--no-overwrites", label: "No overwrites" },
    { flag: "--prefer-free-formats", label: "Free formats" },
    { flag: "--force-ipv4", label: "Force IPv4" },
    { flag: "--geo-bypass", label: "Geo bypass" },
    { flag: "--limit-rate", label: "Limit rate", hasValue: true, placeholder: "e.g. 1M" },
    { flag: "--sleep-interval", label: "Sleep interval", hasValue: true, placeholder: "e.g. 5" },
  ];

  async function toggleFlag(flag: string) {
    if (!settings) return;
    let current = [...(settings.download?.extra_ytdlp_flags ?? [])];
    const idx = current.findIndex((f) => f === flag || f.startsWith(flag + " "));
    if (idx >= 0) current.splice(idx, 1);
    else current.push(flag);
    await updateSettings({ download: { extra_ytdlp_flags: current } });
  }

  async function setFlagValue(flag: string, value: string) {
    if (!settings) return;
    let current = [...(settings.download?.extra_ytdlp_flags ?? [])];
    const idx = current.findIndex((f) => f === flag || f.startsWith(flag + " "));
    if (idx >= 0) {
      current[idx] = value ? `${flag} ${value}` : flag;
    }
    await updateSettings({ download: { extra_ytdlp_flags: current } });
  }

  function isFlagActive(flag: string): boolean {
    return (settings?.download?.extra_ytdlp_flags ?? []).some((f) => f === flag || f.startsWith(flag + " "));
  }

  function getFlagValue(flag: string): string {
    const f = (settings?.download?.extra_ytdlp_flags ?? []).find((f) => f.startsWith(flag + " "));
    return f ? f.slice(flag.length + 1) : "";
  }
</script>

{#if settings}
<section class="section">
  <h5 class="section-title">{$t('settings.proxy.title')}</h5>
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t('settings.proxy.enabled')}</span>
      </div>
      <button
        class="toggle"
        class:on={settings.proxy?.enabled}
        onclick={() => toggleBool("proxy", "enabled", settings.proxy?.enabled ?? false)}
        role="switch"
        aria-checked={settings.proxy?.enabled ?? false}
        aria-label={$t('settings.proxy.enabled') as string}
      >
        <span class="toggle-knob"></span>
      </button>
    </div>
    {#if settings.proxy?.enabled}
      <div class="divider"></div>
      <div class="setting-row">
        <span class="setting-label">{$t('settings.proxy.type')}</span>
        <select class="select" value={settings.proxy?.proxy_type ?? 'http'} onchange={changeProxyType}>
          <option value="http">HTTP</option>
          <option value="https">HTTPS</option>
          <option value="socks5">SOCKS5</option>
        </select>
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <span class="setting-label">{$t('settings.proxy.host')}</span>
        <input type="text" class="input-text" value={proxyHost} oninput={handleProxyHost} placeholder="127.0.0.1" spellcheck="false" />
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <span class="setting-label">{$t('settings.proxy.port')}</span>
        <input type="number" class="input-number" min="1" max="65535" value={settings.proxy?.port ?? 8080} onchange={(e) => changeNumber("proxy", "port", e)} />
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <span class="setting-label">{$t('settings.proxy.username')}</span>
        <input type="text" class="input-text" value={proxyUsername} oninput={handleProxyUsername} placeholder="" spellcheck="false" />
      </div>
      <div class="divider"></div>
      <div class="setting-row">
        <span class="setting-label">{$t('settings.proxy.password')}</span>
        <input type="password" class="input-text" value={proxyPassword} oninput={handleProxyPassword} placeholder="" />
      </div>
    {/if}
  </div>
</section>

<section class="section">
  <h5 class="section-title">{$t('settings.ytdlp_flags.title')}</h5>
  <div class="card">
    <div class="flag-grid">
      {#each YTDLP_FLAG_CATALOG as item (item.flag)}
        <button
          class="flag-chip"
          class:active={isFlagActive(item.flag)}
          onclick={() => toggleFlag(item.flag)}
          title={item.flag}
        >
          {item.label}
        </button>
        {#if item.hasValue && isFlagActive(item.flag)}
          <input
            class="flag-value-input"
            type="text"
            placeholder={item.placeholder}
            value={getFlagValue(item.flag)}
            oninput={(e) => {
              const val = (e.target as HTMLInputElement).value;
              setFlagValue(item.flag, val);
            }}
            spellcheck="false"
          />
        {/if}
      {/each}
    </div>
  </div>
</section>
{/if}
