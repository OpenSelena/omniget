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
