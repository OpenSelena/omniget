<script lang="ts">
  import { t } from "$lib/i18n";
  import { getMembers, getSelectedGuildId, getGuild, getInstance } from "$lib/stores/omnidisc-store.svelte";
  import { getVolume, setVolume, isSpeaking } from "$lib/stores/omnidisc-voice-store.svelte";
  import type { OmnidiscMember } from "$lib/omnidisc/types";
  import ProfilePopover from "./ProfilePopover.svelte";

  let guildId = $derived(getSelectedGuildId());
  let members = $derived(getMembers(guildId));
  let online = $derived(members.filter((m) => m.online));
  let offline = $derived(members.filter((m) => !m.online));
  let guild = $derived(getGuild(guildId));
  let instance = $derived(getInstance(guild?.instanceId ?? null));
  let me = $derived(instance?.userId ?? null);
  let profile = $state<{ userId: string; x: number; y: number } | null>(null);
  let menu = $state<{ member: OmnidiscMember; x: number; y: number } | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);
  let menuVolume = $derived(menu ? Math.round(getVolume(menu.member.id) * 100) : 100);

  function openMenu(e: MouseEvent, member: OmnidiscMember) {
    e.preventDefault();
    if (member.id === me) return;
    const x = Math.min(e.clientX, (typeof window !== "undefined" ? window.innerWidth : 9999) - 240);
    const y = Math.min(e.clientY, (typeof window !== "undefined" ? window.innerHeight : 9999) - 140);
    menu = { member, x, y };
  }

  function closeMenuOutside(e: PointerEvent) {
    if (!menu) return;
    if (menuEl && e.target instanceof Node && menuEl.contains(e.target)) return;
    menu = null;
  }

  function onVolumeInput(e: Event) {
    if (!menu) return;
    const v = Number((e.target as HTMLInputElement).value);
    setVolume(menu.member.id, v / 100);
  }

  function initial(name: string): string {
    return name.trim().charAt(0).toUpperCase() || "?";
  }

  function openProfile(member: OmnidiscMember, anchor: HTMLElement) {
    if (!instance) return;
    const rect = anchor.getBoundingClientRect();
    const width = typeof window !== "undefined" ? window.innerWidth : 1280;
    const height = typeof window !== "undefined" ? window.innerHeight : 800;
    profile = {
      userId: member.id,
      x: Math.max(8, Math.min(rect.left - 310, width - 320)),
      y: Math.min(rect.top, height - 420),
    };
  }
</script>

<aside class="member-list" aria-label={$t("omnidisc.members.title")}>
  {#if members.length === 0}
    <div class="empty">
      <p class="empty-title">{$t("omnidisc.members.empty")}</p>
    </div>
  {:else}
    {#if online.length > 0}
      <h3 class="group-title">{$t("omnidisc.members.online", { count: online.length })}</h3>
      <ul class="group">
        {#each online as member (member.id)}
          <li oncontextmenu={(e) => openMenu(e, member)}>
            <button
              type="button"
              class="member"
              title={$t("omnidisc.profile.open", { name: member.name })}
              onclick={(e) => openProfile(member, e.currentTarget)}
            >
              <span class="avatar" class:speaking={isSpeaking(member.id)}><span>{initial(member.name)}</span><span class="dot online" aria-hidden="true"></span></span>
              <span class="name">{member.name}</span>
              {#if member.role}
                <span class="role">{member.role === "owner" ? $t("omnidisc.members.owner") : member.role}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    {#if offline.length > 0}
      <h3 class="group-title">{$t("omnidisc.members.offline", { count: offline.length })}</h3>
      <ul class="group offline-group">
        {#each offline as member (member.id)}
          <li oncontextmenu={(e) => openMenu(e, member)}>
            <button
              type="button"
              class="member"
              title={$t("omnidisc.profile.open", { name: member.name })}
              onclick={(e) => openProfile(member, e.currentTarget)}
            >
              <span class="avatar"><span>{initial(member.name)}</span><span class="dot" aria-hidden="true"></span></span>
              <span class="name">{member.name}</span>
              {#if member.role}
                <span class="role">{member.role === "owner" ? $t("omnidisc.members.owner") : member.role}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</aside>

{#if profile && instance}
  <ProfilePopover
    instanceId={instance.id}
    userId={profile.userId}
    guildId={guildId}
    x={profile.x}
    y={profile.y}
    onClose={() => (profile = null)}
  />
{/if}

<svelte:window onpointerdown={closeMenuOutside} onkeydown={(e) => { if (e.key === "Escape") menu = null; }} />

{#if menu}
  <div class="ctx" role="dialog" aria-label={$t("omnidisc.voice.volume_for", { name: menu.member.name })} style:left={`${menu.x}px`} style:top={`${menu.y}px`} bind:this={menuEl}>
    <label class="ctx-label" for="od-member-volume">{$t("omnidisc.voice.volume_for", { name: menu.member.name })}</label>
    <div class="ctx-row">
      <input id="od-member-volume" type="range" min="0" max="200" step="5" value={menuVolume} oninput={onVolumeInput} />
      <span class="ctx-value">{menuVolume}%</span>
    </div>
    <div class="ctx-actions">
      <button type="button" class="ctx-btn" onclick={() => menu && setVolume(menu.member.id, 1)} disabled={menuVolume === 100}>{$t("omnidisc.voice.volume_reset")}</button>
      <button type="button" class="ctx-btn" onclick={() => { const m = menu; menu = null; if (m) openProfile(m.member, document.body); }}>{$t("omnidisc.profile.open", { name: menu.member.name })}</button>
    </div>
  </div>
{/if}

<style>
  .member-list {
    width: 220px;
    flex: 0 0 220px;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3) var(--space-2);
    background: var(--surface-mut);
    border-left: 1px solid var(--border);
  }

  .group-title {
    margin: var(--space-2) var(--space-2) var(--space-1);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .group {
    list-style: none;
    margin: 0 0 var(--space-3);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .offline-group {
    opacity: 0.55;
  }

  .member {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: 4px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .member:hover {
    background: var(--fill-1);
  }

  .member:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .avatar {
    position: relative;
    width: 28px;
    height: 28px;
    flex: 0 0 28px;
    border-radius: var(--radius-full);
    background: var(--accent-soft);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .dot {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full);
    border: 2px solid var(--surface-mut);
    background: var(--text-dim);
  }

  .dot.online {
    background: var(--success);
  }

  .avatar.speaking {
    box-shadow: 0 0 0 2px var(--success);
  }

  .ctx {
    position: fixed;
    z-index: 50;
    width: 230px;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.25));
  }

  .ctx-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text);
  }

  .ctx-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .ctx-row input {
    flex: 1;
  }

  .ctx-value {
    min-width: 40px;
    text-align: right;
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .ctx-actions {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ctx-btn {
    width: 100%;
    padding: 6px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .ctx-btn:hover:not(:disabled) {
    background: var(--fill-1);
  }

  .ctx-btn:disabled {
    color: var(--text-muted);
    cursor: default;
  }

  .ctx-btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .role {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .empty {
    padding: var(--space-3) var(--space-2);
  }

  .empty-title {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
</style>
