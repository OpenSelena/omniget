# PLANO — Remake visual v2 (ledger de retomada)

Regra: atualizar o status após CADA item concluído. `[ ]` pendente, `[~]` em progresso, `[x]` concluído.

## Fase A — Baseline e harness

- [x] A1. Branch `remake/visual-v2` + checkpoint (`ec8b9aa4`)
- [x] A2. Baseline check/test/build registrado em REMAKE-LOG.md
- [~] A3. `scripts/shots.mjs` (Playwright, 390×844 / 834×1194 / 1440×900, light+dark, mocks Tauri)
- [ ] A4. Shots baseline em `remake/shots/baseline/`
- [x] A5. REMAKE-LOG.md
- [x] A6. remake/PLANO.md (este arquivo)

## Fase B — Design system

- [x] B1. Abrir referências (≥6 por eixo: tipografia, cor, forma) — parcialmente feito (8 imagens: Apple Music, Apple Mail, Linear ×1, Things 3, App Store, Bear, Raycast, Apple Fitness)
- [x] B2. remake/DESIGN-SYSTEM.md (tipografia, espaçamento, cor, elevação, raio, motion)
- [x] B3. Tokens implementados em src/app.css (formato CSS vars existente)
- [x] B4. Verificação programática de contraste AA (script)
- [x] B5. Aplicação global + check/test/build verdes + commit

## Fase C — Primitivos (ordem obrigatória; todos os estados; foco visível; 2 temas)

- [ ] C1. Button
- [ ] C2. Input/Field
- [ ] C3. Select/Picker
- [ ] C4. Toggle/Checkbox/Radio
- [ ] C5. Badge/Tag
- [ ] C6. Avatar
- [ ] C7. Card/Surface
- [ ] C8. List row
- [ ] C9. Section header
- [ ] C10. Separator
- [ ] C11. Tooltip
- [ ] C12. Menu/Popover
- [ ] C13. Sheet/Modal
- [ ] C14. Toast
- [ ] C15. Tabs
- [ ] C16. Skeleton/Loading
- [ ] C17. Empty state
- [ ] C18. Progress
- [ ] C19. Rota `/_kitchen-sink` com todos os primitivos em todos os estados

## Fase D — Telas (ciclo: ≥6 refs → implementar → estados → shots → auto-crítica)

### Core (redesign completo)

- [ ] D1. Shell global (sidebar + titlebar/toolbar + command palette) — mobbin 01, 02, 03, 22
- [ ] D2. `/` Home (omnibox, inspector, preview/quality, mascote) — mobbin 04, 05, 06, 15
- [ ] D3. `/downloads` (fila, histórico, progresso, gráfico) — mobbin 07, 08, 15, 16
- [ ] D4. `/settings` (estrutura, drill, busca) — mobbin 09, 10, 11
- [ ] D5. `/settings` → Cookies (multi-conta) — mobbin 12
- [ ] D6. `/settings` → Appearance (theme picker) — mobbin 20
- [ ] D7. `/marketplace` — mobbin 13, 15, 16
- [ ] D8. `/about` + changelog/project/terms/privacy — mobbin 19
- [ ] D9. Diálogos/modais globais (confirm, recovery, legal, shortcuts, P2P) — mobbin 17
- [ ] D10. Toasts + banners (yt-dlp, Bilibili) — mobbin 18
- [ ] D11. Onboarding wizard — mobbin 14
- [ ] D12. Empty/loading/skeleton globais — mobbin 15, 16

### Hubs de plugins (redesign completo)

- [ ] D13. `/courses` + `/courses/[platform]`
- [ ] D14. `/convert`
- [ ] D15. `/telegram`
- [ ] D16. `/misc` + studio/library/file-clip

### Study (herda tokens globais; redesign dirigido nas superfícies representativas)

- [ ] D17. `/study` hub + layout próprio (alinhar ao shell)
- [ ] D18. `/study/player` + `/study/watch` — mobbin 21
- [ ] D19. `/study/read` (lista + leitor) — mobbin 21
- [ ] D20. `/study/notes` (lista + editor)
- [ ] D21. `/study/music` (hub + now-playing) — mobbin 21, 06
- [ ] D22. `/study/focus` + achievements + progress
- [ ] D23. `/study/anki` (hub + study)
- [ ] D24. Demais sub-rotas study: varredura de consistência (tokens/primitivos apenas)

## Fase final — verificação

- [ ] F1. check/test/build ≥ baseline (0 erros / ≤100 warnings / 23 testes / build ok)
- [ ] F2. Shots finais em `remake/shots/final/` + comparação com baseline
- [ ] F3. Contraste AA verificado em todos os pares texto/fundo dos 2 temas core
- [ ] F4. REMAKE-LOG.md completo (decisões, contornos, métricas finais)
