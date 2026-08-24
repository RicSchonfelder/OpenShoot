# OpenShoot — Progresso de Desenvolvimento

> Arquivo de continuidade: registra o estado atual, o que funciona, o que falta e
> possíveis pontos de retomada caso o ambiente/agente reinicie.
> **Ações pendentes:** ver `docs/ROADMAP.md` (fonte única de tarefas).

**Última atualização:** 2026-08-24

## Estado atual

### Concluído (visão consolidada)
- **Base (Fases 0-6)** ✅ — Electron+React ⇄ napi-rs ⇄ ONNX/CoreML; catálogo SQLite;
  RAW decode (NEF/ARW/CR3 via parser BMFF); culling IA (NIMA+SCRFD, quantis → ★1-5);
  XMP sidecar LR/C1; edição em lote; retoque; captions locais; empacotamento macOS.
- **Álbuns + fluxo** ✅ — Tela Lar (grid de álbuns com capa/contagem), criar/deletar
  álbum, abas **IMPORT → CULL → EDIT → RETOUCH** por álbum, tipo de sessão,
  import wizard (subpastas + tipo de fotos), "Um clique" (cull + preset).
- **Culling completo** ✅ — ★1-5 clicável, P/X/U, loupe com patch por arrasto e
  moldura de rosto, flags coloridos, meta de nº de picks, buckets "Para revisão" /
  "Destaques IA" / "Selecionado", painel de filtros com contagens vivas, filtros
  duplicatas (sha256) / rosto / orientação / tipo de arquivo / status de edição.
- **Edição completa** ✅ — 8 sliders + curva de tom (4 pontos) + HSL (8 cores) +
  nitidez (unsharp) + redução de ruído (bilateral) + horizonte IA (Hough) +
  recorte IA (faces) + máscara de sujeito (fundo desfocado) + presets nomeados
  com regras (file_type/color_type/source) + aprender perfil (média de edições) +
  importar preset Lightroom (.xmp/.lrtemplate) + mercado JSON (export/import).
- **Retoque completo** ✅ — Pele (YCbCr), acne/olhos/dentes/cabelo por região da
  bbox facial, patch por arrasto, **aplicar retoque em lote ("Colar")**.
- **Exportação** ✅ — Diálogo destino/tipo (JPEG-PNG)/qualidade, edição aplicada +
  orientação EXIF, resolução nativa, sufixo de conflito, **paralela (rayon)**,
  espaço de cor (sRGB / P3 aproximado), templates de nomeação.
- **Reconhecimento facial** ✅ — MobileFaceNet embeddings + agrupamento por
  similaridade + **UI Pessoas** (agrupar/exportar pastas por pessoa) + olhos
  fechados (funções `detect_faces_with_kps`/`eyes_open_score` — integração ao
  culling pendente, ver ROADMAP P2).
- **Extras** ✅ — Labels de cor com menu de contexto, zoom Fit/100% no loupe,
  galeria web estática exportável, modo edição em tela grande, orientação EXIF,
  i18n pt-BR/en, CI GitHub Actions, README/THIRD_PARTY.
- **Qualidade** ✅ — **60 testes Rust**, typecheck limpo, ~11k linhas.

### Como os 10 agentes paralelos contribuíram (2026-08-24)
Jobs `os-agent` (opencode CLI, modelo x-preview-f-free): Pessoas, olhos fechados,
sRGB/nomeação, perfis com regras, galeria web, zoom loupe, CI, docs, export
paralela, labels de cor — integrados e validados (commit `c77a57d`).
  - **Diálogo de deletar 3 opções**: remover só do catálogo / mover p/ Lixeira /
    cancelar (`removePhotoFromCatalog` / `deletePhoto` com `move_to_trash` manual).
  - **⌘A/Ctrl+A** seleciona todas.
  - **Filtros avançados** (dropdown "Outros"): **Duplicatas** (sha256 agrupado),
    **Com rosto** (`has_face` via SCRFD), **Para revisão** (`review`, score 55-70),
    **Orientação** (retrato/paisagem), **Tipo de Arquivo** (RAW/JPEG-TIFF) +
    botão **Reiniciar filtros**.
  - **Meta de nº de picks**: `cullPhotos(targetPicks)` marca as top-N fotos como
    ★5 + slider "Meta de seleção" na toolbar (0 = sem meta/limiar 70).
  - **Edição completa (Tranche B)** ✅ — presets nomeados, **curva de tom**
    (4 pontos), **HSL** (8 cores × matiz/sat/lum), **nitidez** (unsharp) +
    **redução de ruído** (bilateral).
  - **Perfis de IA v1 (Tranche E)** ✅ — **Aprender perfil** (média dos
    parâmetros de edição → preset) e **Importar preset Lightroom** (.xmp/.lrtemplate).
