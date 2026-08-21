# OpenShoot — Progresso de Desenvolvimento

> Arquivo de continuidade: registra o estado atual, o que funciona, o que falta e
> possíveis pontos de retomada caso o ambiente/agente reinicie.

**Última atualização:** 2026-08-21

## Estado atual

### Concluído
- **Fase 0** ✅ — Esqueleto Electron (React/TS) + core Rust (napi-rs) com ponte IPC
  validada E2E. Repo público no GitHub.
- **Fase 1** ✅ — Catálogo + decode + thumbnails + grid virtualizado:
  - `core/src/catalog.rs` — Catálogo SQLite (`photos`), schema, upsert, listagem
    com paginação/busca, `scan_folder` recursivo com `walkdir`.
- **Fase 2 (heurístico + ML local)** ✅ — Culling com IA local (ONNX):
  - **SCRFD multi-escala + NMS** implementado e validado; **NIMA** (estética) +
    **SCRFD** (faces), engine **ort 2.0.0-rc.13** EP **CoreML** + fallback CPU.
  - Score final: heurística (Laplacian) + NIMA + bônus por rostos; fallback p/
    heurística se modelo ausente. `cullPhotos()` → quantis → rating 1-5.
  - `core/src/xmp.rs` — sidecar XMP compatível Lightroom/Capture One.
- **Fase 3** ✅ — Edição em lote não-destrutiva (exposição, WB, contraste, saturação,
  sombras, realces, brilho) com preview e persistência por foto (`edit_json`).
- **Fase 4** ✅ — Retoque local: suavização de pele (YCbCr + blur seletivo) +
  remoção de distrações (inpainting por difusão, bbox central MVP).
- **Fase 6 parcial** ✅ — Captions locais offline (`captions.rs`) + empacotamento
  macOS (electron-builder) → `/Applications/OpenShoot.app` (arm64).
- **UX de culling** ✅ (alinhada à auditoria AfterShoot):
  - **i18n pt-BR + en** (`src/renderer/src/i18n/`): `useT()`, detecção por
    `navigator.language`, "Cull"→"Selecionar" (pt).
  - **Loupe integrado**: duplo clique / Enter abre; setas navegam; P/X/U/1-5 aplicam
    rating e avançam; Esc fecha.
  - **Flags coloridos no grid** (verde P / vermelho X) + **★1-5 clicável por foto**.
  - **Toolbar de culling**: contadores P/X/U/total + "Selecionar todas (⌘A)".
  - **Diálogo de deletar 3 opções**: remover só do catálogo / mover p/ Lixeira /
    cancelar (`removePhotoFromCatalog` / `deletePhoto` com `move_to_trash` manual).
  - **⌘A/Ctrl+A** seleciona todas.
  - **Filtros avançados** (dropdown "Outros"): **Duplicatas** (sha256 agrupado) e
    **Com rosto** (`has_face` populado no culling via SCRFD).
- **Core** ✅ — 33 testes Rust passando; typecheck limpo (main/preload/renderer).

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

## Auditoria AfterShoot (2026-08-18/20)
- Documento de referência: `docs/AUDITORIA-AFTERSHOOT.md` (mapa completo + gap
  analysis com 15 itens priorizados). Implementados até agora:
  ★1-5 por foto, filtros avançados (dropdown), detecção de duplicatas, loupe,
  flags, i18n, toolbar de culling.

## Próximos passos (ordem sugerida)
1. **Presets nomeados de edição** (salvar/carregar receita JSON + biblioteca de
   estilos) — gap #1 da auditoria.
2. **"Para revisão"** (bucket de fotos com score limítrofe) e separação
   "Destaques IA" vs "Selecionado manual".
3. **Wizard de importação** (tipo de fotos, incluir subpastas, tipo de sessão).
4. **Filtro por orientação / câmera / rating numérico** + "Editar status".
5. **Meta de nº de picks** no culling ("quantas fotos selecionar").
6. **Retoque facial** (sliders: acne, olhos, dentes) + seleção por arrasto no patch.
7. **Olhos fechados** (SCRFD landmarks) + flag de aviso.

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
