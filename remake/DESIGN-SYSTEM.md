# OmniGet Design System v2 — "The Clipboard Utility"

Derivado das referências em `mobbin/` (P1: Apple Music, Apple Mail, Apple Fitness, App Store, Apple Podcasts, Apple Notes; P2: Linear, Bear, Things 3, Raycast, Craft, Ghost). North star mantido de `DESIGN.md`: software que parece que veio com o macOS. Um accent, hierarquia por peso e tamanho, profundidade por separadores e mudança de tom — não por sombra.

Implementação: CSS custom properties em `src/app.css` (formato que o projeto já usa). A camada legada (~74 props × 14 temas) permanece funcional; a camada semântica nova é derivada globalmente via `color-mix` — nenhum tema fica órfão de token.

---

## 1. Tipografia

Fonte: stack do sistema. `-apple-system` / SF Pro no macOS; Segoe/Roboto nos demais. O preset de tipografia default do app muda de "Bricolage Grotesque + Inter" para **System** (usuários que salvaram outra escolha mantêm a sua). Mono: IBM Plex Mono (logs, paths, kbd).

Hierarquia por **peso e tamanho**, nunca por cor de fundo. Papéis (base desktop macOS, 13px de corpo):

| Papel | Token de tamanho | px | Peso | LH | Tracking | Uso |
|---|---|---|---|---|---|---|
| Large title | `--text-3xl` | 34 | 700 | 1.15 | −0.022em | hero da About, onboarding |
| Title 1 | `--text-2xl` | 26 | 700 | 1.25 | −0.02em | `.page-title` |
| Title 2 | `--text-xl` | 20 | 600 | 1.35 | −0.015em | títulos de diálogo |
| Title 3 | `--text-lg` | 17 | 600 | 1.4 | −0.01em | `.section-title` |
| Headline | `--text-md` | 15 | 600 | 1.5 | −0.005em | título de linha/card |
| Body | `--text-base` | 13 | 400 | 1.55 | 0 | corpo, rows, forms |
| Callout | `--text-sm` | 12 | 400 | 1.5 | 0 | descrições de setting |
| Subhead | `--text-sm` | 12 | 500 | 1.5 | 0 | metadados com ênfase |
| Footnote | `--text-xs` | 11 | 400 | 1.45 | 0 | timestamps, hints |
| Caption | `--text-caption` | 10 | 600 | 1.3 | +0.06em caps | eyebrow, headers de grupo |
| Display | `--text-display` | 48 | 700 | 1.05 | −0.025em | marketing apenas |

Tracking tokens: `--track-tight: -0.02em`, `--track-snug: -0.01em`, `--track-caps: 0.06em`.
Máximo 2 pesos além do regular por tela. Cor nunca é o único sinal de hierarquia.

## 2. Espaçamento

Escala 4pt existente (`--space-1..9` = 4/8/12/16/24/32/48/64/96). Densidades:

- **Compacta** — linhas de tabela/lista da fila, sidebar: altura 28px (`--row-compact`), gap interno `--space-2`.
- **Padrão** — setting rows, cards, forms: altura mínima 44px (`--row-base`), padding `--space-3` `--space-4`, gap entre itens do grupo `--space-2`, entre grupos `--space-5`+.
- **Espaçosa** — heros (About, onboarding, empty states): padding `--space-6`+, gap `--space-5`.

Agrupamento por proximidade: itens do mesmo grupo a 8px; grupos separados por 24px+.

## 3. Cor

Paleta semântica em dois temas core (dark default). Um accent (azul sistema). Status são semânticos, não decorativos. Todos os pares texto/fundo abaixo verificados ≥4.5:1 (script `scripts/contrast-audit.mjs`).

### Dark (`[data-theme="dark"]`)

| Papel | Valor | Contraste |
|---|---|---|
| bg (window) | `#1C1C1E` | — |
| sidebar | `#252527` | — |
| surface (card/control) | `#2C2C2E` | — |
| surface-hi | `#3A3A3C` | — |
| label primary | `#F5F5F7` | 15.6 no bg / 12.8 na surface |
| label secondary (`--text-muted`) | mix(secondary 65%, tertiary) | ≥8 |
| label tertiary (`--text-dim`) | `#98989D` | 5.9 bg / 4.85 surface |
| label quaternary (`--text-faint`) | tertiary 55% alpha | não-textual |
| separator | `#38383A` (opaco) | — |
| accent (seleção, links no bg) | `#0A84FF` | 4.66 no bg |
| accent-hi (texto accent em surface) | `#409CFF` | 4.92 na surface |
| cta fill (botão primário) | `#0071E3`, texto `#FFFFFF` | 4.70 |
| success | `#30D158` | 8.4 |
| warning | `#FF9F0A` | 8.3 (texto escuro em fill: 8.28) |
| danger | `#FF453A` | 4.99 |

