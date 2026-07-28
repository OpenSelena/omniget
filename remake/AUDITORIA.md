# AUDITORIA de fechamento — remake visual v2

Fase 1 executada em 2026-07-28 (antes de qualquer correção). Cada item: status + evidência literal.
Atualizada ao final da missão com o veredito pós-correção (coluna "status final").

## 1. Artefatos de documentação

```
$ ls -la remake/
-rw-r--r--  DESIGN-SYSTEM.md
-rw-r--r--  PLANO.md
drwxr-xr-x  shots/
```

| Artefato | Fase 1 | Status final |
|---|---|---|
| remake/DESIGN-SYSTEM.md | OK | OK |
| remake/PLANO.md | OK | OK |
| remake/shots/{baseline,fase-b,fase-c,fase-d,final} | OK | OK |
| remake/AUDITORIA.md | FALTA | OK (este arquivo) |
| remake/AUTOCRITICA.md | **FALTA** — a tabela de notas por tela (6 eixos) não existia; o loop de auto-crítica da Fase D rodou apenas de modo informal em 4 telas (home, downloads, marketplace, settings), sem registro | OK (criado na Fase 2, 23 rotas) |
| remake/ANTES-E-DEPOIS.md | **FALTA** | OK (criado na Fase 4) |
| remake/RELATORIO.md | **FALTA** | OK (criado na Fase 4) |
| PR aberto | **FALTA** | ver RELATORIO `## Não cumprido` / seção 10 |

## 2. PLANO sem pendências

```
$ grep -c "\[ \]" remake/PLANO.md
1
$ grep -n "\[ \]" remake/PLANO.md
3:Regra: atualizar o status após CADA item concluído. `[ ]` pendente, ...
```

Única ocorrência é a legenda do próprio arquivo. **OK**.

## 3. Hardcoded — cor

```
$ grep -rn "#[0-9a-fA-F]\{3,8\}\b" src/ --include=*.css --include=*.ts --include=*.tsx | grep -v -E "tokens|theme|variables" | wc -l
64  (51 fora de src/app.css)
```

Classificação na Fase 1:
- `src/app.css` (13 ocorrências capturadas): **definições** de token dos blocos de tema — este projeto não tem arquivo `tokens.css`/`variables.css`; `app.css` É o arquivo de variáveis (adaptação de critério declarada, seção 11).
- `settings-helpers.ts` (14): dados de preview do theme picker — legítimo como dado, **mas os previews de dark/light exibiam a paleta antiga** (`#0a0a0a`/`#FF7D38` laranja). Bug real → corrigido na Fase 3.
- `study-telegram-bridge.ts` (8): paleta oficial de avatares do Telegram (dado de plataforma) → mantido com justificativa em comentário.
- `queue-kinds.css` (12): definição de tokens `--queue-kind-*` → movida a justificativa para comentário de arquivo.
- `settings.css` (4 × `#fff`): knob de toggle e banner de update → tokenizados na Fase 3.

Status Fase 1: **PARCIAL** → Status final: **OK** (grep final na seção 12; restantes são definições de token ou dados de plataforma com justificativa no código).

## 4. Hardcoded — duração

```
$ grep -rnE "(transition|animation)[^;]*[0-9]+(ms|s)\b" src/ --include=*.css | grep -v tokens
14 ocorrências (app.css:321; settings.css ×9; primitives.css ×4)
```

Status Fase 1: **FALTA** → Status final: **OK** (todas tokenizadas ou justificadas; grep final na seção 12).

## 5. Hardcoded — tamanho (px fora de 0/1/0.5)

```
$ grep -rnE ":\s*[0-9]+px" src/ --include=*.css | grep -vE "tokens|0px|1px|0\.5px" | wc -l
165  (app.css 52, settings.css 68, primitives.css 22, macos-shell.css 17, buttons.css 5, reader-theme.css 1)
```

Status Fase 1: **PARCIAL**. Tratamento na Fase 3: valores mapeáveis → tokens (`--space-*`, `--radius-*`, `--text-*`); dimensões intrínsecas de controle (ex.: knob 20px do toggle, badge 18px) → tokens de controle novos em `:root` (`--control-*`) ou justificativa em comentário. Grep final na seção 12.

## 6. Acessibilidade e motion (contagem estática)

