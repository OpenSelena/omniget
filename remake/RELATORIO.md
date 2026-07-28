# RELATÓRIO — Remake visual v2 do OmniGet

Branch `remake/visual-v2`. Fonte estética: biblioteca `mobbin/` (264 referências, 22 padrões; P1 Apple > P2 > P3). Ledgers: `REMAKE-LOG.md` (raiz), `remake/PLANO.md`, `remake/DESIGN-SYSTEM.md`, `remake/AUDITORIA.md`, `remake/AUTOCRITICA.md`, `remake/ANTES-E-DEPOIS.md`.

## Resumo

Remake visual completo, Apple-first ("The Clipboard Utility"): design system HIG nos temas core (accent azul único, corpo 13px, fills derivados por `color-mix` válidos nos 14 temas, elevação por tom+hairline), camada de primitivos compartilhada (`primitives.css` + `/_kitchen-sink`), e todas as 23 rotas principais retrabalhadas ou normalizadas, com ciclo de auto-crítica visual contra referências (notas e iterações em `AUTOCRITICA.md`). Zero mudança de lógica de negócio; a fiação Tauri/Rust não foi tocada.

## Métricas antes → depois

| Métrica | Baseline (`ec8b9aa4`) | Final | Evidência |
|---|---|---|---|
| svelte-check erros | 0 | 0 | AUDITORIA §7 |
| svelte-check warnings | 100 | 100 (limite ≤100) | AUDITORIA §7 |
| Testes (vitest) | 23/23 | 23/23 | AUDITORIA §7 |
| Tempo de build | 25.8s | 22.8s (`/usr/bin/time -p`, real) | saídas do vite na sessão |
| Bundle (`build/`) | 24M (49480 blocos) | 23M (47632 blocos) — **−3.7%** | AUDITORIA §8 |
| Contraste AA | não medido | **14/14 temas PASS** | AUDITORIA §13 |
| Hardcoded cor/duração/espaçamento (.css/.ts) | 64 / 14 / 165 | **0 / 0 / 0** (exclusões declaradas = definições de token e dados de plataforma) | AUDITORIA §12 |
| Chaves i18n cruas visíveis | 13 | 0 | AUDITORIA §10 |
| Foco visível por teclado (Tab ×20, 3 rotas) | não medido (foco do .btn era invisível — bug) | 0 paradas sem affordance | AUDITORIA §13 |
| reduced-motion | não medido | 0 animações de posição/escala com `reduce` (Playwright) | AUDITORIA §13 |

## Auto-crítica consolidada (nota final por tela; histórico completo em AUTOCRITICA.md)

| Rota | hier | respiro | alinh | tipo | acab | HIG |
|---|---|---|---|---|---|---|
| / | 4 | 4 | 4 | 4 | 4 | 4 |
| /downloads | 4 | 4 | 4 | 4 | 4 | 4 |
| /marketplace | 4 | 4 | 4 | 4 | 4 | 4 |
| /settings | 4 | 4 | 4 | 5 | 4 | 5 |
| /about | 4 | 4 | 4 | 4 | 3→4 | 4 |
| /about/changelog | 4 | 4 | 4 | 4 | 3→4 | 4 |
| /about/project | 4 | 4 | 4 | 4 | 4 | 3→4 |
| /about/terms | 4 | 4 | 4 | 4 | 4 | 4 |
| /about/privacy | 4 | 4 | 4 | 4 | 4 | 4 |
| /courses | 4 | 4 | 4 | 4 | 4 | 4 |
| /convert | 4 | 4 | 4 | 4 | 4 | 4 |
| /telegram | 4 | 4 | 4 | 4 | 4 | 4 |
| /misc | 4 | 4 | 4 | 4 | 4 | 4 |
| /misc/studio | 4 | 4 | 4 | 5 | 4 | 4 |
| /misc/library | 4 | 4 | 4 | 4 | 3* | 4 |
| /misc/file-clip | 4 | 4 | 4 | 4 | 4 | 4 |
| /study | 4 | 4 | 4 | 4 | 4 | 4 |
| /study/player | 4 | 4 | 4 | 4 | 4 | 4 |
| /study/read | 4 | 4 | 2→4 | 4 | 4 | 4 |
| /study/library | 4 | 4 | 4 | 4 | 4 | 4 |
| /study/music | 4 | 4 | 4 | 4 | 2→4 | 2→4 |
| /study/watch | 4 | 4 | 4 | 4 | 4 | 4 |
| /_kitchen-sink | 5 | 4 | 4 | 4 | 4 | 4 |

\* `/misc/library` acab=3 apenas no estado mockado (exceção JS crua vinda do mock de plugin) — o chrome da tela é ≥4; ver Dívida.

