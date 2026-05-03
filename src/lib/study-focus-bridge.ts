const BREAK_EVENT = "study:focus:break-start";

export type FocusBreakDetail = {
  sessionId: number;
  reason: "session_ended" | "manual_stop";
};

let lastFiredSessionId: number | null = null;
let lastFiredReason: FocusBreakDetail["reason"] | null = null;

export function emitFocusBreakStart(detail: FocusBreakDetail): void {
  if (
    lastFiredSessionId === detail.sessionId &&
    lastFiredReason === detail.reason
  ) {
    return;
  }
  lastFiredSessionId = detail.sessionId;
  lastFiredReason = detail.reason;
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(BREAK_EVENT, { detail }));
}

export function onFocusBreakStart(
  handler: (detail: FocusBreakDetail) => void,
): () => void {
  if (typeof window === "undefined") return () => {};
  const wrapped = (e: Event) => {
    const ev = e as CustomEvent<FocusBreakDetail>;
    handler(ev.detail);
  };
  window.addEventListener(BREAK_EVENT, wrapped);
  return () => window.removeEventListener(BREAK_EVENT, wrapped);
}
