# ANTES E DEPOIS — remake visual v2

Shots em `remake/shots/baseline/` (antes) e `remake/shots/final/` (depois); nomes seguem `<rota>__<viewport>__<tema>.png` (3 viewports × 2 temas por rota). Os caminhos abaixo citam a variante 1440×900 dark; as demais ficam ao lado.

| Rota | Antes | Depois | O que mudou | Referência mobbin que guiou |
|---|---|---|---|---|
| `/` | baseline/home__1440x900__dark.png | final/home__1440x900__dark.png | Accent único azul (laranja removido), segmented Normal/Advanced quieto (era CTA-fill), foco do omnibox com anel suave em vez de glow, tipografia System 13px | 04 Raycast (campo único como herói), 05 Craft/Apple Mail (inspector e hierarquia), 06 Apple Music |
| `/downloads` | baseline/downloads__1440x900__dark.png | final/downloads__1440x900__dark.png | Faixa lateral colorida removida (anti-padrão), status como tag em caixa normal, cards compactos com hairline, barra 100% redundante oculta, ações alinhadas à coluna | 07 Apple Podcasts/Breaker (linhas de episódio), 08 Apple Fitness (dados tabulares) |
| `/marketplace` | baseline/marketplace__1440x900__dark.png | final/marketplace__1440x900__dark.png | Cards estilo App Store (hairline, ação quieta, tag accent), fundo azulado removido, Uninstall rebaixado, 6 chaves i18n cruas corrigidas | 13 App Store (hierarquia do card e botão GET) |
| `/settings` | baseline/settings__1440x900__dark.png | final/settings__1440x900__dark.png | Grupos caps estilo Linear, drill rows com descrição+chevron, px legados → tokens, cards com borda hairline | 09/10 Linear (estrutura e drill), 11 Bear/Height/Framer (rows) |
| `/about` | baseline/about__1440x900__dark.png | final/about__1440x900__dark.png | Hero com versão em pill, cards de navegação como list-rows, 7 chaves i18n cruas corrigidas, links externos como botões quietos | 19 App Store (What's New) e Bear (about) |
| `/about/changelog` | baseline/about-changelog__1440x900__dark.png | final/about-changelog__1440x900__dark.png | Renderer com cercas de código em `<pre>` e `---` como hairline (antes vazava backticks), títulos tokenizados | 19 App Store (release notes) |
| `/about/project` | baseline/about-project__1440x900__dark.png | final/about-project__1440x900__dark.png | Botão "star on GitHub" laranja sólido → secundário quieto (One Accent Rule) | 19 App Store, DESIGN.md Don'ts |
| `/about/terms` | baseline/about-terms__1440x900__dark.png | final/about-terms__1440x900__dark.png | Cards de leitura com headline + corpo confortável, herda tokens | 19 Bear |
| `/about/privacy` | baseline/about-privacy__1440x900__dark.png | final/about-privacy__1440x900__dark.png | Mesma família do terms; contraste AA nos dois temas | 19 Bear |
| `/courses` | baseline/courses__1440x900__dark.png | final/courses__1440x900__dark.png | Grid de plataformas em cards hairline, busca .input, aviso de update quieto | 01 Things 3 (grid navegável), 13 App Store |
| `/convert` | baseline/convert__1440x900__dark.png | final/convert__1440x900__dark.png | Coluna única com tokens, badge de hwaccel tokenizado, escala tipográfica corrigida | 11 Framer (form de tarefa única) |
| `/telegram` | baseline/telegram__1440x900__dark.png | final/telegram__1440x900__dark.png | Chrome de conta (pill de sync, ações, sign out) no sistema v2; estado feliz depende da DLL real | 12 App Store (linhas de conta) |
| `/misc` | baseline/misc__1440x900__dark.png | final/misc__1440x900__dark.png | Cards de ferramenta com ícone tintado e tags BETA/NEW discretas | 01/02 Things/Fitness (hub) |
| `/misc/studio` | baseline/misc-studio__1440x900__dark.png | final/misc-studio__1440x900__dark.png | Painel de stats em mono tabular, botões de gravação com estados legíveis | 08 Apple Fitness (painel de métricas) |
| `/misc/library` | baseline/misc-library__1440x900__dark.png | final/misc-library__1440x900__dark.png | Busca + pills de tipo no sistema v2; erro cru do mock registrado como dívida de copy | 07 Breaker (lista de mídia) |
| `/misc/file-clip` | baseline/misc-file-clip__1440x900__dark.png | final/misc-file-clip__1440x900__dark.png | Card único de ação + hint em footnote | 11 Framer |
| `/study` | baseline/study__1440x900__dark.png | final/study__1440x900__dark.png | Saudação + focus card + sub-nav alinhada ao shell; CTA azul único | 21 Craft (hub de conteúdo) |
| `/study/player` | baseline/study-player__1440x900__dark.png | final/study-player__1440x900__dark.png | Empty state direcionando à biblioteca; breadcrumb | 21 Apple Music |
| `/study/read` | baseline/study-read__1440x900__dark.png | final/study-read__1440x900__dark.png | Colisão título×toolbar corrigida (h1 colapsava com flex:1/min-width:0), toolbar quebra em 2ª linha | 21 Apple Notes/ElevenReader (biblioteca de leitura) |
| `/study/library` | baseline/study-library__1440x900__dark.png | final/study-library__1440x900__dark.png | Header + segmented + busca no sistema v2 | 21 Craft |
| `/study/music` | baseline/study-music__1440x900__dark.png | final/study-music__1440x900__dark.png | Superfície era dark-only: título, tiles, headings e skeletons ilegíveis no light — home tokenizada (30+ hardcodes em 7 arquivos) | 21/06 Apple Music (now-playing e navegação) |
| `/study/watch` | baseline/study-watch__1440x900__dark.png | final/study-watch__1440x900__dark.png | Painel do player com placeholder e hint de pasta no sistema v2 | 21 Apple Music/Audible |
| `/_kitchen-sink` | — (rota criada pelo remake; sem baseline) | final/_kitchen-sink__1440x900__dark.png | Superfície de auditoria com todos os primitivos × estados nos 14 temas | 11/17/18 (Bear, Apple News, Raycast) |
