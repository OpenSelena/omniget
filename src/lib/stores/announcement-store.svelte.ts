export interface AnnouncementCTA {
  label: string;
  url: string;
}

export interface Announcement {
  id: string;
  active?: boolean;
  min_app_version?: string;
  expires_at?: string | null;
  badge?: string;
  title: string;
  thumbnail?: string | null;
  body: string;
  highlights?: string[];
  cta?: AnnouncementCTA | null;
}

export const ANNOUNCEMENT_STORAGE_KEY = "omniget_seen_announcements";
export const DEFAULT_ANNOUNCEMENT_URL =
  "https://raw.githubusercontent.com/OpenSelena/omniget/main/assets/announcements.json";

let isOpen = $state(false);
let activeAnnouncement = $state<Announcement | null>(null);

export function getAnnouncementOpen(): boolean {
  return isOpen;
}

export function getActiveAnnouncement(): Announcement | null {
  return activeAnnouncement;
}

export function dismissAnnouncement(markAsSeen = true): void {
  isOpen = false;
  if (markAsSeen && activeAnnouncement?.id && typeof localStorage !== "undefined") {
    try {
      const seen: string[] = JSON.parse(localStorage.getItem(ANNOUNCEMENT_STORAGE_KEY) || "[]");
      if (!seen.includes(activeAnnouncement.id)) {
        seen.push(activeAnnouncement.id);
        localStorage.setItem(ANNOUNCEMENT_STORAGE_KEY, JSON.stringify(seen));
      }
    } catch {
      // Ignore localStorage errors
    }
  }
  activeAnnouncement = null;
}

export function resetAnnouncementState(): void {
  isOpen = false;
  activeAnnouncement = null;
}

export async function checkAnnouncements(
  fetchUrl = DEFAULT_ANNOUNCEMENT_URL,
  fetchFn = typeof fetch !== "undefined" ? fetch : undefined
): Promise<void> {
  if (!fetchFn) return;

  try {
    const res = await fetchFn(fetchUrl, {
      cache: "no-cache",
      signal: typeof AbortSignal !== "undefined" && "timeout" in AbortSignal ? AbortSignal.timeout(3500) : undefined,
    });

    if (!res.ok) return;

    const data: Announcement = await res.json();
    if (!data || !data.id || !data.title) return;

    // Check if explicitly disabled
    if (data.active === false) return;

    // Check expiration if specified
    if (data.expires_at) {
      const expTime = new Date(data.expires_at).getTime();
      if (!Number.isNaN(expTime) && expTime < Date.now()) {
        return;
      }
    }

    // Check seen IDs in storage
    if (typeof localStorage !== "undefined") {
      try {
        const seen: string[] = JSON.parse(localStorage.getItem(ANNOUNCEMENT_STORAGE_KEY) || "[]");
        if (Array.isArray(seen) && seen.includes(data.id)) {
          return;
        }

        // Avoid showing if onboarding wizard is incomplete
        const onboardingDone = localStorage.getItem("omniget_onboarding_completed");
        if (onboardingDone === "false") {
          return;
        }
      } catch {
        // Continue if parsing fails
      }
    }

    activeAnnouncement = data;
    isOpen = true;
  } catch {
    // Fail silently on network errors / timeout
  }
}
