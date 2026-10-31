<script lang="ts">
  let { name, size = 22 } = $props<{
    name:
      | "saved"
      | "star"
      | "file"
      | "video"
      | "audio"
      | "image"
      | "tag"
      | "play"
      | "pin"
      | "mute"
      | "verified"
      | "send"
      | "edit"
      | "delete"
      | "add"
      | "lock"
      | "globe"
      | "music"
      | "gif"
      | "round-video"
      | "voice"
      | "chat-photo"
      | "photo-video";
    size?: number;
  }>();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "saved"}
    <!-- bookmark / saved messages -->
    <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" fill="currentColor" stroke="none" />
  {:else if name === "star"}
    <path
      d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
      fill="currentColor"
      stroke="none"
    />
  {:else if name === "file"}
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
    <polyline points="14 2 14 8 20 8" />
  {:else if name === "video"}
    <polygon points="23 7 16 12 23 17 23 7" fill="currentColor" stroke="none" />
    <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
  {:else if name === "audio" || name === "music"}
    <path d="M9 18V5l12-2v13" />
    <circle cx="6" cy="18" r="3" fill="currentColor" stroke="none" />
    <circle cx="18" cy="16" r="3" fill="currentColor" stroke="none" />
  {:else if name === "image"}
    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
    <circle cx="8.5" cy="8.5" r="1.5" fill="currentColor" stroke="none" />
    <polyline points="21 15 16 10 5 21" />
  {:else if name === "tag"}
    <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z" />
    <line x1="7" y1="7" x2="7.01" y2="7" stroke-width="3" />
  {:else if name === "play"}
    <polygon points="5 3 19 12 5 21 5 3" fill="currentColor" stroke="none" />
  {:else if name === "pin"}
    <path d="M12 2v8m0 0l-4 4m4-4l4 4M5 22h14" />
    <circle cx="12" cy="6" r="3" fill="currentColor" stroke="none" />
  {:else if name === "mute"}
    <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" fill="currentColor" stroke="none" />
    <line x1="23" y1="9" x2="17" y2="15" />
    <line x1="17" y1="9" x2="23" y2="15" />
  {:else if name === "verified"}
    <circle cx="12" cy="12" r="10" fill="currentColor" stroke="none" />
    <polyline points="8 12 11 15 16 9" stroke="white" stroke-width="2.5" />
  {:else if name === "send"}
    <line x1="22" y1="2" x2="11" y2="13" />
    <polygon points="22 2 15 22 11 13 2 9 22 2" fill="currentColor" stroke="none" />
  {:else if name === "edit"}
    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
  {:else if name === "delete"}
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
    <path d="M10 11v6M14 11v6" />
    <path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
  {:else if name === "add"}
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  {:else if name === "lock"}
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
  {:else if name === "globe"}
    <circle cx="12" cy="12" r="10" />
    <line x1="2" y1="12" x2="22" y2="12" />
    <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
  {:else if name === "gif"}
    <rect x="2" y="6" width="20" height="12" rx="2" ry="2" />
    <text x="12" y="15" text-anchor="middle" font-size="7" font-weight="700" font-family="sans-serif" fill="currentColor" stroke="none">GIF</text>
  {:else if name === "round-video"}
    <circle cx="12" cy="12" r="10" />
    <polygon points="10 8 16 12 10 16 10 8" fill="currentColor" stroke="none" />
  {:else if name === "voice"}
    <rect x="9" y="2" width="6" height="12" rx="3" ry="3" fill="currentColor" stroke="none" />
    <path d="M5 11a7 7 0 0 0 14 0" />
    <line x1="12" y1="18" x2="12" y2="22" />
  {:else if name === "chat-photo"}
    <circle cx="12" cy="9" r="3" />
    <path d="M5 21a7 7 0 0 1 14 0" />
    <circle cx="12" cy="12" r="10" />
  {:else if name === "photo-video"}
    <rect x="2" y="6" width="14" height="12" rx="2" ry="2" />
    <circle cx="6.5" cy="10.5" r="1.2" fill="currentColor" stroke="none" />
    <polyline points="16 14 11 9 4 18" />
    <polygon points="17 9 22 6 22 18 17 15 17 9" fill="currentColor" stroke="none" />
  {/if}
</svg>

<style>
  svg {
    flex-shrink: 0;
    color: inherit;
  }
</style>
