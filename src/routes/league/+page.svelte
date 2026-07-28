<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { t, locale } from "$lib/i18n";
  import { getSettings, updateSettings } from "$lib/stores/settings-store.svelte";
  import timeAgo from "$lib/time-ago";

  const CDRAGON = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1";

  type LeagueStatus = { connected: boolean; port: number | null; region: string | null };
  type RankedEntry = { tier?: string; division?: string; leaguePoints?: number; wins?: number; losses?: number };

  let settings = $derived(getSettings());
  let enabled = $derived(settings?.league?.enabled ?? false);

  let status = $state<LeagueStatus>({ connected: false, port: null, region: null });
  let summoner = $state<any>(null);
  let ranked = $state<Record<string, RankedEntry>>({});
  let phase = $state<string>("");
  let games = $state<any[]>([]);
  let loadingHistory = $state(false);
  let autoAccept = $state(false);
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const QUEUE_NAMES: Record<number, string> = {
    420: "Solo/Duo",
    440: "Flex",
    400: "Draft",
    430: "Blind",
    450: "ARAM",
    900: "URF",
    1700: "Arena",
    490: "Quickplay",
  };

  function queueName(id: number, mode: string): string {
    return QUEUE_NAMES[id] ?? mode ?? "";
  }

  function rankLabel(entry: RankedEntry | undefined): string {
    if (!entry?.tier || entry.tier === "NONE" || entry.tier === "") return $t("league.unranked") as string;
    const tier = entry.tier.charAt(0) + entry.tier.slice(1).toLowerCase();
    return `${tier} ${entry.division ?? ""} · ${entry.leaguePoints ?? 0} LP`;
  }

  async function refreshStatus() {
    try {
      status = await invoke<LeagueStatus>("league_status");
    } catch {
      status = { connected: false, port: null, region: null };
    }
    if (!status.connected) {
      summoner = null;
      phase = "";
      return;
    }
    try {
      phase = await invoke<string>("league_gameflow");
    } catch {
      phase = "";
    }
    if (!summoner) {
      await loadProfile();
    }
  }

  async function loadProfile() {
    try {
      summoner = await invoke<any>("league_summoner");
    } catch {
      summoner = null;
      return;
    }
    try {
      const stats = await invoke<any>("league_ranked");
      ranked = stats?.queueMap ?? {};
    } catch {
      ranked = {};
    }
    loadHistory();
  }

  async function loadHistory() {
    if (loadingHistory) return;
    loadingHistory = true;
    try {
      const data = await invoke<any>("league_match_history", { begIndex: 0, endIndex: 12 });
      games = data?.games?.games ?? [];
    } catch {
      games = [];
    } finally {
      loadingHistory = false;
    }
  }

  async function toggleAutoAccept() {
    const next = !autoAccept;
    autoAccept = next;
    updateSettings({ league: { auto_accept: next } });
    try {
      await invoke("league_auto_accept_set", { enabled: next });
    } catch {
      autoAccept = !next;
    }
  }

  async function acceptNow() {
    try {
      await invoke("league_accept_ready_check");
    } catch {}
  }

  function playerStats(game: any): { championId: number; kills: number; deaths: number; assists: number; win: boolean } {
    const p = game?.participants?.[0];
    return {
      championId: p?.championId ?? 0,
      kills: p?.stats?.kills ?? 0,
      deaths: p?.stats?.deaths ?? 0,
      assists: p?.stats?.assists ?? 0,
      win: p?.stats?.win ?? false,
    };
  }

  onMount(() => {
    if (!enabled) return;
    invoke<boolean>("league_auto_accept_get")
      .then((v) => { autoAccept = v; })
      .catch(() => {});
    refreshStatus();
    pollTimer = setInterval(refreshStatus, 4000);
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
  });
</script>