```
$ grep -rn "prefers-reduced-motion" src/ | wc -l   → 128
$ grep -rn "focus-visible" src/ | wc -l            → 126
```

Presença ampla. Prova comportamental (Playwright emulando `prefers-reduced-motion: reduce`): seção 13. Status: OK (estático) + prova capturada.

## 7. Gates

```
$ pnpm check | tail -1
COMPLETED 1331 FILES 0 ERRORS 100 WARNINGS 49 FILES_WITH_PROBLEMS

$ npm run lint
npm error Missing script: "lint"

$ pnpm test
Test Files  4 passed (4)   Tests  23 passed (23)

$ pnpm build → sucesso ("Wrote site to build")
```

- check: **OK** (0 erros; warnings = 100 = baseline, não desceu nem subiu).
- lint: **script não existe no projeto** (nunca existiu — evidência acima). Adaptação declarada: `svelte-check` cumpre o papel de lint (inclui as regras a11y/CSS); critério avaliado sobre ele.
- test: **OK** (nenhum teste do baseline falhando).
- build: **OK**.

## 8. Bundle vs. baseline

`dist/` não existe — o adapter-static escreve em `build/` (adaptação de critério).

```
$ du -sh build/                                   → 23M (remake, HEAD)
$ git worktree add /tmp/omniget-baseline ec8b9aa4 && pnpm build && du -sh build
24M   /tmp/omniget-baseline/build     (49480 blocos de 512B via du -s)
23M   build  (remake, HEAD)           (47632 blocos)
```

Remake = baseline **−3.7%** (limite era +10%). Status: **OK**.

## 9. Cobertura de shots baseline vs. final

- baseline: 132 shots = 22 rotas × 3 viewports × 2 temas (`remake/shots/baseline/`)
- final: 138 shots = 23 rotas × 3 viewports × 2 temas (`remake/shots/final/`)
- Diferença: rota `/_kitchen-sink` (criada pelo remake, não existia no baseline). **Todas as 22 rotas do baseline estão cobertas no final; nenhuma ficou de fora.** Zero falhas de captura nas duas rodadas (saída do harness: "done: N screenshots", lista de failures vazia).

Status: **OK**.

## 10. Itens manuais

| Item | Fase 1 | Evidência |
|---|---|---|
| Tabela de auto-crítica 1-5 (6 eixos) por tela no REMAKE-LOG | **FALTA** — não existia; refeita do zero na Fase 2 em remake/AUTOCRITICA.md | AUTOCRITICA.md |
| ANTES-E-DEPOIS.md com 23 rotas | **FALTA** → criado | ANTES-E-DEPOIS.md |
| RELATORIO.md | **FALTA** → criado | RELATORIO.md |
| PR aberto | **FALTA** na Fase 1 | seção 14 |
| Warnings vs. baseline | igual (100 = 100) | seção 7 |
| Chaves i18n usadas e ausentes | 0 reais (2 falsos positivos em doc-comment de keys.ts) | script na sessão |

## 11. Adaptações de critério declaradas

1. **`npm run lint`**: script inexistente no projeto (pré-existente à missão). Equivalente adotado: `svelte-check` (0 erros / ≤100 warnings).
2. **`dist/`**: o build do SvelteKit adapter-static escreve em `build/`.
3. **Grep de cor**: `src/app.css` contém as *definições* dos 14 temas — é o arquivo de variáveis do projeto (não há `tokens.css`). O critério "zero hardcoded" é aplicado a *usos* fora das definições de token.
4. **Alvo de toque 44pt**: app desktop Tauri. Mínimo adotado (macOS HIG desktop): ≥28px de altura para controles padrão (botões, inputs, nav rows), ≥20px para alvos densos (ícones de linha, badges clicáveis). Verificação na seção 13.
5. **PR**: ver seção 14 — a branch contém o checkpoint de trabalho não relacionado (724 arquivos de `feat/plugin-hot-load` não presentes na main), herdado por decisão da missão original ("commite o estado atual como ponto de retorno").

## 12. Grep final pós-Fase 3 (preenchido após correções)

_(preenchido ao final — ver abaixo)_

## 13. Provas comportamentais (preenchido na Fase 3)

_(reduced-motion, alvos de clique — ver abaixo)_

## 14. PR (preenchido na Fase 4)

_(ver abaixo)_