- **Core** ✅ — 43 testes Rust passando; typecheck limpo (main/preload/renderer).

### Failures / pontos de atenção
- **RAW preview** (CR3/NEF/ARW/DNG):
  - NEF/ARW/DNG/CR2 (TIFF-based): **funciona** via `read_embedded_jpeg` (tags
    JPEGInterchangeFormat 0x201/0x202), iterando TODOS os IFDs.
  - **CR3** ✅ funciona via `core/src/cr3.rs` (parser BMFF/HEIF, brand 'crx ').
  - **`jpgfromraw-lib` FALHA de build** (exige nasm/CMake) → **descartado**.
- `setup()` usa `OnceLock` por processo — cada processo Node novo precisa chamar
  `setup()`.
- **userData**: preview = `~/Library/Application Support/openshoot` (nome do
  package.json); empacotado = `~/Library/Application Support/OpenShoot`.
- **Delegações para subagentes não funcionam** (permissão `external_directory`
  auto-rejeitada p/ subagentes) — fazer mudanças no contexto principal.
- Dimensões (width/height) podem vir 0 p/ PNG sem EXIF.
- react-window é a **v2.x** (API nova: `Grid` + `cellComponent`/`cellProps` +
  `gridRef`, `defaultWidth`/`defaultHeight` + `onResize`).

## Decisões técnicas tomadas
- Stack: Electron/React ⇄ IPC ⇄ Rust core via **napi-rs**.
- kamadak-exif (BSD-2-Clause) para EXIF. Preview RAW via tags JPEGInterchangeFormat.
- crate `image` 0.25 para thumbnails. react-window 2.x para virtualização.
- `jpgfromraw-lib` (MIT) rejeitado (build script exige ferramentas C).
- **Culling heurístico**: Laplacian variance + exposição + histograma, rayon paralelo.
- **XMP**: template Lightroom-compatível (xpacket UUID `W5M0MpCehiHzreSzNTczkc9d`,
  xmp:Rating 0-5, xmp:Label Red/Yellow/Green/Blue/Purple, dc:subject Bag).
- **Duplicatas**: agrupamento por `sha256` (coluna já existente), filtro SQL
  `GROUP BY HAVING COUNT(*) > 1` + NAPI `findDuplicates()`; filtro "faces" usa
  coluna `has_face` populada no culling (SCRFD).

## Auditoria referência externa (2026-08-18/20)
- Documento de referência: `docs/AUDITORIA-referência externa.md` (mapa completo + gap
  analysis com 15 itens priorizados). Implementados até agora:
  ★1-5 por foto, filtros avançados (dropdown), detecção de duplicatas, loupe,
  flags, i18n, toolbar de culling.

## Próximos passos
Ver **`docs/ROADMAP.md`** (fonte única, priorizado). Resumo:
1. 🔴 P1 — Portabilidade Windows/Linux (EP de IA, caminhos, lixeira, electron-builder, CI matriz).
2. 🔴 P2 — Olhos fechados integrado ao culling + filtros de aviso.
3. 🟡 P3-P5 — Filtros por cor, refinamentos de exportação, assinatura/auto-update.
4. 🟢 P6-P8 — E2E, OpenRouter opt-in, polimento de UX.

## Como retomar (recuperação de crash)
1. `cd ~/OpenShoot`
2. `npm install` (aprovar scripts electron/esbuild/fsevents se pedir)
3. `npm run build:core` — regenera o addon `.node`
4. `npm run dev` — sobe o app
5. `npm run typecheck` e `npm test` — validação

## Notas de ambiente
- Máquina: macOS (darwin/arm64). Rust 1.97.1, Node 24.18.
- addon: `core/openshoot_core.darwin.arm64.node` (gitignored).
- userData app: `~/Library/Application Support/openshoot` (contém catalog.db).
- Dependencias novas: react-window 2.x, @types/react-window.
