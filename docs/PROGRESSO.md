# OpenShoot — Progresso de Desenvolvimento

> Arquivo de continuidade: registra o estado atual, o que funciona, o que falta e
> possíveis pontos de retomada caso o ambiente/agente reinicie.

**Última atualização:** 2026-08-18

## Estado atual

### Concluído
- **Fase 0** ✅ — Esqueleto Electron (React/TS) + core Rust (napi-rs) com ponte IPC
  validada E2E. Repo público no GitHub.
- **Fase 1** ✅ — Catálogo + decode + thumbnails + grid virtualizado:
  - `core/src/catalog.rs` — Catálogo SQLite (`photos`), schema, upsert, listagem
    com paginação/busca, `scan_folder` recursivo com `walkdir`.
- **Fase 2 (heurístico + ML local)** ✅ — Culling com IA local (ONNX):
  - **SCRFD multi-escala + NMS** implementado e validado: detecta 1 rosto na
    imagem Lena ([0.435,0.404,0.721,0.793]) e 0 em gradientes (discriminativo).
    Outputs SCRFD: score_s [1,N,1] fg, bbox_s [1,N,4], N=(640/stride)^2*2 anchors.
    Correção: normalizar por `scale*width/height` (não só scale) p/ coords 0..1.
  - **NIMA** (estética, NHWC, score 1-10) + **SCRFD** (detecção de faces, NCHW)
  - Engine **ort 2.0.0-rc.13** com EP **CoreML** (GPU/ANE) + fallback CPU
  - Score final: heurística (Laplacian) + NIMA + bônus por rostos; fallback p/ heurística se modelo ausente
  - Modelos em `core/models/` (Apache-2.0): scrfd_2.5g_bnkps.onnx, nima_mobilenet_aesthetic.onnx
  - Validado: NIMA+SCRFD inferem na GPU; culling ML ~5.6s/30 fotos
  - `core/src/culling.rs` — variância do Laplacian (nitidez), score de exposição,
    spread de histograma, score composto 0-100, paralelo via rayon.
  - `core/src/xmp.rs` — sidecar XMP compatível Lightroom/Capture One.
  - NAPI: `cullPhotos()` (quantis → rating 1-5) + `writeXmpForPhoto()`.
  - **Validado E2E na UI**: botão Cull → 30 fotos em ~1.3s, ratings/scores no grid.
  - `core/src/imageproc.rs` — `inspect_file` (dimensões, câmera, EXIF via
    kamadak-exif, hash SHA-256), extração de preview embutido via tags
    JPEGInterchangeFormat, geração de thumbnail JPEG base64.
  - `core/src/lib.rs` — Funções NAPI: `setup`, `scan_folder`, `list_photos`,
    `get_photo`, `photo_count`, `thumb_for_photo` (async), `thumb_for_path`.
  - **UI**: `Gallery.tsx` com grid virtualizado (react-window 2.x `Grid` +
    `cellComponent`/`cellProps`), `App.tsx` com import de pasta via dialog nativo,
    `preload` expondo `scanFolder`/`listPhotos`/`thumbForPhoto`/`pickFolder`.
  - **Validado E2E**: app abre, importa 30 fotos, grid virtualizado exibe todas
    com thumbnails carregadas.

### Failures / pontos de atenção
- **RAW preview** (CR3/NEF/ARW/DNG):
  - NEF/ARW/DNG/CR2 (TIFF-based): **funciona** via `read_embedded_jpeg` usando os
    tags `JPEGInterchangeFormat` (0x201) / `JPEGInterchangeFormatLength` (0x202)
    do kamadak, iterando TODOS os IFDs.
  - **CR3 (Canon, HEIF container)**: NÃO funciona com kamadak. Requer parser de
    container BMFF/HEIF (boxes PRVW/THMB) — futuro.
  - **`jpgfromraw-lib` (MIT) FALHA de build**: build.rs obrigatório exige nasm/CMake
    (gpr_tools + dcraw), panica sem ferramentas C → **descartado**.
- `setup()` usa `OnceLock` por processo — cada processo Node novo precisa chamar
  `setup()`.
- **userData do preview**: electron-vite preview usa `app.getPath('userData')` =
  `~/Library/Application Support/openshoot` (nome do package.json). Para popular o
  catálogo fora do dialog, rodar `setup` apontando para esse diretório.
- Dimensões (width/height) podem vir 0 p/ PNG sem EXIF (kamadak lê TIFF/JPEG/HEIF).
- react-window é a **v2.x** (API nova: `Grid` + `cellComponent`/`cellProps`,
  sem `width`/`height` como props, usa `defaultWidth`/`defaultHeight` + `onResize`).