<div class="league-page">
  {#if !enabled}
    <div class="guard-card">
      <h2>{$t("league.disabled_title")}</h2>
      <p>{$t("league.disabled_body")}</p>
      <button class="button primary" onclick={() => goto("/settings")}>{$t("league.open_settings")}</button>
    </div>
  {:else}
    <header class="league-header">
      <h2>{$t("league.nav")}</h2>
      <div class="status-chip" class:connected={status.connected}>
        <span class="dot"></span>
        {status.connected ? $t("league.connected") : $t("league.disconnected_title")}
        {#if status.connected && status.region}
          <span class="region">{status.region}</span>
        {/if}
      </div>
    </header>

    {#if !status.connected}
      <div class="guard-card">
        <p>{$t("league.disconnected_body")}</p>
      </div>
    {:else}
      {#if summoner}
        <section class="profile-card">
          <img
            class="profile-icon"
            src={`${CDRAGON}/profile-icons/${summoner.profileIconId}.jpg`}
            alt=""
            loading="lazy"
          />
          <div class="profile-info">
            <span class="profile-name">
              {summoner.gameName ?? summoner.displayName}{#if summoner.tagLine}<span class="tag">#{summoner.tagLine}</span>{/if}
            </span>
            <span class="profile-level">{$t("league.level")} {summoner.summonerLevel}</span>
          </div>
          <div class="ranked-chips">
            <div class="ranked-chip">
              <span class="ranked-queue">{$t("league.ranked_solo")}</span>
              <span class="ranked-value">{rankLabel(ranked?.RANKED_SOLO_5x5)}</span>
            </div>
            <div class="ranked-chip">
              <span class="ranked-queue">{$t("league.ranked_flex")}</span>
              <span class="ranked-value">{rankLabel(ranked?.RANKED_FLEX_SR)}</span>
            </div>
          </div>
        </section>
      {/if}

      <section class="actions-card">
        <div class="action-row">
          <div class="action-col">
            <span class="action-label">{$t("league.auto_accept")}</span>
            <span class="action-hint">{$t("league.auto_accept_desc")}</span>
          </div>
          <button
            class="toggle"
            class:on={autoAccept}
            onclick={toggleAutoAccept}
            role="switch"
            aria-checked={autoAccept}
            aria-label={$t("league.auto_accept") as string}
          >
            <span class="toggle-knob"></span>
          </button>
        </div>
        {#if phase && phase !== "None"}
          <div class="divider"></div>
          <div class="action-row">
            <div class="action-col">
              <span class="action-label">{$t("league.phase_label")}</span>
              <span class="action-hint">{phase}</span>
            </div>
            {#if phase === "ReadyCheck"}
              <button class="button primary" onclick={acceptNow}>{$t("league.accept_now")}</button>
            {/if}
          </div>
        {/if}
      </section>

      <section class="history-section">
        <div class="history-head">
          <h3>{$t("league.history_title")}</h3>
          <button class="button" onclick={loadHistory} disabled={loadingHistory}>{$t("league.refresh")}</button>
        </div>
        {#if games.length === 0}
          <p class="history-empty">{$t("league.history_empty")}</p>
        {:else}
          <div class="game-list">
            {#each games as game (game.gameId)}
              {@const p = playerStats(game)}
              <div class="game-row" class:win={p.win} class:loss={!p.win}>
                <img
                  class="champ-icon"
                  src={`${CDRAGON}/champion-icons/${p.championId}.png`}
                  alt=""
                  loading="lazy"
                />
                <div class="game-info">
                  <span class="game-result">{p.win ? $t("league.victory") : $t("league.defeat")}</span>
                  <span class="game-mode">{queueName(game.queueId, game.gameMode)}</span>
                </div>
                <span class="game-kda">{p.kills} / {p.deaths} / {p.assists}</span>
                <span class="game-time">{timeAgo(game.gameCreation, $locale)}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .league-page {
    display: flex;
    flex-direction: column;
    gap: var(--padding);
    padding: var(--padding);
    max-width: 720px;
    margin: 0 auto;
    width: 100%;
  }

  .guard-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: calc(var(--padding) * 2);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
  }

  .guard-card h2 {
    margin: 0;
    font-size: 18px;
  }

  .guard-card p {
    margin: 0;
    color: var(--gray);
    font-size: 13.5px;
  }

  .league-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .league-header h2 {
    margin: 0;
    font-size: 20px;
  }

  .status-chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 12px;
    font-size: 12.5px;
    color: var(--gray);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
  }

  .status-chip .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--gray);
  }

  .status-chip.connected {
    color: var(--text);
  }

  .status-chip.connected .dot {
    background: var(--green, #4ade80);
  }

  .status-chip .region {
    color: var(--gray);
    text-transform: uppercase;
  }

  .profile-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: var(--padding);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
  }

  .profile-icon {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    border: 2px solid var(--border);
    object-fit: cover;
  }

  .profile-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .profile-name {
    font-size: 16px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .profile-name .tag {
    color: var(--gray);
    font-weight: 400;
  }

  .profile-level {
    font-size: 12.5px;
    color: var(--gray);
  }

  .ranked-chips {
    display: flex;
    gap: 8px;
    margin-left: auto;
    flex-wrap: wrap;
  }

  .ranked-chip {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 12px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 2px);
  }

  .ranked-queue {
    font-size: 11px;
    color: var(--gray);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .ranked-value {
    font-size: 13px;
  }

  .actions-card {
    display: flex;
    flex-direction: column;
    padding: var(--padding);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
  }

  .action-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .action-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .action-label {
    font-size: 14px;
  }

  .action-hint {
    font-size: 12.5px;
    color: var(--gray);
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 10px 0;
  }

  .history-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .history-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .history-head h3 {
    margin: 0;
    font-size: 15px;
  }

  .history-empty {
    color: var(--gray);
    font-size: 13px;
    margin: 0;
  }

  .game-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .game-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-left-width: 3px;
    border-radius: calc(var(--border-radius) - 2px);
  }

  .game-row.win {
    border-left-color: var(--green, #4ade80);
  }

  .game-row.loss {
    border-left-color: var(--red, #f87171);
  }

  .champ-icon {
    width: 34px;
    height: 34px;
    border-radius: 6px;
    object-fit: cover;
  }

  .game-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .game-result {
    font-size: 13px;
    font-weight: 600;
  }

  .game-mode {
    font-size: 11.5px;
    color: var(--gray);
  }

  .game-kda {
    margin-left: auto;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  .game-time {
    font-size: 11.5px;
    color: var(--gray);
    min-width: 70px;
    text-align: right;
  }

  .button {
    padding: 6px 14px;
    font-size: 13px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 2px);
    color: var(--text);
    cursor: pointer;
  }

  .button.primary {
    background: var(--accent, var(--secondary));
    color: var(--on-accent, var(--primary));
    border-color: transparent;
  }

  .button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .toggle {
    position: relative;
    width: 40px;
    height: 22px;
    border-radius: 999px;
    background: var(--button);
    border: 1px solid var(--input-border);
    cursor: pointer;
    flex-shrink: 0;
  }

  .toggle .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--gray);
    transition: transform 0.15s ease, background 0.15s ease;
  }

  .toggle.on .toggle-knob {
    transform: translateX(18px);
    background: var(--accent, var(--secondary));
  }
</style>
