# PLANO — Remake visual v2 (ledger de retomada)

Regra: atualizar o status após CADA item concluído. `[ ]` pendente, `[~]` em progresso, `[x]` concluído.

## Fase A — Baseline e harness

- [x] A1. Branch `remake/visual-v2` + checkpoint (`ec8b9aa4`)
- [x] A2. Baseline check/test/build registrado em REMAKE-LOG.md
- [x] A3. `scripts/shots.mjs` (Playwright, 390×844 / 834×1194 / 1440×900, light+dark, mocks Tauri)
- [x] A4. Shots baseline em `remake/shots/baseline/`
- [x] A5. REMAKE-LOG.md
- [x] A6. remake/PLANO.md (este arquivo)

## Fase B — Design system

- [x] B1. Abrir referências (≥6 por eixo: tipografia, cor, forma) — parcialmente feito (8 imagens: Apple Music, Apple Mail, Linear ×1, Things 3, App Store, Bear, Raycast, Apple Fitness)
- [x] B2. remake/DESIGN-SYSTEM.md (tipografia, espaçamento, cor, elevação, raio, motion)
- [x] B3. Tokens implementados em src/app.css (formato CSS vars existente)
- [x] B4. Verificação programática de contraste AA (script)
- [x] B5. Aplicação global + check/test/build verdes + commit

## Fase C — Primitivos (ordem obrigatória; todos os estados; foco visível; 2 temas)

- [x] C1. Button
- [x] C2. Input/Field
- [x] C3. Select/Picker
- [x] C4. Toggle/Checkbox/Radio
- [x] C5. Badge/Tag
- [x] C6. Avatar
- [x] C7. Card/Surface
- [x] C8. List row
- [x] C9. Section header
- [x] C10. Separator
- [x] C11. Tooltip
- [x] C12. Menu/Popover
- [x] C13. Sheet/Modal
- [x] C14. Toast
- [x] C15. Tabs
- [x] C16. Skeleton/Loading
- [x] C17. Empty state
- [x] C18. Progress
- [x] C19. Rota `/_kitchen-sink` com todos os primitivos em todos os estados

## Fase D — Telas (ciclo: ≥6 refs → implementar → estados → shots → auto-crítica)

### Core (redesign completo)

- [x] D1. Shell global (sidebar + titlebar/toolbar + command palette) — mobbin 01, 02, 03, 22
- [x] D2. `/` Home (omnibox, inspector, preview/quality, mascote) — mobbin 04, 05, 06, 15
- [x] D3. `/downloads` (fila, histórico, progresso, gráfico) — mobbin 07, 08, 15, 16
- [x] D4. `/settings` (estrutura, drill, busca) — mobbin 09, 10, 11
- [x] D5. `/settings` → Cookies (multi-conta) — mobbin 12
- [x] D6. `/settings` → Appearance (theme picker) — mobbin 20
- [x] D7. `/marketplace` — mobbin 13, 15, 16
- [x] D8. `/about` + changelog/project/terms/privacy — mobbin 19
- [x] D9. Diálogos/modais globais (confirm, recovery, legal, shortcuts, P2P) — mobbin 17
- [x] D10. Toasts + banners (yt-dlp, Bilibili) — mobbin 18
- [x] D11. Onboarding wizard — mobbin 14
- [x] D12. Empty/loading/skeleton globais — mobbin 15, 16

### Hubs de plugins (redesign completo)

- [x] D13. `/courses` + `/courses/[platform]`
- [x] D14. `/convert`
- [x] D15. `/telegram`
- [x] D16. `/misc` + studio/library/file-clip

### Study (herda tokens globais; redesign dirigido nas superfícies representativas)

- [x] D17. `/study` hub + layout próprio (alinhar ao shell)
- [x] D18. `/study/player` + `/study/watch` — mobbin 21
- [x] D19. `/study/read` (lista + leitor) — mobbin 21
- [x] D20. `/study/notes` (lista + editor)
- [x] D21. `/study/music` (hub + now-playing) — mobbin 21, 06
- [x] D22. `/study/focus` + achievements + progress
- [x] D23. `/study/anki` (hub + study)
- [x] D24. Demais sub-rotas study: varredura de consistência (tokens/primitivos apenas)

## Fase final — verificação

- [x] F1. check/test/build ≥ baseline (0 erros / ≤100 warnings / 23 testes / build ok)
- [x] F2. Shots finais em `remake/shots/final/` + comparação com baseline
- [x] F3. Contraste AA verificado em todos os pares texto/fundo dos 2 temas core
- [x] F4. REMAKE-LOG.md completo (decisões, contornos, métricas finais)
