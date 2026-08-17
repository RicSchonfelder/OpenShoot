# OpenShoot — Progresso de Desenvolvimento

> Arquivo de continuidade: registra o estado atual, o que funciona, o que falta e
> possíveis pontos de retomada caso o ambiente/agente reinicie.

**Última atualização:** 2026-08-17

## Estado atual

### Concluído
- **Fase 0** ✅ — Esqueleto Electron (React/TS) + core Rust (napi-rs) com ponte IPC
  validada E2E. Repo público no GitHub.
- **Fase 1** ✅ — Catálogo + decode + thumbnails + grid virtualizado:
  - `core/src/catalog.rs` — Catálogo SQLite (`photos`), schema, upsert, listagem
    com paginação/busca, `scan_folder` recursivo com `walkdir`.
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

## Próximos passos (ordem sugerida)
1. **Culling (Fase 2)** — detectar faces, nitidez, score, XMP. Este é o próximo
   grande marco (ver issue #2).
2. Melhorias Fase 1: dimensões via cabeçalho p/ imagens sem EXIF; CR3 via parser BMFF.
3. Commit Fase 1 concluído. Próximo commit: Fase 2 ou CR3.

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
