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

## Contornos

(nenhum ainda)

## Progresso

- [x] Fase A.1 — branch `remake/visual-v2` + checkpoint `ec8b9aa4`
- [x] Fase A.2 — baseline registrado (ver Métricas)
- [x] Fase A.3 — `scripts/shots.mjs` (Playwright; mock completo do IPC Tauri via `__TAURI_INTERNALS__`; fixtures sem credenciais; auto-inicia vite dev)
- [x] Fase A.4 — 132/132 shots baseline em `remake/shots/baseline/` (22 rotas × 3 viewports × 2 temas, zero falhas)
- [x] Fase A.5 — REMAKE-LOG.md criado
- [x] Fase A.6 — remake/PLANO.md criado
- [ ] Fase B — design system
- [ ] Fase C — primitivos
- [ ] Fase D — telas

## Métricas

### Baseline (2026-07-28, antes de qualquer mudança visual)

| Verificação | Resultado |
|---|---|
| `pnpm check` (svelte-check) | 0 erros, 100 warnings, 1329 arquivos, 49 arquivos com problemas |
| `pnpm test` (vitest) | 4 suites, 23/23 testes passando |
| `pnpm build` (vite) | sucesso em 25.8s (adapter-static) |
| lint | script inexistente (ver D-003) |

Piso inviolável: check 0 erros / ≤100 warnings; test ≥23 passando; build sucesso.