### Light (`[data-theme="light"]`)

| Papel | Valor | Contraste |
|---|---|---|
| bg (window) | `#F5F5F7` | — |
| sidebar | `#EBEBED` | — |
| surface (card) | `#FFFFFF` | — |
| surface-hi | `#F0F0F2` | — |
| label primary | `#1D1D1F` | 15.5 / 16.8 |
| label tertiary | `#67676C` | ≥4.6 em todas as camadas |
| separator | `#D1D1D6` | — |
| accent | `#007AFF` (seleção/foco); texto accent usa `--accent-lo` `#0066D6` (5.42) | |
| cta fill | `#0071E3`, texto branco | 4.70 |
| success | `#217E38` | 5.11 |
| warning (texto) | `#B25000` | 5.20 |
| danger | `#D70015` | 5.38 |

### Regras

- **The One Accent Rule**: azul = ação primária + seleção + foco. `--cta` e `--accent` são o mesmo azul percebido; laranja saiu da UI (vive só no mascote Loop).
- Fills de controle derivados globalmente: `--fill-1/2/3` = secondary a 8/12/17% (rest/hover/press) — funcionam nos 14 temas.
- Os 12 temas alternativos mantêm suas paletas próprias (correção: o mac-shell deixava de respeitá-las — ver D-008 no REMAKE-LOG).

## 4. Elevação e material

Flat por padrão. Profundidade = mudança de tom (sidebar mais escura/clara que window, surface mais clara/branca) + separadores de 1px (hairline 0.5px em retina via `--hairline`).

Sombras apenas em camadas flutuantes:
- `--elev-1` — controles salientes (raramente)
- `--elev-2` — popovers, menus, tooltips
- `--elev-3` — diálogos, command palette
- Light theme tem overrides próprios (sombras pretas suaves, sem borda branca).

Material translúcido (palette, backdrop de sheet): `--material` (bg a 78% + `--material-blur: blur(24px) saturate(180%)`). Uma camada de material por vez; nunca material sobre material.

Quando cada camada aparece: conteúdo (nível 0, sem sombra) → popover/menu/tooltip (nível 1, elev-2) → diálogo/sheet/palette (nível 2, elev-3 + backdrop). 

## 5. Raio de canto

Escala (efetiva; o shell mac já usava): `--radius-xs` 4, `--radius-sm` 6, `--radius-md` 8, `--radius-lg` 10, `--radius-xl` 16, `--radius-full` 9999.

Uso: controles inline (kbd, checkbox) xs; botões/inputs/nav-item sm–md; cards/tabelas md–lg; diálogos/palette lg–xl. **Regra de aninhamento**: raio interno = raio externo − padding (mínimo xs).

## 6. Motion

| Token | Valor | Uso |
|---|---|---|
| `--duration-fast` | 120ms | hover, toggle de estado |
| `--duration-base` | 200ms | entradas, fades, drill |
| `--duration-slow` | 320ms | sheets, diálogos |
| `--ease-out` | cubic-bezier(0.2, 0.8, 0.2, 1) | entradas |
| `--ease-spring` | cubic-bezier(0.34, 1.56, 0.64, 1) | interação direta (toggle, mascote) |
| `--ease-in-out` | cubic-bezier(0.45, 0, 0.25, 1) | movimentos laterais (drill) |

`prefers-reduced-motion`: transições caem para opacity-only, springs viram ease-out.

## 7. Comportamento (resumo behavioral-design)

- Uma tela = um CTA dominante (o azul de maior contraste pertence a ele).
- Hierarquia de texto por peso/tamanho; cinzas fazem o fundo para o sinal aparecer.
- Empty states direcionam a primeira ação de valor; loading tem skeleton; erro diz o que aconteceu + o que fazer; sucesso celebra (Loop).
- Sem dark patterns: aceitar e recusar com o mesmo peso; undo sempre disponível.
