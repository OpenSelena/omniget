// Ícones de estado canônico (Sessão 7 + Sessão 8). A UI usa esses paths SVG
// inline pra distinguir visualmente cada estado. Cores são **tokens semânticos
// do design system do host** (`--success`, `--danger`, `--warning`,
// `--info`, `--orange`, `--gray`), de modo que cada tema apresente
// status com cores coerentes com o resto da UI (catppuccin, dracula, eink
// etc). Mapeamento:
//   - `--info`     → movimento (picked_up / in_transit / in_warehouse).
//     `--info` aliases `--cta` no app.css, que é um azul real em todos os
//     temas (mesmo nos defaults light/dark, onde `--blue` aliases o accent
//     laranja da marca e portanto não serve).
//   - `--orange`   → ação ativa/atenção (customs / out_for_delivery /
//     awaiting_pickup).
//   - `--warning`  → atraso sem problema concreto (delayed).
//   - `--danger`   → problema (customs_held / delivery_failure / damaged).
//   - `--success`  → estado terminal feliz (delivered / customs_cleared).
//   - `--gray`     → inerte/desconhecido (preadvice / returning / returned
//     / unknown).
//
// Os SVG paths usam o estilo Feather/Lucide (stroke-only, `viewBox="0 0 24 24"`,
// `stroke-width="1.6"`, sem fills). Cada `svgPath` é uma string concatenada de
// múltiplos `M ... L ...` subpaths que serão renderizados num único `<path>`.

import type { CanonicalStatus } from "$lib/tracker-bridge";

export type StatusIcon = {
  svgPath: string;
  /// CSS color expression — tipicamente `var(--info)`, `var(--success)`, etc.
  /// O caller injeta via `style="--status-color: {icon.color}"`, então
  /// qualquer expressão CSS válida funciona.
  color: string;
  /// i18n key (no formato `tracking.status.*`). O caller resolve via `$t(...)`.
  labelKey: string;
};

const ICONS: Record<CanonicalStatus, StatusIcon> = {
  preadvice: {
    labelKey: "tracking.status.preadvice",
    color: "var(--gray)",
    // Caixa lacrada — package outline com um quadrado interno (a etiqueta).
    svgPath:
      "M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z M3.27 6.96 12 12.01l8.73-5.05 M12 22.08V12 M9 12.5h6",
  },
  picked_up: {
    labelKey: "tracking.status.picked_up",
    color: "var(--info)",
    // Caixa + check pequeno em cima — coletado/postado.
    svgPath:
      "M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z M3.27 6.96 12 12.01l8.73-5.05 M9 16l2 2 4-4",
  },
  in_transit: {
    labelKey: "tracking.status.in_transit",
    color: "var(--info)",
    // Caminhão clássico (Feather "truck").
    svgPath:
      "M3 17V7h11v10 M14 11h4l3 3v3h-7 M7 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z M17 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
  },
  in_warehouse: {
    labelKey: "tracking.status.in_warehouse",
    color: "var(--info)",
    // Galpão — telhado de duas águas + portão central.
    svgPath:
      "M3 21V10l9-6 9 6v11 M3 21h18 M9 21v-7h6v7 M10 15h4",
  },
  customs: {
    labelKey: "tracking.status.customs",
    color: "var(--orange)",
    // Edifício com colunas (landmark-style).
    svgPath:
      "M3 21h18 M5 21V10 M9 21V10 M15 21V10 M19 21V10 M3 10h18 M12 3 3 8h18l-9-5z",
  },
  customs_held: {
    labelKey: "tracking.status.customs_held",
    color: "var(--danger)",
    // Edifício + cadeado.
    svgPath:
      "M3 21h12 M5 21V11 M9 21V11 M13 21V11 M3 11h12 M9 4 3 8h12 M16 17a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v4h-6v-4z M18 15v-2a2 2 0 1 1 4 0v2",
  },
  customs_cleared: {
    labelKey: "tracking.status.customs_cleared",
    color: "var(--success)",
    // Edifício + check.
    svgPath:
      "M3 21h12 M5 21V11 M9 21V11 M13 21V11 M3 11h12 M9 4 3 8h12 M16 17l2 2 4-4",
  },
  out_for_delivery: {
    labelKey: "tracking.status.out_for_delivery",
    color: "var(--orange)",
    // Caminhão acelerando — truck com linhas de movimento atrás.
    svgPath:
      "M5 17V8h9v9 M14 12h3l3 3v2h-6 M9 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z M18 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4z M1 9h3 M1 12h2 M1 15h3",
  },
  awaiting_pickup: {
    labelKey: "tracking.status.awaiting_pickup",
    color: "var(--orange)",
    // Pin de localização (Feather "map-pin") — aponta onde retirar.
    svgPath:
      "M21 10c0 7-9 13-9 13S3 17 3 10a9 9 0 0 1 18 0z M12 13a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
  },
  delivered: {
    labelKey: "tracking.status.delivered",
    color: "var(--success)",
    // Check-circle (Feather).
    svgPath: "M22 11.08V12a10 10 0 1 1-5.93-9.14 M22 4 12 14.01l-3-3",
  },
  delivery_failure: {
    labelKey: "tracking.status.delivery_failure",
    color: "var(--danger)",
    // Alert-circle (Feather).
    svgPath:
      "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z M12 8v4 M12 16h.01",
  },
  delayed: {
    labelKey: "tracking.status.delayed",
    color: "var(--warning)",
    // Clock (Feather).
    svgPath: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z M12 6v6l4 2",
  },
  returning_to_sender: {
    labelKey: "tracking.status.returning_to_sender",
    color: "var(--gray)",
    // Corner-up-left arrow (Feather).
    svgPath: "M9 14 4 9l5-5 M4 9h11a5 5 0 0 1 5 5v6",
  },
  returned_to_sender: {
    labelKey: "tracking.status.returned_to_sender",
    color: "var(--gray)",
    // Corner-up-left + dot fechado.
    svgPath: "M9 14 4 9l5-5 M4 9h11a5 5 0 0 1 5 5v6 M20 18h-4",
  },
  damaged: {
    labelKey: "tracking.status.damaged",
    color: "var(--danger)",
    // Alert-triangle (Feather).
    svgPath:
      "m10.29 3.86-8.18 14.18A2 2 0 0 0 3.84 21h16.32a2 2 0 0 0 1.73-2.96L13.71 3.86a2 2 0 0 0-3.42 0z M12 9v4 M12 17h.01",
  },
  unknown: {
    labelKey: "tracking.status.unknown",
    color: "var(--gray)",
    // Help-circle (Feather).
    svgPath:
      "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3 M12 17h.01",
  },
};