## Decisões técnicas tomadas
- Stack: Electron/React ⇄ IPC ⇄ Rust core via **napi-rs**.
- kamadak-exif (BSD-2-Clause) para EXIF. Preview RAW via tags JPEGInterchangeFormat.
- crate `image` 0.25 para thumbnails. react-window 2.x para virtualização.
- `jpgfromraw-lib` (MIT) rejeitado (build script exige ferramentas C).
- **Culling heurístico**: Laplacian variance + exposição + histograma, rayon paralelo.
- **XMP**: template Lightroom-compatível (xpacket UUID `W5M0MpCehiHzreSzNTczkc9d`,
  xmp:Rating 0-5, xmp:Label Red/Yellow/Green/Blue/Purple, dc:subject Bag). Testado
  contra referência do repo `pixcull` (testado em LR Classic 13.x e C1 23).

## Pesquisas concluídas (delegações) — PRÓXIMO PASSO ONNX
- **`ort` (ONNX Runtime) para IA** (ver `~/.local` ou tmp `ort-metal-research.md`):
  - Não existe feature "metal" no ort — o EP Apple é **CoreML** (`feature: coreml`).
    `ort 2.0.0-rc.13` envolve ONNX Runtime 1.28, MSRV 1.88 (OK Rust 1.97).
  - `download-binaries` baixa binário pré-compilado (~8.8MB) no build.
  - Config: `ep::CoreML::default().with_compute_units(ALL).with_mlprogram(true)
    .with_static_input_shapes(true).with_cache_path(<cache>)` + `.error_on_failure()`.
  - Fallback automático p/ CPU se CoreML não registrar. Inicializar `ort::init()` uma vez.
  - **Modelos**: SCRFD-500M bnkps (`det_500m.onnx` no buffalo_s.zip 127MB; ou HF
    RuteNL/SCRFD-face-detection-ONNX `2.5g_bnkps.onnx` 3.3MB, Apache-2.0). NIMA:
    HF `cromsc/nima-mobilenet-aesthetic` `nima_mobilenet_aesthetic.onnx` 12.9MB
    (licença não declarada — usar com cautela).
  - ⚠️ Licença InsightFace: README diz "non-commercial research only" (código MIT,
    modelos restritos) — verificar antes de uso comercial.
  - SCRFD input 640x640, NIMA 224x224. `Session` é Send+Sync (rodar em spawn_blocking).
- **XMP** (ver tmp `xmp-research.md`): confirmou xmp:Rating, xmp:Label; Capture One
  usa sidecar .xmp sync ou `.cos` em CaptureOne/Settings*; naming `<stem>.xmp`.

## Próximos passos (ordem sugerida)
1. **Refinar SCRFD decode multi-escala**: o SCRFD emite score_8/16/32 + bbox_8/16/32 (multi-escala). A decodificação atual é simplificada (assume N detecções). Para produção, implementar NMS (non-max suppression) por escala (ver InsightFace pynms).
2. **Filtro/UI picks + export XMP em massa**.
3. Melhorias Fase 1: dimensões via cabeçalho; CR3 via parser BMFF.

## NOTA de integração ONNX (feito)
- `ort = { version="=2.0.0-rc.13", features=["coreml"] }` + `ndarray = 0.17` (ALINHAR versão com a do ort — havia 0.16 vs 0.17 duplicadas; resolver usando 0.17).
- **NIMA = NHWC** `(1,224,224,3)`; **SCRFD = NCHW** `(1,3,640,640)`. Erro 'invalid dimensions' = layout errado.
- `Session` não clona nem é &mut através de OnceLock → usar `Mutex<Session>`.
- `ComputeUnits::All` (não ALL). `session.inputs()/outputs()` são métodos.
- `Tensor::from_array(Array4)` (feature ndarray). `try_extract_tensor` retorna `(&Shape, &[T])`.
- **Adicionado**: `ort` 2.0.0-rc.13, `ndarray` 0.17, modelos em core/models/. adicionar `ort = { version="=2.0.0-rc.13", features=["coreml"] }`
   + `ndarray`. Criar módulo `ml.rs` com SCRFD (faces) + NIMA (qualidade). Integrar ao
   score do culling (com fallback ao heurístico se modelo ausente). Baixar modelos p/
   `core/models/` (não commitar binários grandes; baixar no primeiro uso).
2. **Filtro de XMP**: UI para ver/excluir picks; exportar XMP em massa.
3. Melhorias Fase 1: dimensões via cabeçalho p/ imagens sem EXIF; CR3 via parser BMFF.

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
- Catalogo de teste: `/tmp/ostest/shoot` (30 PNGs gerados; `.jpg` fake ignorados).
- Dependencias novas: react-window 2.x, @types/react-window.

## Como retomar (recuperação de crash)
1. `cd ~/OpenShoot`
2. `npm install` (aprovar scripts electron/esbuild/fsevents se pedir)
3. `npm run build:core` — regenera `core/openshoot_core.<plat>.<arch>.node`
4. `npm run dev` — sobe o app
5. `npm run typecheck` e `npm test` — validação

## Notas de ambiente
- Máquina: macOS (darwin/arm64). Rust 1.97.1, Node 24.18.
- addon gerado: `core/openshoot_core.darwin.arm64.node` (3.8MB, gitignored).
- Diretório de dados de teste: `/tmp/ostest`.
