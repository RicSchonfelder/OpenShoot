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
- **RAW preview** (CR3/NEF/ARW/DNG) ainda NÃO funciona — `extract_preview_bytes`
  retorna erro para esses formatos. Depende da pesquisa de crates (ver abaixo).
- `setup()` usa `OnceLock` por processo — cada processo Node novo precisa chamar
  `setup()` antes de usar o catálogo.

## Decisões técnicas tomadas
- Stack: Electron/React (UI) ⇄ IPC ⇄ Rust core via **napi-rs** (mesmo do Aftershoot).
- Decode de metadados: **kamadak-exif** (ÉXIF). Preview de RAW embutido: via tags
  `JPEGInterchangeFormat` (`Tag::JPEGInterchangeFormat`, 0x201) e
  `JPEGInterchangeFormatLength` (0x202) — confirmados no kamadak 0.5.5.
- Thumbnails: crate `image` 0.25 (thumbnail + JPEG encode base64).
- Async: funções NAPI com `#[napi] async` + `tokio::task::spawn_blocking` para
  decode pesado.

## Próximos passos (ordem sugerida)
1. **RAW preview** — integrar crate para CR3/HEIF e NEF/ARW/DNG (ver pesquisa em
   andamento: `delegate`). Registrar licença em THIRD_PARTY.md.
2. **IPC/preload/renderer** — expor as novas APIs `setup`, `scanFolder`,
   `listPhotos`, `thumbForPhoto` no Electron e criar grid virtualizado na UI.
3. **UI Grid** — virtualização (react-window ou similar), decisão necessária.
4. Commit da Fase 1 + validação final.

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
