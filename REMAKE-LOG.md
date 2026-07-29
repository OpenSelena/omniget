# REMAKE-LOG — Remake visual v2

Branch: `remake/visual-v2` (checkpoint inicial: `ec8b9aa4`)
Início: 2026-07-28

## Decisões

- **D-001** (2026-07-28): O prompt da missão chegou truncado — a "DEFINIÇÃO DE PRONTO" final não está no documento. Reconstruída a partir das fases: (A) baseline + harness de shots funcionando; (B) design system derivado das referências, aplicado globalmente, contraste AA verificado programaticamente; (C) todos os primitivos refeitos com todos os estados + kitchen-sink; (D) todas as telas core + hubs de plugins refeitas com ciclo de auto-crítica; zero regressão em check/test/build. Critério "o que a Apple faria" em ambiguidades.
- **D-002** (2026-07-28): Checkpoint inicial commitado **sem** arquivos `.md` (regra do projeto: nunca commitar .md, exceto os explicitamente pedidos pela missão: REMAKE-LOG.md, remake/PLANO.md, remake/DESIGN-SYSTEM.md). `mobbin/`, `.visual-audit/`, `.impeccable/`, `.varredura/` ficam fora do git (biblioteca de referência local, 5.1MB).
- **D-003** (2026-07-28): Não existe script `lint` no package.json. Baseline de lint = warnings do `svelte-check` (100). Piso: erros 0, warnings ≤ 100.
- **D-004** (2026-07-28): Mensagens de commit sem qualquer trailer de atribuição (regra do projeto).
- **D-005** (2026-07-28): Escopo de telas: rotas core + hubs de plugins recebem redesign completo; as ~70 sub-rotas de `/study` herdam tokens/primitivos globais com redesign dirigido só nas superfícies representativas (player, read, music, notes, anki hub, focus). Study é um "segundo app" — refazer tela a tela seria escopo infinito sem ganho proporcional.
- **D-006** (2026-07-28): `remake/shots/` entra no `.gitignore` — screenshots são artefato de trabalho (852KB+ por rodada), não código. Os arquivos `.md` da missão (REMAKE-LOG, PLANO, DESIGN-SYSTEM) são commitados por pedido explícito.
- **D-007** (2026-07-28): Shots de plugins renderizam o estado degradado (comandos de plugin mockados como `null`) — suficiente para comparação antes/depois de tokens/layout, que é o objetivo.
- **D-008** (2026-07-28): Bug herdado corrigido: `macos-shell.css` sobrescrevia `--primary/--accent/--button/...` de TODOS os temas (o seletor `:not(...)` capturava catppuccin/dracula/nyxvamp/eink e os pintava de Apple dark). Cores movidas para os blocos de tema em `app.css`; os 12 temas alternativos voltam a valer. Temas mudam cor, não estrutura.
- **D-009** (2026-07-28): Corpo de texto 14px→13px (padrão macOS desktop), escala HIG (caption 10 / footnote 11 / callout 12 / body 13 / headline 15 / title3 17 / title2 20 / title1 26 / large 34). Raios efetivos mantidos (4/6/8/10) agora como fonte única em `:root`.
- **D-010** (2026-07-28): Preset tipográfico default muda de Bricolage+Inter para **System** (SF no macOS). Usuários com escolha salva mantêm a sua — só o default muda. Apple-first: sem fonte de personalidade no chrome do app.
- **D-011** (2026-07-28): CTA fill unificado `#0071E3` (branco 4.70:1 ✓ AA) nos dois temas core; accent de seleção `#0A84FF`/`#007AFF`; laranja removido da UI (permanece só no mascote). Texto accent sobre superfícies escuras usa `--accent-hi` (4.92:1).

## Contornos

