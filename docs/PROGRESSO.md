# OpenShoot — Progresso de Desenvolvimento

> Arquivo de continuidade: registra o estado atual, o que funciona, o que falta e
> possíveis pontos de retomada caso o ambiente/agente reinicie.

**Última atualização:** 2026-08-17

## Estado atual

### Concluído
- **Fase 0** ✅ — Esqueleto Electron (React/TS) + core Rust (napi-rs) com ponte IPC
  validada E2E. Repo público no GitHub.
- **Fase 1 (em andamento)** — Catálogo + decode + thumbnails:
  - `core/src/catalog.rs` — Catálogo SQLite (`photos`), schema, upsert, listagem
    com paginação/busca, `scan_folder` recursivo com `walkdir`.
  - `core/src/imageproc.rs` — `inspect_file` (dimensões, câmera, EXIF via
    kamadak-exif, hash SHA-256), extração de preview embutido via tags
    JPEGInterchangeFormat, geração de thumbnail JPEG base64.
  - `core/src/types.rs` — Tipos NAPI (`PhotoMeta`, `ScanResult`, `PhotoList`).
  - `core/src/lib.rs` — Funções NAPI: `setup`, `scan_folder`, `list_photos`,
    `get_photo`, `photo_count`, `thumb_for_photo` (async), `thumb_for_path`
    (async).
  - **Validado em runtime**: scan de pasta + catálogo + thumbnail de PNG/JPG
    funcionando via addon `.node` no Node.

### Failures / pontos de atenção
- **RAW preview** (CR3/NEF/ARW/DNG):
  - NEF/ARW/DNG/CR2 (TIFF-based): **funciona** via `read_embedded_jpeg` usando os
    tags `JPEGInterchangeFormat` (0x201) / `JPEGInterchangeFormatLength` (0x202)
    do kamadak, iterando TODOS os IFDs (o preview full-size costuma estar num
    SubIFD/thumbnail).
  - **CR3 (Canon, HEIF container)**: NÃO funciona com kamadak (ele só extrai EXIF,
    não o preview). Requer parser de container BMFF/HEIF (boxes PRVW/THMB).
  - **`jpgfromraw-lib` (MIT) FALHA de build**: o `build.rs` obrigatório tenta
    compilar `gpr_tools` + `dcraw_tool` (C/CMake/nasm) com `.expect`, o que panica
    sem nasm/CMake. Tentamos `default-features = false`, mas o build.rs não é
    controlado por features → **descartado**. Se um dia quisermos CR3, usar parser
    BMFF próprio leve (box `PRVW`/`THMB`).
- `setup()` usa `OnceLock` por processo — cada processo Node novo precisa chamar
  `setup()` antes de usar o catálogo.
- Dimensões (width/height) podem vir como 0 para PNG sem EXIF (kamadak lê
  primariamente TIFF/JPEG/HEIF). Melhoria futura: ler cabeçalho de imagem.

## Decisões técnicas tomadas
- Stack: Electron/React (UI) ⇄ IPC ⇄ Rust core via **napi-rs** (mesmo do Aftershoot).
- Decode de metadados: **kamadak-exif** (BSD-2-Clause). Preview de RAW embutido: via
  tags `JPEGInterchangeFormat` (0x201) e `JPEGInterchangeFormatLength` (0x202),
  iterando todos os IFDs. Confirmado no fonte do kamadak 0.6.1.
- Thumbnails: crate `image` 0.25 (thumbnail + JPEG encode base64). Funções async
  com `#[napi] async` + `tokio::task::spawn_blocking` para decode pesado.
- **`jpgfromraw-lib` (MIT) rejeitado** — build.rs obrigatório exige ferramentas C
  (nasm/CMake). Abordagem kamadak + parser próprio leve é preferível.

## Próximos passos (ordem sugerida)
1. Template de thumbnail/único caminho: manter `thumbForPhoto` (id) e
   `thumbForPath` — validar dimensões via cabeçalho p/ PNG.
2. **IPC/preload/renderer** — expor `setup`, `scanFolder`, `listPhotos`,
   `thumbForPhoto` no Electron e criar grid virtualizado na UI (react-window).
3. **UI Grid** — decisão de lib de virtualização (react-window recomendado).
4. Commit da Fase 1 (UI) + validação final + push.

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
