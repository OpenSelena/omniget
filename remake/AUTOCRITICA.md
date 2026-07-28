# AUTOCRÍTICA visual — remake v2 (Fase 2 da auditoria)

Método: para cada rota, abertos os shots finais 1440×900 (light e dark) + ≥4 referências da(s) pasta(s) `mobbin/` do padrão correspondente, avaliação 1–5 em seis eixos. Eixo < 4 → correção, recaptura, reavaliação (máx. 3 iterações), mantendo o valor inicial registrado como `X→Y`.

Eixos: hier(arquia) · respiro · alinh(amento) · tipo(grafia) · acab(amento) · HIG (fidelidade).

| rota | hier | respiro | alinh | tipo | acab | HIG | notas |
|---|---|---|---|---|---|---|---|
| `/` home | 4 | 4 | 4 | 4 | 4 | 4 | Refs: Obsidian, Craft, Apple Music, Fabric (04/05/06). URL field é o herói; azul só em seleção/badge. Observações não bloqueantes: banner de manutenção disputa o primeiro olhar (aceito: é temporário e dispensável) e o ícone ⓘ do hint fica visualmente órfão sob o input. |
| `/downloads` | 4 | 4 | 4 | 4 | 4 | 4 | Refs: Apple Podcasts, Apple Fitness, Spotify, Breaker (07/08). Faixa lateral de cor eliminada; status como tag; barra de 100% redundante removida. Medição de coordenadas confirmou header e lista na mesma coluna de 800px (falso alarme de alinhamento na 1ª passada — registrado por honestidade). |
| `/marketplace` | 4 | 4 | 4 | 4 | 4 | 4 | Refs: App Store ×4 (13). Cards hairline, ação quieta, tag accent discreta, i18n corrigida. Observação: tag "In the sidebar" + label do toggle são semanticamente redundantes (aceito: a tag some quando o plugin está oculto e vira o sinal de estado). |
| `/settings` | 4 | 4 | 4 | 5 | 4 | 5 | Refs: Linear ×2 (09/10), Framer (11), Bear (12). Estrutura idêntica ao padrão Linear: nav agrupada + drill rows com descrição + chevron; busca no topo; caps headers. Sub-views de drill auditadas por código (C-003). |
| `/about` | 4 | 4 | 4 | 4 | 3→4 | 4 | Refs: App Store ×2, Bear ×2 (19). Iteração 1: 7 chaves i18n cruas (`about.tab.overview`, `card_*`) corrigidas + recaptura. Hero com ícone + versão em pill segue o padrão App Store. |
| `/about/changelog` | 4 | 4 | 4 | 4 | 3→4 | 4 | Iteração 1: renderer não tratava cercas ``` nem `---` (backticks crus na tela); fence→`<pre>` estilizado, `---`→hairline; recapturado nos 2 temas. |
| `/about/project` | 4 | 4 | 4 | 4 | 4 | 3→4 | Iteração 1: botão "star on GitHub" em laranja sólido violava a One Accent Rule (Don't do explícito do DESIGN.md); virou secundário quieto com estrela tingida; recapturado. |
| `/about/terms` | 4 | 4 | 4 | 4 | 4 | 4 | Cards de leitura com títulos headline e corpo confortável; sem correção. |
| `/about/privacy` | 4 | 4 | 4 | 4 | 4 | 4 | Mesma família visual do terms (verificado light+dark); sem correção. |
| `/courses` | 4 | 4 | 4 | 4 | 4 | 4 | Refs: Things 3 ×2, Apple Fitness ×2 (01/02). Grid de plataformas com brand colors legítimas; aviso de update quieto; busca .input. |
| `/convert` | 4 | 4 | 4 | 4 | 4 | 4 | Coluna única centrada (foco em tarefa única, aceitável); 1ª leitura acusou "título gigante", medição de coordenadas mostrou .page-title correto (registrado por honestidade). rgba hardcoded do badge hwaccel tokenizado nesta fase. |
| `/telegram` | 4 | 4 | 4 | 4 | 4 | 4 | Estado mockado (loading chats); chrome (toolbar de conta, sync pill, ações) coerente com o sistema nos 2 temas. Estado feliz exige DLL real (D-007/C-002). |
| `/misc` | 4 | 4 | 4 | 4 | 4 | 4 | Cards de ferramenta com ícone tintado + tags BETA/NEW discretas; 2 temas ok. |
| `/misc/studio` | 4 | 4 | 4 | 5 | 4 | 4 | Painel de stats em mono tabular (movimento Apple Fitness); botão destrutivo desabilitado legível; 2 temas ok. |
| `/misc/library` | 4 | 4 | 4 | 4 | 3* | 4 | *Sob mock, o comando de plugin nulo vaza exceção JS crua ("Cannot read properties of null") + "Loading…" simultâneo — viola a regra de copy de erro. Causa é o mock (backend real fornece dados), mas a robustez de copy é dívida real registrada no RELATORIO (correção exige tocar o catch — lógica, fora do guardrail visual). Chrome da tela (título, busca, pills) ≥4. |
| `/misc/file-clip` | 4 | 4 | 4 | 4 | 4 | 4 | Card único com ação + hint em footnote; 2 temas ok. |
| `/study` | 4 | 4 | 4 | 4 | 4 | 4 | Refs: Apple Notes, Apple Music, Craft, Audible (21). Saudação + focus card + sub-nav alinhada ao shell; 2 temas ok. |
| `/study/player` | 4 | 4 | 4 | 4 | 4 | 4 | Empty state direciona ("Select a lesson from the library"); breadcrumb correto; 2 temas ok. |
| `/study/read` | 4 | 4 | 2→4 | 4 | 4 | 4 | Iteração 1: h1 com flex:1/min-width:0 colapsava e o texto vazava por baixo do botão "Folders" (colisão real nos 2 temas). Fix: flex 0 0 auto + margin-right auto; toolbar quebra em 2ª linha; recapturado light+dark. |
| `/study/library` | 4 | 4 | 4 | 4 | 4 | 4 | Header + segmented + busca coerentes; seção "Continue where you left off" vazia sob mock (sem dados) — chrome ok nos 2 temas. |
| `/study/music` | 4 | 4 | 4 | 4 | 2→4 | 2→4 | Iteração 1: superfície music era dark-only (branco hardcoded sobre fundo claro: título, tiles, headings, skeletons ilegíveis no light). Tokenizados +page e 6 componentes da home (NavigationTitle, SpeedDialGridItem, EmptyPlaceholder, YoutubeSkeleton, AboutLink, ChipsRow) — 30+ hardcodes → tokens; recapturado. Restante das 33 sub-rotas do music: dívida declarada no RELATORIO (~280 hardcodes em componentes profundos). |
| `/study/watch` | 4 | 4 | 4 | 4 | 4 | 4 | Painel de player com placeholder e hint de pasta; 2 temas ok. |
| `/_kitchen-sink` | 5 | 4 | 4 | 4 | 4 | 4 | Superfície de auditoria: todos os primitivos × estados verificados nos 2 temas (light nesta fase; dark validado na captura da Fase C com layout idêntico). Serve de prova de foco visível e estados obrigatórios. |
