# Portabilidade multiplataforma — estado atual

> **Criada em:** 2026-08-25 (sessão de higiene documental).
> **Motivo:** vários documentos referenciavam `docs/MULTIPLATAFORMA.md`, mas o arquivo
> original do agente Windows (citado como commit `9776371`) **não está presente no
> histórico local** — o repositório foi re-baselinado num único commit
> (`2752c48 chore: establish local audit baseline`). Esta nota reconcilia as
> referências pendentes usando **somente evidências já existentes no repositório**
> e marca explicitamente o que **não foi executado/revalidado**.
> **Método:** inspeção estática de `core/Cargo.toml`, `electron-builder.yml`,
> `package.json`, `.github/workflows/ci.yml`, `core/src/ml.rs`, `core/src/lib.rs`
> e `src/main/index.ts`, cruzados com os docs de sessões anteriores.
> Nenhum código foi alterado nesta sessão.

---

## 1. O que o código/build atual garante (evidência verificada no repo)

| Item | Evidência |
|---|---|
| Execution Provider por plataforma | `core/Cargo.toml`: `ort` + `coreml` só em `[target.'cfg(target_os = "macos")']`; `ort` + `directml` em `[target.'cfg(target_os = "windows")']`; demais targets usam CPU. Feature opt-in `ort-load-dynamic` para dylib externa (`ORT_DYLIB_PATH`). |
| Lixeira nativa nas 3 plataformas | `core/Cargo.toml`: `trash = "5"`; `core/src/lib.rs:1124` (`move_to_trash` → `trash::delete`) usado pela deleção de fotos e XMPs órfãos. |
| Resolução de modelos em runtime | `core/src/ml.rs:11` lê `OPENSHOOT_MODELS_DIR`; `src/main/index.ts:47` define a env var no main process antes do `loadCore()`. Fallbacks: diretório dev e busca por ancestrais do executável. |
| Empacotamento multiplataforma | `electron-builder.yml`: mac → dmg+zip (arm64+x64), win → NSIS (x64), linux → AppImage+deb (x64, depends `libgtk-3-0/libnss3/libasound2t64`); `asarUnpack: core/**/*.node` + `core/models/**`. |
| Scripts de build/dist/smoke | `package.json`: `dist:mac`, `dist:linux`, `dist:win`, `smoke:core`. |
| CI com matriz | `.github/workflows/ci.yml`: `cargo test` em macos+ubuntu+windows (+ deps Linux `libgomp1 pkg-config libssl-dev`); typecheck Node 20; clippy ubuntu com `continue-on-error: true` (non-blocking). |

## 2. Status de validação por plataforma

### Linux — ✅ validado (sessões anteriores; ver `docs/AUDITORIA-LINUX.md`)
- 60/60 testes Rust em container Ubuntu 24.04; typecheck limpo;
  `npm run smoke:core` OK; E2E headless (xvfb + CDP) OK.
- `dist:linux` gerou AppImage (~141 MB) e .deb (~114 MB); .deb instalado em
  Ubuntu 24.04 limpo e app empacotado rodou sob xvfb
  (`docs/AUDITORIA-LINUX.md §6`).
- Limitações conhecidas: ONNX Runtime pré-compilado exige glibc ≥ 2.38 e AVX2;
  alternativa validada via feature `ort-load-dynamic`
  (`docs/AUDITORIA-LINUX.md §4`).

### macOS — ✅ sem regressão reportada (sessões anteriores)
- `docs/SESSAO.md §12` registra 60/60 testes Rust no host macOS pós-integração.
- ⚠️ **Não revalidado nesta sessão** (nenhuma execução macOS aqui).

### Windows — 🟡 evidência apenas documental
- Relatos das sessões anteriores (`docs/ROADMAP.md` P1, `docs/SESSAO.md §12`):
  DirectML testado e smoke do addon no Windows pelo agente Windows.
- ⚠️ **Nenhuma artefato/log de Windows existe neste repositório** (histórico
  re-baselinado) e **nada foi executado em Windows nesta sessão**. Tratar como
  "relatado, não verificável localmente".

## 3. O que ainda NÃO foi executado (pendente — não marcar como feito)

- [ ] Build/teste do core para `aarch64-unknown-linux-gnu` (ARM Linux) —
      último item aberto da P1 em `docs/ROADMAP.md`.
- [ ] CUDA/TensorRT opt-in no Linux — `docs/AUDITORIA-LINUX.md §2.2/§5`.
- [ ] Release workflow (tag → build nas 3 plataformas → anexar instaladores) —
      P5 `docs/ROADMAP.md`.
- [ ] Assinatura/notarização macOS e auto-update (electron-updater) — P5.
- [ ] Investigar falha intermitente de `cargo test` no runner
      `windows-latest` do CI — relato pré-existente em
      `docs/AUDITORIA-LINUX.md §5` (suspeita: flakiness SQLite/OnceLock, P6).
- [ ] Reexecução das validações de Windows (DirectML + NSIS + smoke) em
      máquina/runner Windows com logs arquivados no repo.

## 4. Como validar localmente

```bash
npm install           # aprovar scripts electron/esbuild/fsevents se pedir
npm run build:core    # compila core/openshoot_core.<platform>.<arch>.node
npm test              # cargo test no core
npm run typecheck     # TS main/preload + renderer
npm run smoke:core    # carrega o addon sem Electron

# Empacotamento por plataforma:
npm run dist:linux    # AppImage + .deb (Linux)
npm run dist:mac      # .app/.dmg (macOS)
npm run dist:win      # NSIS (Windows)

# CPUs/glibc antigos (opt-in): compilar/apontar dylib externa do ORT
# ver receita completa em docs/AUDITORIA-LINUX.md §4
```

---

## 5. Fontes no repositório

- Código: `core/Cargo.toml`, `core/src/ml.rs`, `core/src/lib.rs`,
  `src/main/index.ts`, `electron-builder.yml`, `package.json`,
  `.github/workflows/ci.yml`.
- Docs de sessão anteriores: `docs/AUDITORIA-LINUX.md`, `docs/ROADMAP.md`,
  `docs/SESSAO.md` (§11–12), `docs/PROGRESSO.md`, `AGENTS.md`.
