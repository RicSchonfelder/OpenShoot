# OpenShoot — Roadmap de Paridade com o AfterShoot

> **Objetivo:** deixar o OpenShoot com o mesmo conjunto de features/UX do AfterShoot
> (v2.21.4, auditado em 2026-08-18/20). Cada item = feature do AfterShoot → plano de
> implementação no OpenShoot, com prioridade e esforço.
> **Fonte:** `docs/AUDITORIA-AFTERSHOOT.md` (mapa completo + gap analysis).
> **Atualizado:** 2026-08-21

**Legenda de status:** ✅ feito · 🟡 em andamento · ⬜ pendente

---

## Tranche A — Culling & Seleção (núcleo, alto valor / baixo custo)

| AfterShoot | OpenShoot | Status |
|---|---|---|
| Culling IA (score + rating ★1-5) | NIMA + SCRFD + heurística → quantis ★1-5 | ✅ |
| ★1-5 clicável por foto no grid | `cell-stars` (5 botões, clique zera) | ✅ |
| Atalhos P/X/U/1-5 (culling rápido) | `p`/`x`/`u`/`1-5` aplicam e avançam | ✅ |
| Loupe / revisão (foto grande) | `LoupeView` (duplo clique/Enter, setas, Esc) | ✅ |
| Flags P verde / X vermelho | `flag-pick`/`flag-reject` no grid | ✅ |
| Toolbar com contadores P/X/U | `cull-toolbar` | ✅ |
| Filtro "Sem classificação" | filtro `unrated` | ✅ |
| Filtros "Duplicatas" | filtro `duplicates` (sha256) + `findDuplicates()` | ✅ |
| Filtros "Com/Sem rosto" | filtro `faces` (`has_face` via SCRFD) | ✅ |
| Dropdown "Outros" (filtros avançados) | dropdown no App.tsx | ✅ |
| Deleção (catálogo vs lixeira) | diálogo 3 opções | ✅ |
| **Meta de nº de picks** ("quantas selecionar") | `cullPhotos(targetPicks)` + slider na toolbar | ✅ |
| **Bucket "Para revisão"** (fotos ambíguas) | coluna `review` (score 55-70) + filtro + contador | ✅ |
| **Filtro Tipo de Arquivo** (RAW / JPEG-TIFF) | filtros `raw`/`jpeg` no dropdown | ✅ |
| **Filtro Orientação** (retrato/paisagem) | filtros `portrait`/`landscape` no dropdown | ✅ |
| **Reiniciar filtros** | botão reset no dropdown | ✅ |
| **"Destaques IA" vs "Selecionado manual"** | ⬜ separar origem do rating (IA vs manual) |
| **Filtro "Editar status"** | ⬜ coluna `edited` + filtro |
| **Contadores vivos no painel de filtros** | ⬜ painel lateral com contagens por bucket |

## Tranche B — Edição (expansão tonal)

| AfterShoot | OpenShoot | Status |
|---|---|---|
| 8 sliders básicos (exposição, WB, contraste, sat, sombras, realces, brilho) | `edit.rs` + `EditPanel` | ✅ |
| Aplicar em lote + preview | `applyEditAll`/`applyEditOne` | ✅ |
| **Presets nomeados** (salvar/carregar receita JSON) | tabela `presets` + NAPI + UI | ✅ |
| **Curva de tom** (destaques/luzes/escuros/sombras) | `tone_curve` + sliders | ✅ |
| **HSL** (8 cores × matiz/sat/lum) | `hsl` [24] + seletor de cor | ✅ |
| **Nitidez** (unsharp mask) | `sharpen` | ✅ |
| **Redução de ruído** (bilateral leve) | `denoise` | ✅ |
| **Ajuste de horizonte com IA** | Hough `autoLevelPhoto` | ✅ |
| **Recorte por IA** (suave/padrão) | faces + centralização `aiCropPhoto` | ✅ |
| **Máscara de IA** (sujeito/fundo) | ⬜ SelfieSegmentation ONNX |
| **Perfil de IA por álbum** (estilo aprendido) | ⬜ v1: média de tom de amostras |

## Tranche C — Importação (wizard)