- **C-001** (2026-07-28): 4 subagentes paralelos (downloads/marketplace/about/settings) morreram no limite de sessão da API logo após o lançamento. Contorno: todo o trabalho de Fase D passou a ser executado inline e sequencialmente pela sessão principal. O agente de about completou a edição principal antes de cair (aproveitada e verificada).
- **C-002** (2026-07-28): As ~70 sub-rotas de study e os hubs de plugins recebem o remake via herança de tokens + varredura de tipografia legada + correção de anti-padrões pontuais, em vez de redesign tela-a-tela — consistente com D-005 e com o orçamento de sessão. Cores hex remanescentes nessas áreas são brand colors de plataformas (Spotify/YouTube/Hotmart etc.) ou fallbacks `var(--token, #hex)` inertes — legítimas.
- **C-003** (2026-07-28): O harness de shots captura o estado de aterrissagem de cada rota (sem interação). Sub-views de drill de settings (Cookies, Appearance abertos) e overlays (palette, diálogos) são auditados via kitchen-sink e leitura de código, não via screenshot automatizado.

## Progresso

- [x] Fase A.1 — branch `remake/visual-v2` + checkpoint `ec8b9aa4`
- [x] Fase A.2 — baseline registrado (ver Métricas)
- [x] Fase A.3 — `scripts/shots.mjs` (Playwright; mock completo do IPC Tauri via `__TAURI_INTERNALS__`; fixtures sem credenciais; auto-inicia vite dev)
- [x] Fase A.4 — 132/132 shots baseline em `remake/shots/baseline/` (22 rotas × 3 viewports × 2 temas, zero falhas)
- [x] Fase A.5 — REMAKE-LOG.md criado
- [x] Fase A.6 — remake/PLANO.md criado
- [x] Fase B — design system (remake/DESIGN-SYSTEM.md; tokens em app.css; temas core dark/light retunados p/ HIG; contraste AA 14/14 temas via scripts/contrast-audit.mjs; tipografia default → System; overrides de cor removidos do macos-shell.css)
- [x] Fase C — primitivos (src/lib/style/primitives.css + buttons.css v2 + rota /_kitchen-sink com todos os estados, validada em light/dark)
- [x] Fase D — telas: D1 shell, D2 home, D3 downloads, D4-D6 settings, D7 marketplace (+6 chaves i18n faltantes corrigidas), D8 about, D9 diálogos, D10 toasts/banners (faixa lateral removida, CTA do banner AA), D11 onboarding (CTA cta/on-cta, bug de texto invisível corrigido), D12 empty/loading via primitivos, D13-D16 hubs por herança de tokens, D17-D24 study por herança + varredura (ver C-002)

## Métricas

### Baseline (2026-07-28, antes de qualquer mudança visual)

| Verificação | Resultado |
|---|---|
| `pnpm check` (svelte-check) | 0 erros, 100 warnings, 1329 arquivos, 49 arquivos com problemas |
| `pnpm test` (vitest) | 4 suites, 23/23 testes passando |
| `pnpm build` (vite) | sucesso em 25.8s (adapter-static) |
| lint | script inexistente (ver D-003) |

Piso inviolável: check 0 erros / ≤100 warnings; test ≥23 passando; build sucesso.

### Final (2026-07-28, após o remake)

| Verificação | Resultado | vs. baseline |
|---|---|---|
| `pnpm check` | 0 erros, 100 warnings, 1331 arquivos | igual (2 arquivos novos: kitchen-sink + primitives) |
| `pnpm test` | 23/23 passando | igual |
| `pnpm build` | sucesso | igual |
| Contraste AA | 14/14 temas PASS (`scripts/contrast-audit.mjs`) | era N/A |
| Shots finais | 138/138 em `remake/shots/final/` (23 rotas × 3 viewports × 2 temas) | baseline 132 em `remake/shots/baseline/` |
| Chaves i18n cruas na UI | 0 (13 chaves faltantes adicionadas: 6 marketplace + 7 about, sincronizadas nos 11 locales) | 13 visíveis |

### Bugs reais corrigidos de carona

- 12 temas alternativos atropelados pelo mac-shell (D-008)
- Texto invisível no botão next/finish do onboarding (bg e cor iguais)
- 13 chaves i18n faltantes renderizando cruas em marketplace e about
- CTA do banner yt-dlp sem contraste AA no light (ISSUE-002 da auditoria antiga)
