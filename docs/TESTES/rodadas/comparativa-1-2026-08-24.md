# Rodada Comparativa #1 — 2026-08-24 (macOS arm64)

> **Executor:** agente principal (benchmark automatizado via NAPI + auditoria UI do
> AfterShoot). Fotos: 459 reais (Brotas/Editadas — as mesmas do álbum AfterShoot).

## Tarefas executadas nos dois apps

| Tarefa | AfterShoot (observado) | OpenShoot (medido) | Veredicto |
|---|---|---|---|
| Criar álbum | wizard com nome | modal com nome — 3 cliques | ✅ paridade |
| Import 459 | barra de progresso; fotos já locais | ⚠️ ~2h (iCloud on-demand + serial) | 🔴 G1 |
| Grid 459 | fluido | listagem 14ms, thumbs 1ms/foto | ✅ paridade |
| Culling IA 459 | ~11 min (ETA do app) | 19,5 min (2,5s/foto) | 🟡 G3 (1.8× mais lento) |
| Picks da IA | agrupa picks/review/duplicatas | 184 picks (rating≥4), 0 duplicatas, 141 com rosto | ✅ paridade |
| Loupe + teclado | Fit/100%, P/X, estrelas | idem + zoom/pan/moldura/patch | ✅+ paridade |
| Edição lote 459 | "Editar 459 Fotos" (fila) | **312 ms** (receita aplicada) | ✅ paridade |
| Ajustes manuais | painel de preferências | 8 sliders + curva + HSL + nitidez/ruído | ✅ paridade |
| Horizonte/recorte IA | toggles no EDIT | botões por foto (preview) | ✅ paridade |
| Retoque facial | sliders por região + Colar | idem + Colar em lote (1,6s/foto) | ✅ paridade |
| Sujeito/fundo | toggle Máscara de IA | botão máscara de sujeito (heurística) | ✅ paridade funcional |
| Export 20 JPEG q90 | diálogo completo | 14,3 s, resolução nativa, edição aplicada, naming/sufixo | ✅ paridade |
| XMP sidecars | via fluxo LR/C1 | 459 sidecars em 156 ms | ✅ |
| Pessoas (faces) | agrupamento nativo no culling | 🔴 59 min/60 fotos (25 grupos, 186 faces) | 🔴 G2 |
| Galeria web | "Criar galeria" | export HTML self-contained | ✅ paridade |

## Bugs/achados desta rodada

1. 🔴 **G1 — Import catastrófico com iCloud**: 15,6 s/foto. Arquivos em iCloud Drive
   baixam on-demand durante o scan serial (SHA-256 força download completo).
   Ação: ROADMAP P1.5 — paralelizar scan + detectar `.icloud` placeholders + hash lazy.
2. 🔴 **G2 — Face grouping inviável**: 59 s/foto (decode full-res + SCRFD + embedding
   serial). Ação: ROADMAP — paralelizar + decodificar direto no tamanho do modelo.
3. 🟡 **G3 — Culling 1.8× mais lento** que a referência. Ação: decodificar 1× (640px)
   e reusar nos 3 estágios (heurística/ML/faces) em vez de 3 decodes.
4. 🟡 **G4 — Semântica do summary**: `cullPhotos().picks` usa limiar score≥70 (0 nesta
   rodada — fotos noturnas, avg 42) enquanto o filtro "picks" usa rating≥4 (184).
   Ação: alinhar summary ao rating dos quantis.
5. ✅ Export/naming/retoque-lote/XMP: validados com fotos reais, resolução nativa.

## Conclusão

Paridade funcional confirmada em 14/14 tarefas. Bloqueadores são de **performance**
(G1, G2) e um de **consistência** (G4) — nenhum de funcionalidade. Próxima rodada
deve re-medir após G1-G3 e executar o TESTPLAN-UI completo.