export function getStatusIcon(status: CanonicalStatus | string | null | undefined): StatusIcon {
  if (status == null) return ICONS.unknown;
  const key = status as CanonicalStatus;
  return ICONS[key] ?? ICONS.unknown;
}

/// Classificador client-side espelhando `classify_text` do backend Rust. Usado
/// na timeline da página de detalhe pra dar a cada evento seu próprio ícone
/// canônico sem precisar de uma round-trip ao plugin. Mantém a mesma ordem de
/// precedência (`Delivered` ganha de `OutForDelivery`, etc).
export function classifyEvent(rawStatus: string | null | undefined, rawLocation?: string | null): CanonicalStatus {
  const t = `${rawStatus ?? ""} ${rawLocation ?? ""}`.toLowerCase().trim();
  if (!t) return "unknown";

  // Terminais.
  if (
    t.includes("devolvido ao remetente") ||
    t.includes("objeto devolvido") ||
    t.includes("returned to sender") ||
    t.includes("devolução concluída")
  )
    return "returned_to_sender";
  if (
    t.includes("entrega realizada") ||
    t.includes("entregue ao destinat") ||
    t.includes("objeto entregue") ||
    t.includes("pacote entregue") ||
    t.includes("delivered") ||
    t.includes("recebido por") ||
    t.includes("assinado pelo destinat") ||
    (t.startsWith("entregue") && !t.includes("não foi"))
  )
    return "delivered";

  // Problemas.
  if (t.includes("avariad") || t.includes("danific") || t.includes("damaged") || t.includes("destru"))
    return "damaged";
  if (
    t.includes("não foi possível entregar") ||
    t.includes("tentativa de entrega") ||
    t.includes("entrega não realizada") ||
    t.includes("destinatário ausente") ||
    t.includes("delivery failure") ||
    t.includes("delivery attempted")
  )
    return "delivery_failure";
  if (
    t.includes("em devolução") ||
    t.includes("retornando ao remetente") ||
    t.includes("returning to sender")
  )
    return "returning_to_sender";

  // Retirada.
  if (
    t.includes("disponível para retirada") ||
    t.includes("aguardando retirada") ||
    t.includes("pronto para retirada") ||
    t.includes("retirar na agência") ||
    t.includes("ready for pickup") ||
    t.includes("available for pickup")
  )
    return "awaiting_pickup";

  // Alfândega.
  if (
    t.includes("liberado pela alfândega") ||
    t.includes("liberado pela aduana") ||
    t.includes("customs released") ||
    t.includes("customs cleared") ||
    (t.includes("liberado") && (t.includes("alfândega") || t.includes("aduane")))
  )
    return "customs_cleared";
  if (
    t.includes("retido na alfândega") ||
    t.includes("retido na aduana") ||
    t.includes("aguardando pagamento") ||
    t.includes("aguardando documento") ||
    t.includes("customs held")
  )
    return "customs_held";
  if (
    t.includes("desembaraço aduaneiro") ||
    t.includes("alfândega") ||
    t.includes("aduana") ||
    t.includes("customs")
  )
    return "customs";

  // Saiu pra entrega antes de in_transit.
  if (
    t.includes("saiu para entrega") ||
    t.includes("em rota de entrega") ||
    t.includes("em rota") ||
    t.includes("rota de entrega") ||
    t.includes("em entrega") ||
    t.includes("out for delivery")
  )
    return "out_for_delivery";

  // Warehouse.
  if (
    t.includes("centro de distribuição") ||
    t.includes("centro logístico") ||
    t.includes("chegou ao centro") ||
    t.includes("triagem") ||
    t.includes("unidade de tratamento") ||
    t.includes("warehouse") ||
    t.includes("sorting center") ||
    t.includes("left sorting")
  )
    return "in_warehouse";

  if (t.includes("atras") || t.includes("delayed") || t.includes("sem movimenta")) return "delayed";

  if (
    t.includes("em trânsito") ||
    t.includes("em transito") ||
    t.includes("encaminhado") ||
    t.includes("in transit") ||
    t.includes("trânsito para")
  )
    return "in_transit";

  if (
    t.includes("objeto postado") ||
    t.includes("postagem") ||
    t.includes("postado") ||
    t.includes("coletado") ||
    t.includes("collected") ||
    t.includes("picked up") ||
    t.includes("recebido pelo") ||
    t.includes("origem")
  )
    return "picked_up";

  if (
    t.includes("pedido criado") ||
    t.includes("aguardando coleta") ||
    t.includes("preparing") ||
    t.includes("preadvice") ||
    t.includes("etiqueta gerada") ||
    t.includes("pending pickup")
  )
    return "preadvice";

  return "unknown";
}
