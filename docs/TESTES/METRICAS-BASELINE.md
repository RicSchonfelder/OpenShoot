# Métricas Baseline — OpenShoot × AfterShoot (macOS arm64)

> **Data:** 2026-08-24 · **Máquina:** macOS darwin/arm64 · **Suite:** 459 fotos reais
> (JPEG 8MP, evento noturno "Brotas/Editadas" — as mesmas fotos processadas pelo
> AfterShoot no álbum de 459 imagens).
> **Método OpenShoot:** benchmark automatizado via NAPI (`/tmp/os_bench.js`).
> **Método AfterShoot:** observação de UI durante auditoria funcional (tempos de
> import/culling exibidos pelo próprio app).

## 1. Resultados OpenShoot (medidos)

| Tarefa | Tempo | Detalhes |
|---|---|---|
| Setup catálogo | 9 ms | — |
| **Import 459 fotos** | ⚠️ **7.159 s (~2 h)** | 15,6 s/foto — arquivos em iCloud Drive (download on-demand) + scan serial + SHA-256 por arquivo |
| Listar grid (200) | 14 ms | ✅ instantâneo |
| Thumbnails 20 (cache frio) | 12 ms | ✅ ~1 ms/foto |
| **Culling IA 459 fotos** | ⚠️ **1.173 s (~19,5 min)** | 2,5 s/foto; avg score 42 (fotos noturnas); 0 erros |
| Filtros (picks/dup/faces) | 20 ms | ✅ picks 184 · duplicatas 0 · com rosto 141 |
| **Edição em lote 459** | ✅ 312 ms | receita completa (exposição+curva+contraste+sat) |
| **Export 20 JPEG q90** | ✅ 14,3 s | 0,7 s/foto, resolução nativa 6960×4640, edição aplicada |
| Export naming `{n}_{original}` | ✅ 3,2 s (5 fotos) | gerou `1__MG_7864.jpg` corretamente |
| **Face grouping 60 fotos** | 🔴 **3.541 s (~59 min)** | 59 s/foto! 25 grupos, 186 faces — decode full-res + SCRFD + embedding serial |
| **Retoque lote 5** (pele+acne) | ✅ 8,2 s | 1,6 s/foto, resolução nativa |
| XMP 459 sidecars | ✅ 156 ms | — |

## 2. AfterShoot (observado na auditoria funcional)

| Tarefa | Tempo observado | Fonte |
|---|---|---|
| Import 459 fotos | não cronometrado (wizard com barra de progresso; fotos já locais) | UI |
| Culling IA 459 | **~11 min estimado pelo app** ("11m 16s" exibido como ETA) | UI |
| Edição 459 ("Editar 459 Fotos") | não cronometrado (fila com progresso) | UI |
| Export | diálogo com opções; tempo não medido | UI |

## 3. Gaps de performance priorizados

| # | Gap | Causa raiz | Ação proposta |
|---|---|---|---|
| G1 | 🔴 Import 2 h | iCloud on-demand + scan serial + hash síncrono | Paralelizar scan (rayon, conexão por thread); detectar placeholder `.icloud` e baixar antes/avisar; hash opcional (lazy) |
| G2 | 🔴 Face grouping 59 min/60 | decode full-res por foto + SCRFD + embedding serial | Paralelizar com rayon; decodificar direto em 512px (evitar full-res); reusar embedding já calculado |
| G3 | 🟡 Culling 19,5 min vs ~11 min | decode triplo por foto (heurística 320px + ML 640px + faces 640px) | Decodificar 1× em 640px e reusar para os 3 estágios |
| G4 | 🟡 Summary do cull confuso | `picks` do summary = score≥70; filtro picks = rating≥4 (184 vs 0) | Alinhar summary ao rating (quantis), não ao limiar de score |
| G5 | 🟢 Export 0,7 s/foto | aceitável | paralelizado (rayon) já aplicado |

## 4. Como reproduzir

```bash
# Benchmark automatizado (não destrutivo — catálogo em /tmp):
node /tmp/os_bench.js          # suite completa (~2h+ por causa do import iCloud)
node /tmp/os_retest_export.js  # só export (rápido)
# Resultados: /tmp/os_bench_results.json
```

> ⚠️ As fotos de teste estão em iCloud Drive
> (`~/Library/Mobile Documents/com~apple~CloudDocs/Desktop/2026-08-15 - Brotas/Editadas`).
> Para benchmark de import SEM influência da rede, copie a pasta para o disco local
> primeiro e rode o import sobre a cópia (esperado: <2 min para 459 fotos).
