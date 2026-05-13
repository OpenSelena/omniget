import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { pluginInvoke } from "$lib/plugin-invoke";

export type Carrier =
  | "loggi"
  | "cainiao"
  | "melhor_envio"
  | "anjun"
  | "shopee"
  | "total_express"
  | "jt_express"
  | "correios"
  | "unknown";

export type CanonicalStatus =
  | "preadvice"
  | "picked_up"
  | "in_transit"
  | "in_warehouse"
  | "customs"
  | "customs_held"
  | "customs_cleared"
  | "out_for_delivery"
  | "awaiting_pickup"
  | "delivered"
  | "delivery_failure"
  | "delayed"
  | "returning_to_sender"
  | "returned_to_sender"
  | "damaged"
  | "unknown";

export type TrackedPackage = {
  id: number;
  code: string;
  alias: string | null;
  carrier: Carrier | string;
  cpf: string | null;
  last_status: string | null;
  last_event_at_ms: number | null;
  days_in_transit: string | null;
  created_at_ms: number;
  updated_at_ms: number;
  is_delivered: boolean;
  canonical_status: CanonicalStatus | null;
  is_archived: boolean;
  archive_prompt_dismissed: boolean;
};

export type ListFilter = "active" | "archived" | "all";

export type TrackingEvent = {
  id: number;
  package_id: number;
  seq: number;
  status: string | null;
  date: string | null;
  time: string | null;
  location: string | null;
  sub_status: string[];
};

export type NewPackageInput = {
  code: string;
  alias?: string;
  carrier_hint?: string;
  cpf?: string;
};

export type CarrierDetection = {
  carrier: Carrier | string;
  display_name: string;
  requires_cpf: boolean;
};

export type TrackingGetResponse = {
  package: TrackedPackage;
  events: TrackingEvent[];
} | null;

export type RefreshResult = {
  ok: boolean;
  id: number;
  events_added?: number;
  status_changed?: boolean;
};

export type TrackerEventName =
  | "status-changed"
  | "refresh-started"
  | "refresh-progress"
  | "refresh-done";

export type TrackerEventPayload = {
  "status-changed": {
    id: number;
    code: string;
    old_status: string | null;
    new_status: string;
    old_canonical: CanonicalStatus | null;
    new_canonical: CanonicalStatus | null;
    canonical_changed: boolean;
    location: string | null;
  };
  "refresh-started": { id: number; code: string; carrier: string };
  "refresh-progress": { id: number; ok: boolean };
  "refresh-done": {
    id: number;
    events_added: number;
    status_changed: boolean;
    error?: string;
  };
};

const PLUGIN_ID = "misc";

export function trackerList(filter: ListFilter = "active"): Promise<TrackedPackage[]> {
  return pluginInvoke<TrackedPackage[]>(PLUGIN_ID, "misc:tracking:list", { filter });
}

export function trackerGet(id: number): Promise<TrackingGetResponse> {
  return pluginInvoke<TrackingGetResponse>(PLUGIN_ID, "misc:tracking:get", { id });
}

export function trackerAdd(input: NewPackageInput): Promise<TrackedPackage> {
  const args: Record<string, unknown> = { code: input.code };
  if (input.alias != null) args.alias = input.alias;
  if (input.carrier_hint != null) args.carrier_hint = input.carrier_hint;
  if (input.cpf != null) args.cpf = input.cpf;
  return pluginInvoke<TrackedPackage>(PLUGIN_ID, "misc:tracking:add", args);
}

export function trackerRemove(id: number): Promise<{ ok: boolean; id: number }> {
  return pluginInvoke(PLUGIN_ID, "misc:tracking:remove", { id });
}

export function trackerRefresh(id: number): Promise<RefreshResult> {
  return pluginInvoke<RefreshResult>(PLUGIN_ID, "misc:tracking:refresh", { id });
}

export function trackerRefreshAll(): Promise<{ ok: boolean }> {
  return pluginInvoke(PLUGIN_ID, "misc:tracking:refresh_all", {});
}

export function trackerDetectCarrier(code: string): Promise<CarrierDetection> {
  return pluginInvoke<CarrierDetection>(PLUGIN_ID, "misc:tracking:detect_carrier", { code });
}

export function trackerSetAlias(id: number, alias: string | null): Promise<{ ok: boolean; id: number }> {
  return pluginInvoke(PLUGIN_ID, "misc:tracking:set_alias", { id, alias });
}

export function trackerSetCpf(id: number, cpf: string | null): Promise<{ ok: boolean; id: number }> {
  return pluginInvoke(PLUGIN_ID, "misc:tracking:set_cpf", { id, cpf });
}

export function trackerSetArchived(
  id: number,
  archived: boolean,
): Promise<{ ok: boolean; id: number; archived: boolean }> {
  return pluginInvoke(PLUGIN_ID, "misc:tracking:set_archived", { id, archived });
}

export function trackerDismissArchivePrompt(
  id: number,
): Promise<{ ok: boolean; id: number }> {
  return pluginInvoke(PLUGIN_ID, "misc:tracking:dismiss_archive_prompt", { id });
}

export async function onTrackerEvent<N extends TrackerEventName>(
  name: N,
  cb: (payload: TrackerEventPayload[N]) => void,
): Promise<UnlistenFn> {
  return listen<TrackerEventPayload[N]>(`misc:tracking:${name}`, (e) => cb(e.payload));
}