## Decisões de design (raciocínio)

- **Azul de acento — mantido `#0071E3` no CTA, `#007AFF`/`#0A84FF` como accent.** O system blue `#007AFF` com texto branco dá **4.02:1** — reprova AA para texto normal (4.5:1); `#0071E3` (o azul de botão que a própria Apple usa na web) dá **4.70:1** — aprova. Seleção, links, foco e tints usam o system blue nativo (`#007AFF` light / `#0A84FF` dark); só o *fill* de botão primário usa `#0071E3`. Percebe-se um único azul; a exigência da missão (AA programático em todos os pares) decide o empate. Números: AUDITORIA §13 e `DESIGN-SYSTEM.md §3`.
- **Um accent, laranja fora da UI** — laranja sobrevive apenas no mascote e em dados de marca (ex.: estrela do GitHub tintada). Elimina o CTA duplo laranja+azul (Don't explícito do DESIGN.md).
- **Corpo 13px (macOS)** com papéis completos de tipografia; hierarquia por peso/tamanho.
- **Temas mudam cor, não estrutura** — os overrides de cor do `macos-shell.css` atropelavam os 12 temas alternativos (o seletor `:not(...)` pintava Dracula/Catppuccin de Apple dark); cores movidas para os blocos de tema.
- **Tokens novos derivados globalmente** (`--fill-1/2/3`, `--text-faint`, `--material`) via `color-mix` sobre tokens existentes — funcionam nos 14 temas sem tocar cada bloco.
- **Alvos de clique**: 44pt não se aplica a desktop; adotados os três tamanhos AppKit — regular ≥28, small ≥24, mini ≥20 — verificados programaticamente (AUDITORIA §13).

## Bugs reais corrigidos de carona

1. Foco invisível em todos os `.btn` (`box-shadow: var(--focus-ring)` com valor de outline — inválido).
2. 12 temas alternativos sobrescritos pelo mac-shell (acima).
3. Texto invisível no botão next/finish do onboarding (bg = cor do texto).
4. 13 chaves i18n inexistentes renderizando cruas (marketplace + about), sincronizadas nos 11 locales.
5. Colisão título × toolbar no `/study/read` (h1 com `flex:1; min-width:0` colapsava).
6. Superfície music dark-only: ilegível no tema claro (home + 6 componentes tokenizados).
7. Renderer de changelog vazava ``` e `---` como texto.
8. CTA do banner yt-dlp sem contraste AA no light (ISSUE-002 da auditoria antiga).
9. Previews do theme picker mostravam a paleta antiga (laranja) para dark/light.
10. Toast com faixa lateral colorida e download-item com borda-esquerda de status (anti-padrões banidos).
11. `-webkit-app-region: drag` não funciona no WKWebView — arrasto da titlebar via `data-tauri-drag-region` (correção do usuário incorporada na branch).

## Dívida conhecida e próximos passos

- **Music sub-rotas (33)**: ~280 hardcodes brancos em componentes profundos (now-playing, filas, cards YouTube/Spotify/SoundCloud) — a home foi tokenizada; o restante precisa da mesma varredura mapeada (padrão em `AUTOCRITICA.md` linha `/study/music`).
- **Copy de erro em `/misc/library`**: exceção JS crua chega à UI quando o backend do plugin responde nulo/malformado; passar por `error-translate.ts` exige tocar o catch (lógica) — fora do escopo visual desta missão.
- **Estados felizes de plugins** (telegram logado, library com itens) não são capturáveis sem as DLLs (repos irmãos na máquina Windows) — validar visualmente no Windows após deploy local.
- Redesign profundo por tela das sub-rotas de study (herdaram tokens; ver D-005/C-002 no REMAKE-LOG).
- Slimming dos ~74 props/tema para ~18 tokens semânticos continua planejado (fora do escopo, como no CLAUDE.md).

## Verificação das rotas nos 3 viewports

138 shots finais (23 rotas × 3 viewports × 2 temas) capturados sem nenhuma falha de render (lista de failures do harness vazia). Inspeção visual humana-por-imagem cobriu 100% das rotas em 1440×900 (ambos os temas) + amostras em 834×1194; os viewports 390/834 completos estão em `remake/shots/final/` para conferência.

## Não cumprido

- **PR aberto**: ver AUDITORIA §14 — status registrado após a tentativa nesta sessão (a branch contém, por decisão da missão original, o checkpoint com trabalho pré-existente de `feat/plugin-hot-load` não presente na main; o diff do PR inclui esse material além do remake).
- Fora isso: **nenhum** — todos os demais critérios da Definição de Pronto estão com evidência em `remake/AUDITORIA.md`.
