import type { Carrier } from "$lib/tracker-bridge";

export type CarrierIcon = {
  svgPath: string;
  color: string;
  /// Nome de exibição. Nomes de transportadora são marcas registradas e
  /// permanecem literais em todos os idiomas — exceto `unknown`, que usa
  /// `labelKey` pra resolver via i18n. Quando `labelKey` está presente, o
  /// caller deve preferi-la sobre `label`.
  label: string;
  labelKey?: string;
};

const ICONS: Record<Carrier, CarrierIcon> = {
  correios: {
    label: "Correios",
    color: "#FFB300",
    svgPath: "M4 6h16v12H4z M4 6l8 6 8-6",
  },
  loggi: {
    label: "Loggi",
    color: "#FF6F00",
    svgPath: "M3 17V7h11v10 M14 11h4l3 3v3h-7 M7 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z M17 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
  },
  shopee: {
    label: "Shopee",
    color: "#EE4D2D",
    svgPath: "M5 8h14l-1 12H6L5 8z M9 8V5a3 3 0 0 1 6 0v3",
  },
  cainiao: {
    label: "Cainiao",
    color: "#1E88E5",
    svgPath: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z M2 12h20 M12 2a15 15 0 0 1 0 20 M12 2a15 15 0 0 0 0 20",
  },
  melhor_envio: {
    label: "Melhor Envio",
    color: "#43A047",
    svgPath: "M4 13h16v8H4z M4 13l2-4h12l2 4 M12 9V2 M8.5 5.5 12 2l3.5 3.5",
  },
  anjun: {
    label: "Anjun",
    color: "#8E24AA",
    svgPath: "M21 16v-2l-8-5V3.5a1.5 1.5 0 0 0-3 0V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z",
  },
  total_express: {
    label: "Total Express",
    color: "#E53935",
    svgPath: "M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z M7.5 4.21 12 6.81l4.5-2.6 M7.5 19.79V14.6L3 12 M21 12l-4.5 2.6v5.19 M3.27 6.96 12 12.01l8.73-5.05 M12 22.08V12",
  },
  jt_express: {
    label: "J&T Express",
    color: "#C62828",
    svgPath: "M4 10h11v8H4z M4 10l2-3h7l2 3 M17 11h3 M17 14h3 M17 17h3",
  },
  unknown: {
    label: "Unknown",
    labelKey: "tracking.carrier.unknown",
    color: "#757575",
    svgPath: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3 M12 17h.01",
  },
};

export function getCarrierIcon(carrier: string): CarrierIcon {
  return ICONS[(carrier as Carrier)] ?? ICONS.unknown;
}

export type CarrierRoute = { from: string; to: string };

const ROUTES: Record<Carrier, CarrierRoute | null> = {
  correios: { from: "BRA", to: "BRA" },
  loggi: { from: "BRA", to: "BRA" },
  shopee: { from: "BRA", to: "BRA" },
  melhor_envio: { from: "BRA", to: "BRA" },
  total_express: { from: "BRA", to: "BRA" },
  jt_express: { from: "BRA", to: "BRA" },
  cainiao: { from: "CHN", to: "BRA" },
  anjun: { from: "CHN", to: "BRA" },
  unknown: null,
};

export function getCarrierRoute(carrier: string): CarrierRoute | null {
  return ROUTES[(carrier as Carrier)] ?? null;
}
