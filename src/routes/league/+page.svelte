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
  type Champion = { id: number; name: string; alias: string };
  type LobbyQueue = { id: number; name: string; shortName: string; gameMode: string };

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

  let champions = $state<Champion[]>([]);
  let championById = $derived(new Map(champions.map((c) => [c.id, c])));
  let championByAlias = $derived(new Map(champions.map((c) => [c.alias.toLowerCase(), c])));

  let queues = $state<LobbyQueue[]>([]);
  let lobby = $state<any>(null);
  let champSelect = $state<any>(null);
  let liveGame = $state<any>(null);
  let actionError = $state("");

  let pickSearch = $state("");
  let banSearch = $state("");

  type ScoutPlayer = { puuid: string; gameName: string; tagLine: string; championId: number; cellId: number | null; isAlly: boolean; summonerLevel?: number };
  let scoutPlayers = $state<ScoutPlayer[]>([]);
  let scoutReports = $state<Record<string, any>>({});
  let scoutLoading = $state(false);
  let notes = $state<Record<string, string>>({});
  let openNotes = $state<Record<string, boolean>>({});

  const NOTES_KEY = "league-player-notes";

  const TAG_KEYS: Record<string, string> = {
    hot_streak: "league.tag_hot_streak",
    cold_streak: "league.tag_cold_streak",
    high_winrate: "league.tag_high_winrate",
    low_winrate: "league.tag_low_winrate",
    one_trick: "league.tag_one_trick",
    high_kda: "league.tag_high_kda",
    low_kda: "league.tag_low_kda",
  };

  function loadNotes() {
    try {
      notes = JSON.parse(localStorage.getItem(NOTES_KEY) ?? "{}");
    } catch {
      notes = {};
    }
  }

  function saveNote(puuid: string, text: string) {
    notes = { ...notes, [puuid]: text };
    const clean = Object.fromEntries(Object.entries(notes).filter(([, v]) => (v as string).trim() !== ""));
    localStorage.setItem(NOTES_KEY, JSON.stringify(clean));
  }

  async function loadScouting() {
    if (scoutLoading) return;
    scoutLoading = true;
    try {
      const data = await invoke<any>("league_game_players");
      scoutPlayers = (data?.players ?? []).filter((p: any) => p);
      for (const p of scoutPlayers) {
        if (!p.puuid || scoutReports[p.puuid]) continue;
        invoke<any>("league_player_report", { puuid: p.puuid })
          .then((r) => {
            scoutReports = { ...scoutReports, [p.puuid]: r };
          })
          .catch(() => {});
      }
    } catch {
      scoutPlayers = [];
    } finally {
      scoutLoading = false;
    }
  }

  function scoutGroups(): { label: string; ally: boolean; list: ScoutPlayer[] }[] {
    return [
      { label: $t("league.scout_enemies") as string, ally: false, list: scoutPlayers.filter((p) => !p.isAlly) },
      { label: $t("league.scout_allies") as string, ally: true, list: scoutPlayers.filter((p) => p.isAlly) },
    ];
  }

  const TAB_IDS = ["overview", "analysis", "meta", "search", "live", "goals", "automation", "history"] as const;
  type Tab = (typeof TAB_IDS)[number];
  let tab = $state<Tab>("overview");

  let analysis = $state<any>(null);
  let analysisLoading = $state(false);
  let liveMetrics = $state<any>(null);

  const ROLES = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"] as const;
  type Role = (typeof ROLES)[number];
  const GOALS_KEY = "league-role-goals";
  const GOAL_FIELDS = [
    { key: "csPerMin", labelKey: "league.goal_cs", step: 0.1 },
    { key: "goldPerMin", labelKey: "league.goal_gold", step: 10 },
    { key: "kda", labelKey: "league.goal_kda", step: 0.1 },
    { key: "visionPerMin", labelKey: "league.goal_vision", step: 0.1 },
  ] as const;
  type GoalKey = (typeof GOAL_FIELDS)[number]["key"];

  // Baselines mirror the Rust defaults; a support is not judged on CS.
  const DEFAULT_GOALS: Record<Role, Record<GoalKey, number>> = {
    TOP: { csPerMin: 7.0, goldPerMin: 380, kda: 2.5, visionPerMin: 0.6 },
    JUNGLE: { csPerMin: 5.5, goldPerMin: 360, kda: 3.0, visionPerMin: 1.0 },
    MIDDLE: { csPerMin: 7.5, goldPerMin: 400, kda: 3.0, visionPerMin: 0.7 },
    BOTTOM: { csPerMin: 8.0, goldPerMin: 420, kda: 3.0, visionPerMin: 0.6 },
    UTILITY: { csPerMin: 1.5, goldPerMin: 260, kda: 3.0, visionPerMin: 2.0 },
  };

  let goalRole = $state<Role>("MIDDLE");
  let goals = $state<Record<string, Record<string, number>>>({});

  function loadGoals() {
    try {
      goals = JSON.parse(localStorage.getItem(GOALS_KEY) ?? "{}");
    } catch {
      goals = {};
    }
  }

  function goalValue(role: Role, key: GoalKey): number {
    return goals[role]?.[key] ?? DEFAULT_GOALS[role][key];
  }

  function setGoal(role: Role, key: GoalKey, value: number) {
    if (!Number.isFinite(value) || value < 0) return;
    goals = { ...goals, [role]: { ...(goals[role] ?? {}), [key]: value } };
    localStorage.setItem(GOALS_KEY, JSON.stringify(goals));
  }

  function resetGoals(role: Role) {
    const next = { ...goals };
    delete next[role];
    goals = next;
    localStorage.setItem(GOALS_KEY, JSON.stringify(goals));
  }

  async function loadAnalysis() {
    if (analysisLoading) return;
    analysisLoading = true;
    try {
      analysis = await invoke<any>("league_match_analysis");
    } catch {
      analysis = null;
    } finally {
      analysisLoading = false;
    }
  }

  async function loadLiveMetrics() {
    try {
      liveMetrics = await invoke<any>("league_live_metrics");
    } catch {
      liveMetrics = null;
    }
  }

  let selfRow = $derived(liveMetrics?.players?.find((r: any) => r.isSelf) ?? null);
  let myTeam = $derived(selfRow?.team ?? "ORDER");
  let enemyTeam = $derived(myTeam === "ORDER" ? "CHAOS" : "ORDER");
  let teamGoldLead = $derived(
    (liveMetrics?.teamGold?.[myTeam] ?? 0) - (liveMetrics?.teamGold?.[enemyTeam] ?? 0)
  );

  let liveGoals = $derived.by(() => {
    if (!selfRow) return [];
    const role: Role = (ROLES as readonly string[]).includes(selfRow.position)
      ? (selfRow.position as Role)
      : "MIDDLE";
    const minutes = Math.max((liveMetrics?.gameTime ?? 0) / 60, 0.1);
    const visionPerMin = (selfRow.visionScore ?? 0) / minutes;
    return [
      {
        key: "csPerMin",
        labelKey: "league.goal_cs",
        current: (selfRow.csPerMin ?? 0).toFixed(1),
        target: goalValue(role, "csPerMin").toFixed(1),
        ratio: (selfRow.csPerMin ?? 0) / goalValue(role, "csPerMin"),
      },
      {
        key: "goldPerMin",
        labelKey: "league.goal_gold",
        current: String(selfRow.goldPerMin ?? 0),
        target: String(goalValue(role, "goldPerMin")),
        ratio: (selfRow.goldPerMin ?? 0) / goalValue(role, "goldPerMin"),
      },
      {
        key: "kda",
        labelKey: "league.goal_kda",
        current: (selfRow.kda ?? 0).toFixed(2),
        target: goalValue(role, "kda").toFixed(1),
        ratio: (selfRow.kda ?? 0) / goalValue(role, "kda"),
      },
      {
        key: "visionPerMin",
        labelKey: "league.goal_vision",
        current: visionPerMin.toFixed(1),
        target: goalValue(role, "visionPerMin").toFixed(1),
        ratio: visionPerMin / goalValue(role, "visionPerMin"),
      },
    ];
  });

  let searchQuery = $state("");
  let searchResult = $state<any>(null);
  let searchLoading = $state(false);
  let searchError = $state("");
  let jungleReport = $state<any>(null);
  let duos = $state<any>(null);
  let duosLoading = $state(false);
  let chatSending = $state(false);
  let chatPreview = $state("");

  async function runSearch() {
    const raw = searchQuery.trim();
    if (!raw || searchLoading) return;
    const [namePart, tagPart = ""] = raw.split("#");
    searchLoading = true;
    searchError = "";
    jungleReport = null;
    try {
      searchResult = await invoke<any>("league_search_player", {
        gameName: namePart.trim(),
        tagLine: tagPart.trim(),
      });
      const puuid = searchResult?.summoner?.puuid;
      if (puuid) {
        invoke<any>("league_jungle_report", { puuid, sample: 8 })
          .then((r) => { jungleReport = r; })
          .catch(() => { jungleReport = null; });
      }
    } catch (e: any) {
      searchResult = null;
      searchError = typeof e === "string" ? e : (e?.message ?? String(e));
    } finally {
      searchLoading = false;
    }
  }

  async function loadDuos() {
    if (duosLoading) return;
    duosLoading = true;
    try {
      duos = await invoke<any>("league_duos", { sample: 20 });
    } catch {
      duos = { duos: [] };
    } finally {
      duosLoading = false;
    }
  }

  // Builds the one-line-per-player summary that gets posted to champ select.
  function buildChatSummary(): string {
    const parts: string[] = [];
    for (const p of scoutPlayers.filter((x) => !x.isAlly)) {
      const r = p.puuid ? scoutReports[p.puuid] : null;
      if (!r?.stats?.games) continue;
      const champ = championById.get(p.championId)?.name ?? "";
      parts.push(`${champ || p.gameName}: ${r.stats.winrate}%WR ${r.stats.kda}KDA (${r.stats.games})`);
    }
    return parts.join(" | ");
  }

  async function sendChatSummary() {
    const text = chatPreview.trim();
    if (!text || chatSending) return;
    chatSending = true;
    actionError = "";
    try {
      await invoke("league_send_chat", { message: text });
      chatPreview = "";
    } catch (e: any) {
      actionError = typeof e === "string" ? e : (e?.message ?? String(e));
    } finally {
      chatSending = false;
    }
  }

  const TIER_POSITIONS = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"] as const;
  let tierPosition = $state<string>("JUNGLE");
  let tiers = $state<any>(null);
  let tiersLoading = $state(false);
  let tiersError = $state("");

  let runePages = $state<any[]>([]);
  let runeApplying = $state(false);
  let runeError = $state("");
  let appliedRuneIndex = $state<number | null>(null);
  let lastRuneChampion = 0;

  let perkMeta = $state<Record<number, string>>({});
  let spellMeta = $state<Record<number, string>>({});

  // Champion the local player has locked in (or hovered) during champ select.
  let champSelectChampionId = $derived.by(() => {
    const cell = champSelect?.localPlayerCellId;
    if (cell === undefined || cell === null) return 0;
    const me = (champSelect?.myTeam ?? []).find((m: any) => m.cellId === cell);
    return me?.championId ?? 0;
  });

  let myAssignedPosition = $derived.by(() => {
    const cell = champSelect?.localPlayerCellId;
    const me = (champSelect?.myTeam ?? []).find((m: any) => m.cellId === cell);
    return (me?.assignedPosition ?? "").toUpperCase();
  });

  function perkName(id: number): string {
    return perkMeta[id] ?? String(id);
  }

  function spellName(id: number): string {
    return spellMeta[id] ?? String(id);
  }

  // op.gg encodes tiers as 1 = best; the labels mirror what their site shows.
  function tierLabel(tier: number): string {
    if (tier <= 1) return "S+";
    if (tier === 2) return "S";
    if (tier === 3) return "A";
    if (tier === 4) return "B";
    if (tier === 5) return "C";
    return "D";
  }

  let tierRows = $derived.by(() => {
    const wanted = tierPosition;
    const rows: any[] = [];
    for (const champ of tiers?.champions ?? []) {
      const entry = (champ.positions ?? []).find((p: any) => (p.position ?? "").toUpperCase() === wanted);
      if (!entry || entry.tier === null || entry.tier === undefined) continue;
      rows.push({
        championId: champ.championId,
        tier: entry.tier,
        rank: entry.rank ?? 999,
        winRate: Math.round((entry.winRate ?? 0) * 1000) / 10,
        pickRate: Math.round((entry.pickRate ?? 0) * 1000) / 10,
        banRate: Math.round((entry.banRate ?? 0) * 1000) / 10,
      });
    }
    rows.sort((a, b) => a.tier - b.tier || a.rank - b.rank);
    return rows.slice(0, 40);
  });

  async function loadTiers() {
    if (tiersLoading) return;
    tiersLoading = true;
    tiersError = "";
    try {
      tiers = await invoke<any>("league_champion_tiers", {
        region: (status.region ?? "br").toLowerCase(),
      });
    } catch (e: any) {
      tiers = null;
      tiersError = typeof e === "string" ? e : (e?.message ?? String(e));
    } finally {
      tiersLoading = false;
    }
  }

  async function loadPerkMeta() {
    try {
      const perks = await invoke<any[]>("league_get", { path: "/lol-perks/v1/perks" });
      const map: Record<number, string> = {};
      for (const p of perks ?? []) map[p.id] = p.name;
      perkMeta = map;
    } catch {
      perkMeta = {};
    }
    try {
      const spells = await invoke<any[]>("league_get", {
        path: "/lol-game-data/assets/v1/summoner-spells.json",
      });
      const map: Record<number, string> = {};
      for (const s of spells ?? []) map[s.id] = s.name;
      spellMeta = map;
    } catch {
      spellMeta = {};
    }
  }

  async function loadRunes(championId: number) {
    if (championId <= 0) {
      runePages = [];
      return;
    }
    runeError = "";
    try {
      const result = await invoke<any>("league_rune_recommendations", {
        championId,
        position: myAssignedPosition || null,
      });
      runePages = result?.pages ?? [];
      appliedRuneIndex = null;
    } catch (e: any) {
      runePages = [];
      runeError = typeof e === "string" ? e : (e?.message ?? String(e));
    }
  }

  async function applyRunePage(index: number) {
    const page = runePages[index];
    if (!page || runeApplying) return;
    runeApplying = true;
    runeError = "";
    try {
      const champName = championById.get(champSelectChampionId)?.name ?? "";
      await invoke("league_apply_runes", {
        name: `${champName} ${page.keystoneName ?? ""}`.trim(),
        primaryStyleId: page.primaryStyleId,
        subStyleId: page.subStyleId,
        selectedPerkIds: page.selectedPerkIds,
        spell1: page.summonerSpellIds?.[0] ?? null,
        spell2: page.summonerSpellIds?.[1] ?? null,
      });
      appliedRuneIndex = index;
    } catch (e: any) {
      runeError = typeof e === "string" ? e : (e?.message ?? String(e));
    } finally {
      runeApplying = false;
    }
  }

  // Load recommendations when the locked champion changes; apply the first one
  // automatically only when the user asked for it.
  $effect(() => {
    const champ = champSelectChampionId;
    if (champ > 0 && champ !== lastRuneChampion) {
      lastRuneChampion = champ;
      loadRunes(champ).then(() => {
        if (settings?.league?.auto_runes && runePages.length > 0) {
          applyRunePage(0);
        }
      });
    } else if (champ === 0 && lastRuneChampion !== 0) {
      lastRuneChampion = 0;
      runePages = [];
    }
  });

  const PHASE_KEYS: Record<string, string> = {
    Lobby: "league.phase_lobby",
    Matchmaking: "league.phase_matchmaking",
    ReadyCheck: "league.phase_ready_check",
    ChampSelect: "league.phase_champ_select",
    GameStart: "league.phase_in_progress",
    InProgress: "league.phase_in_progress",
    WaitingForStats: "league.phase_end_of_game",
    PreEndOfGame: "league.phase_end_of_game",
    EndOfGame: "league.phase_end_of_game",
  };

  function phaseLabel(p: string): string {
    const key = PHASE_KEYS[p];
    return key ? ($t(key) as string) : p;
  }

  const QUEUE_NAMES: Record<number, string> = {
    420: "Solo/Duo",
    440: "Flex",
    400: "Draft",
    430: "Blind",
    450: "ARAM",
    480: "Swiftplay",
    900: "URF",
    1700: "Arena",
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
      lobby = null;
      champSelect = null;
      liveGame = null;
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
    if (champions.length === 0) {
      loadChampions();
    }
    if (queues.length === 0) {
      loadQueues();
    }
    refreshPhaseData();
  }

  async function refreshPhaseData() {
    if (phase === "ChampSelect") {
      try {
        champSelect = await invoke<any>("league_champ_select_session");
      } catch {
        champSelect = null;
      }
    } else {
      champSelect = null;
    }
    if (phase === "InProgress") {
      try {
        liveGame = await invoke<any>("league_live_game");
      } catch {
        liveGame = null;
      }
    } else {
      liveGame = null;
    }
    if (phase === "ChampSelect" || phase === "InProgress") {
      if (scoutPlayers.length === 0) loadScouting();
      if (!analysis && !analysisLoading) loadAnalysis();
    } else if (scoutPlayers.length > 0) {
      scoutPlayers = [];
      analysis = null;
    }
    if (phase === "InProgress") {
      loadLiveMetrics();
    } else if (liveMetrics) {
      liveMetrics = null;
    }
    if (phase === "Lobby" || phase === "Matchmaking") {
      try {
        lobby = await invoke<any>("league_get", { path: "/lol-lobby/v2/lobby" });
      } catch {
        lobby = null;
      }
    } else {
      lobby = null;
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

  async function loadChampions() {
    try {
      const data = await invoke<any[]>("league_get", { path: "/lol-game-data/assets/v1/champion-summary.json" });
      champions = (data ?? [])
        .filter((c) => c.id > 0)
        .map((c) => ({ id: c.id, name: c.name, alias: c.alias }))
        .sort((a, b) => a.name.localeCompare(b.name));
    } catch {
      champions = [];
    }
  }

  async function loadQueues() {
    try {
      queues = await invoke<LobbyQueue[]>("league_lobby_queues");
    } catch {
      queues = [];
    }
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

  async function action(cmd: string, args?: Record<string, unknown>) {
    actionError = "";
    try {
      await invoke(cmd, args ?? {});
      refreshStatus();
    } catch (e: any) {
      actionError = typeof e === "string" ? e : e.message ?? String(e);
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

  function toggleLeagueFlag(field: "auto_pick" | "auto_ban" | "auto_lock" | "auto_runes") {
    const current = (settings?.league as any)?.[field] ?? false;
    updateSettings({ league: { [field]: !current } });
  }

  function listFor(kind: "pick" | "ban"): number[] {
    const l = settings?.league as any;
    return (kind === "pick" ? l?.pick_champions : l?.ban_champions) ?? [];
  }

  function saveList(kind: "pick" | "ban", ids: number[]) {
    updateSettings({ league: kind === "pick" ? { pick_champions: ids } : { ban_champions: ids } });
  }

  function addToList(kind: "pick" | "ban", id: number) {
    const ids = listFor(kind);
    if (ids.includes(id)) return;
    saveList(kind, [...ids, id]);
    if (kind === "pick") pickSearch = "";
    else banSearch = "";
  }

  function removeFromList(kind: "pick" | "ban", id: number) {
    saveList(kind, listFor(kind).filter((x) => x !== id));
  }

  function searchResults(query: string, kind: "pick" | "ban"): Champion[] {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    const existing = new Set(listFor(kind));
    return champions
      .filter((c) => !existing.has(c.id))
      .filter((c) => c.name.toLowerCase().includes(q) || c.alias.toLowerCase().includes(q))
      .slice(0, 8);
  }

  function liveChampionId(player: any): number | null {
    const raw: string = player?.rawChampionName ?? "";
    const alias = raw.split("_").pop() ?? "";
    return championByAlias.get(alias.toLowerCase())?.id ?? null;
  }

  function liveTeams(players: any[]): { order: any[]; chaos: any[] } {
    return {
      order: players.filter((p) => p.team === "ORDER"),
      chaos: players.filter((p) => p.team === "CHAOS"),
    };
  }

  function formatGameTime(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  function myTeamPicks(session: any): { cellId: number; championId: number }[] {
    return (session?.myTeam ?? []).map((m: any) => ({ cellId: m.cellId, championId: m.championId }));
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
    loadNotes();
    loadGoals();
    loadPerkMeta();
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
      <nav class="league-tabs" aria-label={$t("league.nav") as string}>
        {#each TAB_IDS as id (id)}
          <button class="league-tab" class:on={tab === id} onclick={() => (tab = id)} aria-current={tab === id}>
            {$t(`league.tab_${id}`)}
          </button>
        {/each}
      </nav>

      {#if tab === "overview"}
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
      {#if actionError}
        <div class="action-error" role="alert">{actionError}</div>
      {/if}
      {#if phase === "ChampSelect" && champSelect}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.champ_select_title")}</h3>
            <span class="phase-tag">{phaseLabel(phase)}</span>
          </div>
          <div class="team-picks">
            {#each myTeamPicks(champSelect) as pick (pick.cellId)}
              {#if pick.championId > 0}
                <img class="champ-icon" src={`${CDRAGON}/champion-icons/${pick.championId}.png`} alt={championById.get(pick.championId)?.name ?? ""} title={championById.get(pick.championId)?.name ?? ""} loading="lazy" />
              {:else}
                <div class="champ-icon champ-empty" aria-hidden="true"></div>
              {/if}
            {/each}
          </div>
          {#if champSelect.benchEnabled}
            <div class="bench-row">
              <span class="bench-label">{$t("league.bench_title")}</span>
              <div class="bench-champs">
                {#each champSelect.benchChampions ?? [] as bc (bc.championId)}
                  <button
                    class="bench-swap"
                    onclick={() => action("league_bench_swap", { championId: bc.championId })}
                    title={championById.get(bc.championId)?.name ?? ""}
                    aria-label={`${$t("league.swap")} ${championById.get(bc.championId)?.name ?? bc.championId}`}
                  >
                    <img class="champ-icon" src={`${CDRAGON}/champion-icons/${bc.championId}.png`} alt="" loading="lazy" />
                  </button>
                {/each}
              </div>
              <button class="button" onclick={() => action("league_reroll")}>{$t("league.reroll")}</button>
            </div>
          {/if}
        </section>
      {:else if phase === "InProgress" && liveGame?.stats}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.live_title")}</h3>
            <span class="phase-tag">{formatGameTime(liveGame.stats.gameTime ?? 0)}</span>
          </div>
          {#if Array.isArray(liveGame.players)}
            {@const teams = liveTeams(liveGame.players)}
            <div class="live-teams">
              {#each [teams.order, teams.chaos] as team, ti}
                <div class="live-team">
                  {#each team as p (p.riotId ?? p.summonerName ?? p.championName)}
                    {@const cid = liveChampionId(p)}
                    <div class="live-row" class:me={(p.riotId ?? p.summonerName) === liveGame.activePlayer}>
                      {#if cid}
                        <img class="champ-icon small" src={`${CDRAGON}/champion-icons/${cid}.png`} alt="" loading="lazy" />
                      {:else}
                        <div class="champ-icon small champ-empty" aria-hidden="true"></div>
                      {/if}
                      <span class="live-name">{p.championName}</span>
                      <span class="live-kda">{p.scores?.kills ?? 0}/{p.scores?.deaths ?? 0}/{p.scores?.assists ?? 0}</span>
                      {#if p.isDead && p.respawnTimer > 0}
                        <span class="live-respawn">{$t("league.respawn_in")} {Math.ceil(p.respawnTimer)}s</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {:else}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.lobby_title")}</h3>
            {#if phase && phase !== "None"}
              <span class="phase-tag">{phaseLabel(phase)}</span>
            {/if}
          </div>
          {#if phase === "ReadyCheck"}
            <div class="lobby-actions">
              <button class="button primary" onclick={() => action("league_accept_ready_check")}>{$t("league.accept_now")}</button>
            </div>
          {:else if phase === "Matchmaking"}
            <div class="lobby-actions">
              <span class="searching-hint">{$t("league.searching")}</span>
              <button class="button" onclick={() => action("league_stop_matchmaking")}>{$t("league.stop_queue")}</button>
            </div>
          {:else if phase === "Lobby" && lobby}
            <div class="lobby-actions">
              <button class="button primary" onclick={() => action("league_start_matchmaking")}>{$t("league.start_queue")}</button>
              <button class="button" onclick={() => action("league_leave_lobby")}>{$t("league.leave_lobby")}</button>
            </div>
          {:else if phase === "EndOfGame" || phase === "PreEndOfGame" || phase === "WaitingForStats"}
            <div class="lobby-actions">
              <button class="button primary" onclick={() => action("league_play_again")}>{$t("league.play_again")}</button>
            </div>
          {:else if queues.length > 0}
            <div class="queue-grid">
              {#each queues as q (q.id)}
                <button class="button" onclick={() => action("league_create_lobby", { queueId: q.id })}>{q.shortName || q.name}</button>
              {/each}
            </div>
          {:else}
            <p class="empty-hint">{$t("league.lobby_hint")}</p>
          {/if}
        </section>
      {/if}
      {/if}

      {#if tab === "analysis"}
        {#if analysis}
          <section class="card">
            <div class="card-head">
              <h3>{$t("league.win_title")}</h3>
              <button class="button" onclick={loadAnalysis} disabled={analysisLoading}>{$t("league.refresh")}</button>
            </div>
            <div class="winbar-wrap">
              <div class="winbar" role="img" aria-label={`${$t("league.win_allies")} ${analysis.winProbability}%`}>
                <div class="winbar-fill" style={`width:${analysis.winProbability}%`}></div>
                <div class="winbar-range" style={`left:${analysis.winLow}%;width:${Math.max(analysis.winHigh - analysis.winLow, 0)}%`}></div>
              </div>
              <div class="winbar-legend">
                <span class="win-value">{analysis.winProbability}%</span>
                <span class="win-range">{$t("league.win_interval")} {analysis.winLow}%–{analysis.winHigh}%</span>
              </div>
            </div>
            <p class="win-note">
              {$t("league.win_gap")} {analysis.ratingGap > 0 ? "+" : ""}{analysis.ratingGap} · {analysis.knownPlayers}/{analysis.totalPlayers} {$t("league.win_known")}
            </p>
            <p class="win-disclaimer">{$t("league.win_disclaimer")}</p>
            <div class="chat-send">
              <input
                class="input-text"
                placeholder={$t("league.chat_placeholder") as string}
                bind:value={chatPreview}
              />
              <button class="button" onclick={() => (chatPreview = buildChatSummary())}>{$t("league.chat_build")}</button>
              <button class="button primary" onclick={sendChatSummary} disabled={chatSending || !chatPreview.trim()}>{$t("league.chat_send")}</button>
            </div>
            {#if analysis.premades?.length}
              <div class="premade-row">
                <span class="bench-label">{$t("league.premades")}</span>
                {#each analysis.premades as group (group.label)}
                  <span class="scout-tag">{group.label}: {group.puuids.length} {$t("league.players")}</span>
                {/each}
              </div>
            {/if}
          </section>
        {:else}
          <div class="guard-card">
            <p>{$t("league.win_unavailable")}</p>
            <button class="button" onclick={loadAnalysis} disabled={analysisLoading}>{$t("league.refresh")}</button>
          </div>
        {/if}

      {#if (phase === "ChampSelect" || phase === "InProgress") && scoutPlayers.length > 0}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.scout_title")}</h3>
            <button class="button" onclick={loadScouting} disabled={scoutLoading}>{$t("league.refresh")}</button>
          </div>
          <div class="scout-teams">
            {#each scoutGroups() as group (group.label)}
              <div class="scout-team">
                <h4 class="scout-team-title" class:enemy={!group.ally}>{group.label}</h4>
                {#if group.list.length === 0}
                  <p class="empty-hint">{$t("league.scout_enemies_hidden")}</p>
                {:else}
                  {#each group.list as p (p.puuid || String(p.cellId))}
                    {@const r = p.puuid ? scoutReports[p.puuid] : null}
                    <div class="scout-row">
                      <div class="scout-main">
                        {#if p.championId > 0}
                          <img class="champ-icon small" src={`${CDRAGON}/champion-icons/${p.championId}.png`} alt="" title={championById.get(p.championId)?.name ?? ""} loading="lazy" />
                        {:else}
                          <div class="champ-icon small champ-empty" aria-hidden="true"></div>
                        {/if}
                        <div class="scout-id">
                          <span class="scout-name">{p.gameName || "—"}{#if p.tagLine}<span class="tag">#{p.tagLine}</span>{/if}</span>
                          <span class="scout-rank">{r ? rankLabel(r.solo) : "…"}</span>
                        </div>
                        {#if r?.stats?.games > 0}
                          <div class="scout-stats">
                            <span class="scout-wr" class:good={r.stats.winrate >= 55} class:bad={r.stats.winrate <= 45}>{r.stats.winrate}% WR</span>
                            <span class="scout-kda">
                              {r.stats.kda} KDA{#if r.impact !== null && r.impact !== undefined} · <span class="impact" title={$t("league.impact_hint") as string}>{r.impact}</span>{/if}
                            </span>
                          </div>
                        {:else if r?.privateProfile}
                          <span class="scout-private">{$t("league.scout_private")}</span>
                        {/if}
                        {#if p.puuid}
                          <button class="note-toggle" class:has-note={(notes[p.puuid] ?? "").trim() !== ""} onclick={() => { openNotes = { ...openNotes, [p.puuid]: !openNotes[p.puuid] }; }} aria-label={$t("league.scout_note_placeholder") as string} aria-expanded={openNotes[p.puuid] ?? false}>✎</button>
                        {/if}
                      </div>
                      {#if r?.stats?.topChampions?.length}
                        <div class="scout-champs">
                          {#each r.stats.topChampions as tc (tc.championId)}
                            <span class="scout-champ">
                              <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${tc.championId}.png`} alt="" title={championById.get(tc.championId)?.name ?? ""} loading="lazy" />
                              <span class="scout-champ-record">{tc.wins}/{tc.games}</span>
                            </span>
                          {/each}
                          {#if r?.stats?.insights?.length}
                            {#each r.stats.insights as tag (tag)}
                              <span class="scout-tag">{$t(TAG_KEYS[tag] ?? tag)}</span>
                            {/each}
                          {/if}
                        </div>
                      {/if}
                      {#if p.puuid && (openNotes[p.puuid] || (notes[p.puuid] ?? "").trim() !== "")}
                        <input class="input-text note-input" placeholder={$t("league.scout_note_placeholder") as string} value={notes[p.puuid] ?? ""} onchange={(e) => saveNote(p.puuid, e.currentTarget.value)} />
                      {/if}
                    </div>
                  {/each}
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}
      {/if}

      {#if tab === "meta"}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.runes_title")}</h3>
            {#if champSelectChampionId > 0}
              <span class="phase-tag">{championById.get(champSelectChampionId)?.name ?? champSelectChampionId}</span>
            {/if}
          </div>
          <p class="win-disclaimer">{$t("league.runes_desc")}</p>
          <div class="action-row">
            <div class="action-col">
              <span class="action-label">{$t("league.runes_auto")}</span>
              <span class="action-hint">{$t("league.runes_auto_desc")}</span>
            </div>
            <button
              class="toggle"
              class:on={settings?.league?.auto_runes}
              onclick={() => toggleLeagueFlag("auto_runes")}
              role="switch"
              aria-checked={settings?.league?.auto_runes ?? false}
              aria-label={$t("league.runes_auto") as string}
            >
              <span class="toggle-knob"></span>
            </button>
          </div>
          {#if runeError}
            <p class="action-error" role="alert">{runeError}</p>
          {/if}
          {#if runePages.length > 0}
            <div class="rune-list">
              {#each runePages as page, i (page.recommendationId ?? i)}
                <div class="rune-card" class:applied={appliedRuneIndex === i}>
                  <div class="rune-head">
                    <span class="rune-keystone">{page.keystoneName ?? page.keystoneId}</span>
                    {#if page.isDefault}
                      <span class="scout-tag">{$t("league.runes_default")}</span>
                    {/if}
                  </div>
                  <div class="rune-perks">
                    {#each page.selectedPerkIds as perkId (perkId)}
                      <img
                        class="perk-icon"
                        src={`${CDRAGON}/perk-images/styles/${perkId}.png`}
                        alt=""
                        title={perkName(perkId)}
                        loading="lazy"
                        onerror={(e) => { (e.currentTarget as HTMLImageElement).style.visibility = "hidden"; }}
                      />
                    {/each}
                  </div>
                  <div class="rune-foot">
                    <span class="dim">{$t("league.runes_spells")}: {(page.summonerSpellIds ?? []).map(spellName).join(" + ")}</span>
                    <button class="button" onclick={() => applyRunePage(i)} disabled={runeApplying}>
                      {$t("league.runes_apply")}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="empty-hint">{$t("league.runes_empty")}</p>
          {/if}
        </section>

        <section class="card">
          <div class="card-head">
            <h3>{$t("league.tiers_title")}</h3>
            <div class="tier-controls">
              <select class="select-role" bind:value={tierPosition} aria-label={$t("league.tiers_position") as string}>
                {#each TIER_POSITIONS as pos (pos)}
                  <option value={pos}>{$t(`league.role_${pos.toLowerCase()}`)}</option>
                {/each}
              </select>
              <button class="button" onclick={loadTiers} disabled={tiersLoading}>{$t("league.refresh")}</button>
            </div>
          </div>
          <p class="win-disclaimer">{$t("league.tiers_desc")}</p>
          {#if tiersError}
            <p class="action-error" role="alert">{tiersError}</p>
          {:else if tierRows.length > 0}
            <div class="champ-table">
              {#each tierRows as row (row.championId)}
                <div class="champ-row">
                  <span class={`tier-badge tier-${row.tier}`}>{tierLabel(row.tier)}</span>
                  <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${row.championId}.png`} alt="" loading="lazy" />
                  <span class="champ-row-name">{championById.get(row.championId)?.name ?? row.championId}</span>
                  <span class="champ-row-wr" class:good={row.winRate >= 52} class:bad={row.winRate <= 48}>{row.winRate}%</span>
                  <span class="champ-row-games dim">{$t("league.tiers_pick")} {row.pickRate}%</span>
                  <span class="champ-row-kda dim">{$t("league.tiers_ban")} {row.banRate}%</span>
                </div>
              {/each}
            </div>
          {:else}
            <p class="empty-hint">{tiersLoading ? $t("league.searching_player") : $t("league.tiers_empty")}</p>
          {/if}
        </section>
      {/if}

      {#if tab === "search"}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.search_title")}</h3>
          </div>
          <form class="search-form" onsubmit={(e) => { e.preventDefault(); runSearch(); }}>
            <input
              class="input-text"
              placeholder={$t("league.search_placeholder") as string}
              bind:value={searchQuery}
              spellcheck="false"
            />
            <button class="button primary" type="submit" disabled={searchLoading}>
              {searchLoading ? $t("league.searching_player") : $t("league.search_button")}
            </button>
          </form>
          {#if searchError}
            <p class="action-error" role="alert">{searchError}</p>
          {:else}
            <p class="win-disclaimer">{$t("league.search_hint")}</p>
          {/if}
        </section>

        {#if searchResult}
          {@const s = searchResult.summoner}
          {@const r = searchResult.report}
          <section class="profile-card">
            <img class="profile-icon" src={`${CDRAGON}/profile-icons/${s.profileIconId}.jpg`} alt="" loading="lazy" />
            <div class="profile-info">
              <span class="profile-name">{s.gameName}<span class="tag">#{s.tagLine}</span></span>
              <span class="profile-level">{$t("league.level")} {s.summonerLevel ?? "—"}</span>
            </div>
            <div class="ranked-chips">
              <div class="ranked-chip">
                <span class="ranked-queue">{$t("league.ranked_solo")}</span>
                <span class="ranked-value">{rankLabel(r?.solo)}</span>
              </div>
              <div class="ranked-chip">
                <span class="ranked-queue">{$t("league.ranked_flex")}</span>
                <span class="ranked-value">{rankLabel(r?.flex)}</span>
              </div>
            </div>
          </section>

          {#if r?.stats?.games > 0}
            <section class="card">
              <div class="card-head"><h3>{$t("league.search_recent")}</h3></div>
              <div class="stat-grid">
                <div class="stat-cell">
                  <span class="stat-value" class:good={r.stats.winrate >= 55} class:bad={r.stats.winrate <= 45}>{r.stats.winrate}%</span>
                  <span class="stat-label">{$t("league.stat_winrate")} ({r.stats.games})</span>
                </div>
                <div class="stat-cell">
                  <span class="stat-value">{r.stats.kda}</span>
                  <span class="stat-label">KDA</span>
                </div>
                <div class="stat-cell">
                  <span class="stat-value">{r.stats.streak?.length ?? 0}</span>
                  <span class="stat-label">{r.stats.streak?.win ? $t("league.tag_hot_streak") : $t("league.tag_cold_streak")}</span>
                </div>
                {#if r.impact !== null && r.impact !== undefined}
                  <div class="stat-cell">
                    <span class="stat-value">{r.impact}<span class="dim">/10</span></span>
                    <span class="stat-label">{$t("league.impact_label")} ({r.impactGames})</span>
                  </div>
                {/if}
              </div>
              {#if r.stats.insights?.length}
                <div class="scout-champs">
                  {#each r.stats.insights as tagId (tagId)}
                    <span class="scout-tag">{$t(TAG_KEYS[tagId] ?? tagId)}</span>
                  {/each}
                </div>
              {/if}
            </section>
          {/if}

          {#if searchResult.champions?.length}
            <section class="card">
              <div class="card-head">
                <h3>{$t("league.search_champions")}</h3>
                <span class="phase-tag">{$t("league.search_champions_hint")}</span>
              </div>
              <div class="champ-table">
                {#each searchResult.champions as ch (ch.championId)}
                  <div class="champ-row">
                    <img class="champ-icon small" src={`${CDRAGON}/champion-icons/${ch.championId}.png`} alt="" loading="lazy" />
                    <span class="champ-row-name">{championById.get(ch.championId)?.name ?? ch.championId}</span>
                    <span class="champ-row-games">{ch.games} {$t("league.games_short")}</span>
                    <span class="champ-row-wr" class:good={ch.winrate >= 55} class:bad={ch.winrate <= 45}>{ch.winrate}%</span>
                    <span class="champ-row-kda">{ch.kda} KDA</span>
                    <span class="champ-row-cs dim">{ch.csPerMin}/m</span>
                  </div>
                {/each}
              </div>
            </section>
          {/if}

          {#if r?.mastery?.length}
            <section class="card">
              <div class="card-head"><h3>{$t("league.search_mastery")}</h3></div>
              <div class="scout-champs">
                {#each r.mastery as m (m.championId)}
                  <span class="champ-chip">
                    <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${m.championId}.png`} alt="" loading="lazy" />
                    {championById.get(m.championId)?.name ?? m.championId}
                    <span class="dim">M{m.championLevel} · {Math.round((m.championPoints ?? 0) / 1000)}k</span>
                  </span>
                {/each}
              </div>
            </section>
          {/if}

          {#if jungleReport}
            <section class="card">
              <div class="card-head">
                <h3>{$t("league.jungle_title")}</h3>
                <span class="phase-tag">{jungleReport.analysedGames} {$t("league.games_short")}</span>
              </div>
              {#if jungleReport.analysedGames > 0}
                <div class="zone-bars">
                  {#each [["top", jungleReport.zones.top], ["mid", jungleReport.zones.mid], ["bot", jungleReport.zones.bot]] as [zone, pct] (zone)}
                    <div class="zone-row">
                      <span class="zone-name">{$t(`league.zone_${zone}`)}</span>
                      <div class="goal-bar"><div class="goal-fill" style={`width:${pct}%`}></div></div>
                      <span class="goal-value">{pct}%</span>
                    </div>
                  {/each}
                </div>
                <p class="win-note">
                  {$t(`league.pref_${jungleReport.preference}`)} · {$t("league.jungle_invade")} {jungleReport.invadeRate}% · {$t("league.jungle_gank3")} {jungleReport.level3GankRate}%
                </p>
              {:else}
                <p class="empty-hint">{$t("league.jungle_empty")}</p>
              {/if}
            </section>
          {/if}
        {/if}

        <section class="card">
          <div class="card-head">
            <h3>{$t("league.duos_title")}</h3>
            <button class="button" onclick={loadDuos} disabled={duosLoading}>{$t("league.refresh")}</button>
          </div>
          <p class="win-disclaimer">{$t("league.duos_desc")}</p>
          {#if duos?.duos?.length}
            <div class="champ-table">
              {#each duos.duos as d (d.puuid)}
                <div class="champ-row">
                  <span class="champ-row-name">{d.gameName ?? "—"}{#if d.tagLine}<span class="dim">#{d.tagLine}</span>{/if}</span>
                  <span class="champ-row-games">{d.games} {$t("league.games_short")}</span>
                  <span class="champ-row-wr" class:good={d.winrate >= 55} class:bad={d.winrate <= 45}>{d.winrate}%</span>
                  <span class="champ-row-kda dim">{$t("league.duos_score")} {d.score}%</span>
                </div>
              {/each}
            </div>
          {:else if duos}
            <p class="empty-hint">{$t("league.duos_empty")}</p>
          {/if}
        </section>
      {/if}

      {#if tab === "live"}
        {#if liveMetrics?.players?.length}
          <section class="card">
            <div class="card-head">
              <h3>{$t("league.gold_title")}</h3>
              <span class="phase-tag">{formatGameTime(liveMetrics.gameTime ?? 0)}</span>
            </div>
            <div class="gold-summary">
              <span class="gold-team">{$t("league.your_team")}: <strong>{liveMetrics.teamGold?.[myTeam] ?? 0}</strong></span>
              <span class="gold-diff" class:good={teamGoldLead > 0} class:bad={teamGoldLead < 0}>
                {teamGoldLead > 0 ? "+" : ""}{teamGoldLead}
              </span>
              <span class="gold-team">{$t("league.enemy_team")}: <strong>{liveMetrics.teamGold?.[enemyTeam] ?? 0}</strong></span>
            </div>
            <div class="metric-table" role="table">
              <div class="metric-head" role="row">
                <span role="columnheader">{$t("league.col_player")}</span>
                <span role="columnheader">KDA</span>
                <span role="columnheader">CS</span>
                <span role="columnheader">{$t("league.col_gold")}</span>
                <span role="columnheader">{$t("league.col_diff")}</span>
              </div>
              {#each liveMetrics.players as row (row.riotId)}
                <div class="metric-row" role="row" class:self={row.isSelf}>
                  <span class="metric-name" role="cell">
                    <span class="pos-chip">{(row.position ?? "?").slice(0, 3)}</span>
                    {row.championName}
                  </span>
                  <span role="cell">{row.kills}/{row.deaths}/{row.assists}</span>
                  <span role="cell">{row.cs} <span class="dim">({row.csPerMin}/m)</span></span>
                  <span role="cell">{row.itemGold}</span>
                  <span role="cell" class="diff" class:good={(row.goldDiff ?? 0) > 0} class:bad={(row.goldDiff ?? 0) < 0}>
                    {#if row.goldDiff !== undefined && row.goldDiff !== null}
                      {row.goldDiff > 0 ? "+" : ""}{row.goldDiff}g
                      <span class="dim">{(row.csDiff ?? 0) > 0 ? "+" : ""}{Math.round(row.csDiff ?? 0)}cs</span>
                    {:else}—{/if}
                  </span>
                </div>
              {/each}
            </div>
          </section>

          {#if selfRow}
            <section class="card">
              <div class="card-head">
                <h3>{$t("league.goals_live")}</h3>
                <span class="phase-tag">{selfRow.position ?? "?"}</span>
              </div>
              <div class="goal-list">
                {#each liveGoals as goal (goal.key)}
                  <div class="goal-row">
                    <span class="goal-name">{$t(goal.labelKey)}</span>
                    <div class="goal-bar"><div class="goal-fill" class:met={goal.ratio >= 1} style={`width:${Math.min(goal.ratio * 100, 100)}%`}></div></div>
                    <span class="goal-value" class:met={goal.ratio >= 1}>{goal.current} <span class="dim">/ {goal.target}</span></span>
                  </div>
                {/each}
              </div>
            </section>
          {/if}
        {:else}
          <div class="guard-card">
            <p>{$t("league.gold_unavailable")}</p>
          </div>
        {/if}
      {/if}

      {#if tab === "goals"}
        <section class="card">
          <div class="card-head">
            <h3>{$t("league.goals_title")}</h3>
            <select class="select-role" bind:value={goalRole} aria-label={$t("league.goals_role") as string}>
              {#each ROLES as r (r)}
                <option value={r}>{$t(`league.role_${r.toLowerCase()}`)}</option>
              {/each}
            </select>
          </div>
          <p class="win-disclaimer">{$t("league.goals_desc")}</p>
          <div class="goal-config">
            {#each GOAL_FIELDS as field (field.key)}
              <label class="goal-field">
                <span class="goal-field-label">{$t(field.labelKey)}</span>
                <input
                  type="number"
                  class="input-text"
                  min="0"
                  step={field.step}
                  value={goalValue(goalRole, field.key)}
                  onchange={(e) => setGoal(goalRole, field.key, Number(e.currentTarget.value))}
                />
              </label>
            {/each}
          </div>
          <button class="button" onclick={() => resetGoals(goalRole)}>{$t("league.goals_reset")}</button>
        </section>
      {/if}

      {#if tab === "automation"}
      <section class="card">
        <div class="card-head">
          <h3>{$t("league.automation_title")}</h3>
        </div>
        <div class="action-row">
          <div class="action-col">
            <span class="action-label">{$t("league.auto_accept")}</span>
            <span class="action-hint">{$t("league.auto_accept_desc")}</span>
          </div>
          <button class="toggle" class:on={autoAccept} onclick={toggleAutoAccept} role="switch" aria-checked={autoAccept} aria-label={$t("league.auto_accept") as string}>
            <span class="toggle-knob"></span>
          </button>
        </div>
        <div class="divider"></div>
        <div class="action-row">
          <div class="action-col">
            <span class="action-label">{$t("league.auto_pick")}</span>
            <span class="action-hint">{$t("league.auto_pick_desc")}</span>
          </div>
          <button class="toggle" class:on={settings?.league?.auto_pick} onclick={() => toggleLeagueFlag("auto_pick")} role="switch" aria-checked={settings?.league?.auto_pick ?? false} aria-label={$t("league.auto_pick") as string}>
            <span class="toggle-knob"></span>
          </button>
        </div>
        {#if settings?.league?.auto_pick}
          <div class="champ-list-block">
            <span class="list-label">{$t("league.pick_list")} <span class="list-hint">({$t("league.list_hint")})</span></span>
            <div class="champ-chips">
              {#each listFor("pick") as id (id)}
                <span class="champ-chip">
                  <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${id}.png`} alt="" loading="lazy" />
                  {championById.get(id)?.name ?? id}
                  <button class="chip-remove" onclick={() => removeFromList("pick", id)} aria-label={`${$t("league.remove")} ${championById.get(id)?.name ?? id}`}>×</button>
                </span>
              {/each}
            </div>
            <div class="champ-search">
              <input type="text" class="input-text" placeholder={$t("league.search_champion") as string} bind:value={pickSearch} />
              {#if searchResults(pickSearch, "pick").length > 0}
                <div class="search-results">
                  {#each searchResults(pickSearch, "pick") as c (c.id)}
                    <button class="search-result" onclick={() => addToList("pick", c.id)}>
                      <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${c.id}.png`} alt="" loading="lazy" />
                      {c.name}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}
        <div class="divider"></div>
        <div class="action-row">
          <div class="action-col">
            <span class="action-label">{$t("league.auto_ban")}</span>
            <span class="action-hint">{$t("league.auto_ban_desc")}</span>
          </div>
          <button class="toggle" class:on={settings?.league?.auto_ban} onclick={() => toggleLeagueFlag("auto_ban")} role="switch" aria-checked={settings?.league?.auto_ban ?? false} aria-label={$t("league.auto_ban") as string}>
            <span class="toggle-knob"></span>
          </button>
        </div>
        {#if settings?.league?.auto_ban}
          <div class="champ-list-block">
            <span class="list-label">{$t("league.ban_list")} <span class="list-hint">({$t("league.list_hint")})</span></span>
            <div class="champ-chips">
              {#each listFor("ban") as id (id)}
                <span class="champ-chip">
                  <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${id}.png`} alt="" loading="lazy" />
                  {championById.get(id)?.name ?? id}
                  <button class="chip-remove" onclick={() => removeFromList("ban", id)} aria-label={`${$t("league.remove")} ${championById.get(id)?.name ?? id}`}>×</button>
                </span>
              {/each}
            </div>
            <div class="champ-search">
              <input type="text" class="input-text" placeholder={$t("league.search_champion") as string} bind:value={banSearch} />
              {#if searchResults(banSearch, "ban").length > 0}
                <div class="search-results">
                  {#each searchResults(banSearch, "ban") as c (c.id)}
                    <button class="search-result" onclick={() => addToList("ban", c.id)}>
                      <img class="champ-icon tiny" src={`${CDRAGON}/champion-icons/${c.id}.png`} alt="" loading="lazy" />
                      {c.name}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}
        {#if settings?.league?.auto_pick}
          <div class="divider"></div>
          <div class="action-row">
            <div class="action-col">
              <span class="action-label">{$t("league.auto_lock")}</span>
              <span class="action-hint">{$t("league.auto_lock_desc")}</span>
            </div>
            <button class="toggle" class:on={settings?.league?.auto_lock} onclick={() => toggleLeagueFlag("auto_lock")} role="switch" aria-checked={settings?.league?.auto_lock ?? false} aria-label={$t("league.auto_lock") as string}>
              <span class="toggle-knob"></span>
            </button>
          </div>
        {/if}
      </section>

      {/if}

      {#if tab === "history"}
      <section class="history-section">
        <div class="history-head">
          <h3>{$t("league.history_title")}</h3>
          <button class="button" onclick={loadHistory} disabled={loadingHistory}>{$t("league.refresh")}</button>
        </div>
        {#if games.length === 0}
          <p class="empty-hint">{$t("league.history_empty")}</p>
        {:else}
          <div class="game-list">
            {#each games as game (game.gameId)}
              {@const p = playerStats(game)}
              <div class="game-row">
                <img class="champ-icon" src={`${CDRAGON}/champion-icons/${p.championId}.png`} alt="" loading="lazy" />
                <div class="game-info">
                  <span class="game-result" class:win={p.win} class:loss={!p.win}>{p.win ? $t("league.victory") : $t("league.defeat")}</span>
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
    background: var(--success);
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

  .card {
    display: flex;
    flex-direction: column;
    padding: var(--padding);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .card-head h3 {
    margin: 0;
    font-size: 15px;
  }

  .phase-tag {
    font-size: 12px;
    color: var(--gray);
    padding: 3px 10px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: 999px;
  }

  .action-error {
    font-size: 12.5px;
    color: var(--danger);
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: calc(var(--border-radius) - 2px);
  }

  .lobby-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .searching-hint {
    font-size: 13px;
    color: var(--gray);
  }

  .queue-grid {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .empty-hint {
    color: var(--gray);
    font-size: 13px;
    margin: 0;
  }

  .team-picks {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .bench-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    flex-wrap: wrap;
  }

  .bench-label {
    font-size: 12.5px;
    color: var(--gray);
  }

  .bench-champs {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .bench-swap {
    padding: 0;
    background: none;
    border: 2px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    line-height: 0;
  }

  .bench-swap:hover,
  .bench-swap:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  .live-teams {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  @media (max-width: 560px) {
    .live-teams {
      grid-template-columns: 1fr;
    }
  }

  .live-team {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .live-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border-radius: calc(var(--border-radius) - 4px);
    min-width: 0;
  }

  .live-row.me {
    background: var(--accent-soft, var(--button));
  }

  .live-name {
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .live-kda {
    margin-left: auto;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--gray);
  }

  .live-respawn {
    font-size: 11.5px;
    color: var(--danger);
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

  .champ-list-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
  }

  .list-label {
    font-size: 12.5px;
    color: var(--gray);
  }

  .list-hint {
    font-size: 11.5px;
  }

  .champ-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .champ-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 6px 3px 3px;
    font-size: 12.5px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: 999px;
  }

  .chip-remove {
    background: none;
    border: none;
    color: var(--gray);
    font-size: 14px;
    cursor: pointer;
    padding: 0 3px;
    line-height: 1;
  }

  .chip-remove:hover,
  .chip-remove:focus-visible {
    color: var(--danger);
    outline: none;
  }

  .champ-search {
    position: relative;
    max-width: 260px;
  }

  .input-text {
    width: 100%;
    padding: 7px 10px;
    font-size: 13px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 2px);
    color: var(--text);
  }

  .input-text:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  .search-results {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: calc(var(--border-radius) - 2px);
    overflow: hidden;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
  }

  .search-result {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    font-size: 13px;
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }

  .search-result:hover,
  .search-result:focus-visible {
    background: var(--button);
    outline: none;
  }

  .league-tabs {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    padding-bottom: 2px;
    border-bottom: 1px solid var(--border);
  }

  .league-tab {
    padding: 6px 12px;
    font-size: 12.5px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--gray);
    cursor: pointer;
  }

  .league-tab:hover {
    color: var(--text);
  }

  .league-tab.on {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .league-tab:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }

  .winbar-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .winbar {
    position: relative;
    height: 12px;
    border-radius: 999px;
    background: var(--button);
    border: 1px solid var(--input-border);
    overflow: hidden;
  }

  .winbar-fill {
    height: 100%;
    background: var(--accent);
  }

  .winbar-range {
    position: absolute;
    top: 0;
    height: 100%;
    background: var(--accent);
    opacity: 0.25;
  }

  .winbar-legend {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .win-value {
    font-size: 22px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .win-range,
  .win-note {
    font-size: 12px;
    color: var(--gray);
  }

  .win-note {
    margin: 8px 0 0;
  }

  .win-disclaimer {
    margin: 4px 0 0;
    font-size: 11.5px;
    color: var(--gray);
    line-height: 1.45;
  }

  .premade-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 10px;
  }

  .gold-summary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    margin-bottom: 10px;
    font-size: 13px;
    flex-wrap: wrap;
  }

  .gold-team {
    color: var(--gray);
  }

  .gold-diff {
    font-size: 17px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .gold-diff.good,
  .diff.good {
    color: var(--success);
  }

  .gold-diff.bad,
  .diff.bad {
    color: var(--danger);
  }

  .metric-table {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-x: auto;
  }

  .metric-head,
  .metric-row {
    display: grid;
    grid-template-columns: minmax(120px, 1.6fr) 68px 92px 64px minmax(110px, 1fr);
    gap: 8px;
    align-items: center;
    padding: 5px 7px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    min-width: 460px;
  }

  .metric-head {
    color: var(--gray);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .metric-row {
    border-radius: calc(var(--border-radius) - 4px);
    background: var(--button);
  }

  .metric-row.self {
    background: var(--accent-soft, var(--surface));
  }

  .metric-name {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pos-chip {
    font-size: 9.5px;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface);
    color: var(--gray);
    letter-spacing: 0.03em;
  }

  .dim {
    color: var(--gray);
  }

  .goal-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .goal-row {
    display: grid;
    grid-template-columns: minmax(80px, 1fr) minmax(90px, 2fr) minmax(90px, 1fr);
    gap: 10px;
    align-items: center;
    font-size: 12.5px;
  }

  .goal-name {
    color: var(--gray);
  }

  .goal-bar {
    height: 8px;
    border-radius: 999px;
    background: var(--button);
    border: 1px solid var(--input-border);
    overflow: hidden;
  }

  .goal-fill {
    height: 100%;
    background: var(--gray);
  }

  .goal-fill.met {
    background: var(--success);
  }

  .goal-value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .goal-value.met {
    color: var(--success);
  }

  .goal-config {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 10px;
    margin: 10px 0;
  }

  .goal-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .goal-field-label {
    font-size: 12px;
    color: var(--gray);
  }

  .select-role {
    padding: 5px 10px;
    font-size: 12.5px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 2px);
    color: var(--text);
  }

  .search-form {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .search-form .input-text {
    flex: 1;
    min-width: 180px;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 10px;
  }

  .stat-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 2px);
  }

  .stat-value {
    font-size: 19px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .stat-value.good {
    color: var(--success);
  }

  .stat-value.bad {
    color: var(--danger);
  }

  .stat-label {
    font-size: 11.5px;
    color: var(--gray);
  }

  .champ-table {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .champ-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    background: var(--button);
    border-radius: calc(var(--border-radius) - 4px);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
  }

  .champ-row-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .champ-row-games,
  .champ-row-kda,
  .champ-row-cs {
    color: var(--gray);
    flex-shrink: 0;
  }

  .champ-row-wr {
    flex-shrink: 0;
    font-weight: 600;
  }

  .champ-row-wr.good {
    color: var(--success);
  }

  .champ-row-wr.bad {
    color: var(--danger);
  }

  .zone-bars {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .zone-row {
    display: grid;
    grid-template-columns: 60px 1fr 52px;
    gap: 10px;
    align-items: center;
    font-size: 12.5px;
  }

  .zone-name {
    color: var(--gray);
  }

  .chat-send {
    display: flex;
    gap: 8px;
    margin-top: 12px;
    flex-wrap: wrap;
  }

  .chat-send .input-text {
    flex: 1;
    min-width: 180px;
  }

  .impact {
    color: var(--accent);
    font-weight: 600;
  }

  .rune-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
  }

  .rune-card {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 9px 11px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 2px);
  }

  .rune-card.applied {
    border-color: var(--accent);
  }

  .rune-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .rune-keystone {
    font-size: 13px;
    font-weight: 600;
  }

  .rune-perks {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }

  .perk-icon {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--surface);
  }

  .rune-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
    flex-wrap: wrap;
  }

  .tier-controls {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .tier-badge {
    flex-shrink: 0;
    min-width: 30px;
    text-align: center;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .tier-badge.tier-1 {
    color: var(--on-accent);
    background: var(--accent);
    border-color: transparent;
  }

  .tier-badge.tier-2 {
    color: var(--success);
    border-color: var(--success);
  }

  .tier-badge.tier-3 {
    color: var(--text);
  }

  .scout-teams {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  @media (max-width: 620px) {
    .scout-teams {
      grid-template-columns: 1fr;
    }
  }

  .scout-team {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .scout-team-title {
    margin: 0 0 2px;
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--gray);
  }

  .scout-team-title.enemy {
    color: var(--danger);
  }

  .scout-row {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 7px 9px;
    background: var(--button);
    border: 1px solid var(--input-border);
    border-radius: calc(var(--border-radius) - 3px);
  }

  .scout-main {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .scout-id {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }

  .scout-name {
    font-size: 12.5px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scout-name .tag {
    color: var(--gray);
    font-weight: 400;
  }

  .scout-rank {
    font-size: 11px;
    color: var(--gray);
  }

  .scout-stats {
    display: flex;
    flex-direction: column;
    gap: 1px;
    align-items: flex-end;
    flex-shrink: 0;
  }

  .scout-wr {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .scout-wr.good {
    color: var(--success);
  }

  .scout-wr.bad {
    color: var(--danger);
  }

  .scout-kda {
    font-size: 11px;
    color: var(--gray);
    font-variant-numeric: tabular-nums;
  }

  .scout-private {
    font-size: 11.5px;
    color: var(--gray);
    flex-shrink: 0;
  }

  .note-toggle {
    background: none;
    border: none;
    color: var(--gray);
    font-size: 13px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .note-toggle.has-note,
  .note-toggle:hover,
  .note-toggle:focus-visible {
    color: var(--accent);
    outline: none;
  }

  .scout-champs {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .scout-champ {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .scout-champ-record {
    font-size: 10.5px;
    color: var(--gray);
    font-variant-numeric: tabular-nums;
  }

  .scout-tag {
    font-size: 10.5px;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text);
  }

  .note-input {
    font-size: 12px;
    padding: 5px 8px;
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
    border-radius: calc(var(--border-radius) - 2px);
  }

  .champ-icon {
    width: 34px;
    height: 34px;
    border-radius: 6px;
    object-fit: cover;
    background: var(--button);
  }

  .champ-icon.small {
    width: 24px;
    height: 24px;
    border-radius: 5px;
  }

  .champ-icon.tiny {
    width: 20px;
    height: 20px;
    border-radius: 4px;
  }

  .champ-empty {
    border: 1px dashed var(--input-border);
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

  .game-result.win {
    color: var(--success);
  }

  .game-result.loss {
    color: var(--danger);
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

  .button:hover {
    background: var(--button-elevated, var(--button));
  }

  .button:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  .button.primary {
    background: var(--accent);
    color: var(--on-accent);
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

  .toggle:focus-visible {
    border-color: var(--accent);
    outline: none;
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
    background: var(--accent);
  }

  @media (prefers-reduced-motion: reduce) {
    .toggle .toggle-knob {
      transition: none;
    }
  }
</style>