| AfterShoot | OpenShoot | Status |
|---|---|---|
| Import via drag-and-drop + navegar + recentes | `pickFolder` (dialog nativo) | 🟡 |
| **Wizard: tipo de fotos** (RAW/JPEG/TIFF) | modal de importação (subpastas + tipo) | ✅ |
| **Incluir subpastas** (toggle) | checkbox + `max_depth(1)` | ✅ |
| **"Começar" one-click** (cull+edit) | botão "Um clique" (cull + preset) | ✅ |
| Progresso + contador "X/Y" | `scanFolderProgress` | ✅ |
| **Tipo de sessão** (casamento, retrato, família...) | ⬜ seletor de gênero |

## Tranche D — Retoque (expansão)

| AfterShoot | OpenShoot | Status |
|---|---|---|
| Suavização de pele | YCbCr + blur seletivo | ✅ |
| Remover distração (inpainting) | difusão, bbox central | ✅ |
| **Patch com seleção por arrasto** | arrasto no loupe + overlay | ✅ |
| **Sliders faciais** (acne, olhos, dentes, cabelo) | `retouch_face_region` + NAPI | ✅ |
| **Mostrar moldura do rosto** | toggle no loupe (overlay de bbox SCRFD) | ✅ |
| **Modos SUJEITO/FUNDO/PATCH** | máscara de sujeito (face+pele nítido, fundo desfocado) | ✅ |
| **Colar / Redefinir** (retoque em lote) | "Aplicar retoque em lote (Colar)" → grava cópias retocadas das fotos selecionadas | ✅ |

## Tranche F — Exportação (teste comparativo 2026-08-21)

Auditoria do diálogo de exportação do AfterShoot (álbum Editadas, 459 fotos) e
implementação equivalente no OpenShoot:

| AfterShoot | OpenShoot | Status |
|---|---|---|
| Diálogo Exportar N Foto(s) com aba Pasta/Outros Apps | diálogo com destino/tipo/qualidade | ✅ |
| Destino + arquivos em conflito (sufixo automático) | `export_photos` com sufixo _1/_2 | ✅ |
| Tipo de imagem JPEG / Quality slider LOW→BEST | JPEG/PNG + slider 1-100 | ✅ |
| Exporta com edição aplicada | aplica edit_json + orientação EXIF, resolução nativa (6960x4640 validado) | ✅ |
| Dica "⌘A para selecionar todas" | dica no diálogo | ✅ |
| Retoque gravado na exportação | applyRetouchAll (pele+faces) grava cópias retocadas | ✅ |
| Espaço de cor (sRGB) | ⬜ pendente menor |
| Nomeação de ficheiros (templates) | ⬜ pendente menor |
| Aba "Outros Apps" (LR/C1/PS) | ⬜ via XMP sidecars (fluxo existente) |

## Tranche E — Perfis de IA (o mais pesado)

| AfterShoot | OpenShoot | Status |
|---|---|---|
| Perfil Profissional (treinar com 2.500 fotos editadas LR/C1) | ✅ v1: **Aprender perfil** (média dos parâmetros de edição das fotos → preset "Perfil aprendido") |
| Perfil Instantâneo (preset Lightroom) | ✅ **Importar preset LR** (.xmp crs: / .lrtemplate → receita) |
| **Mercado de perfis** | ✅ exportar/importar estilo como JSON (estilos compartilháveis) |
| Regras (um tipo de arquivo/cor/catálogo por perfil) | ⬜ metadados do perfil |

---

## Priorização sugerida (execução)

1. ✅ **Tranche A** — meta de picks, "Para revisão", "Destaques" vs "Selecionado",
   painel de filtros com contadores + Reiniciar, filtros Tipo de Arquivo/Orientação/
   Câmera/Editar status.
2. ✅ **Tranche B** — presets nomeados, curva de tom, HSL, nitidez, redução de ruído,
   horizonte IA, recorte IA. (Pendente: máscara de sujeito — requer SelfieSegmentation.)
3. ✅ **Tranche C** — wizard de importação (subpastas + tipo), "Um clique".
   (Pendente: tipo de sessão/gênero.)
4. ✅ **Tranche D** — patch por arrasto, sliders faciais.
   (Pendente: moldura do rosto.)
5. 🟡 **Tranche E** — aprender perfil de XMPs, importar preset LR. 
   (Pendente: mercado de perfis, metadados de regras.)

> Cada item novo segue as regras do AGENTS.md: typecheck + cargo test verdes,
> DESIGN.md atualizado, novo código Rust com teste.
