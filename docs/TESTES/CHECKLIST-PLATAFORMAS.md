# Checklist Multiplataforma — Windows / Linux

> **Para:** agentes validando a portabilidade (ver ROADMAP P1).
> **Estado atual:** app só compila/roda em macOS. Este checklist valida cada ponto
> após a portabilidade. Executar em ordem.

## Pré-requisitos por plataforma

### Windows
- [ ] Rust (msvc toolchain) + Node 20+ + Python 3
- [ ] VS Build Tools (Desktop C++ workload)
- [ ] `npm install` conclui (scripts electron/esbuild aprovados)

### Linux (Ubuntu 22.04+)
- [ ] `build-essential`, `libwebkit2gtk-4.0-dev`, `libssl-dev`, `rustc`, `nodejs 20+`
- [ ] `npm install` conclui

## PORT-01 Compilação do core
- [ ] `npm run build:core` gera `core/openshoot_core.<plat>.<arch>.node`
      (win: `win32_x64_msvc`; linux: `linux_x64_gnulibc`)
- [ ] `cargo test --manifest-path core/Cargo.toml` — todos passam
- [ ] Sem warnings novos de plataforma (cfg)

## PORT-02 EP de IA por plataforma
- [ ] macOS: logs mostram CoreML ativo (cache em ~/Library/Caches/OpenShoot/coreml)
- [ ] Windows: DirectML ativo (ou CPU com aviso claro no log)
- [ ] Linux: CPU ativa (ou CUDA se configurado)
- [ ] Culling roda nos 3 (medir tempo em 30 fotos e registrar vs baseline macOS 5.6s)

## PORT-03 Caminhos
- [ ] Cache de thumbs em `dirs::cache_dir()/OpenShoot/thumbs`
      (Win: %LOCALAPPDATA% · Linux: ~/.cache) — criar/listar/apagar funcionam
- [ ] Catálogo em userData correto por SO
- [ ] "Limpar cache" funciona e reporta N arquivos

## PORT-04 Lixeira
- [ ] Windows: deletePhoto envia para Recycle Bin (restaurável via Explorer)
- [ ] Linux: envia para XDG trash (~/.local/share/Trash)
- [ ] macOS: mantém implementação manual ~/.Trash (sem permissão Finder)

## PORT-05 UI/Empacotamento
- [ ] `npm run dev` abre o app na plataforma
- [ ] `npm run dist` gera instalador (win: NSIS .exe; linux: AppImage/.deb)
- [ ] App empacotado encontra os modelos ONNX em core/models/ (path no asar unpacked)
- [ ] Fontes/acentos/emoji renderizam (i18n pt-BR com acentos)
- [ ] Atalhos de teclado usam Ctrl (não ⌘) no Win/Linux onde aplicável — registrar
      o que precisa trocar (⌘A → Ctrl+A etc.)

## PORT-06 Funcional ponta-a-ponta (rodar TESTPLAN-UI resumido)
- [ ] Import 30 fotos → grid OK
- [ ] Cull → ratings/flags OK
- [ ] Editar 1 foto + aplicar lote OK
- [ ] Export 5 JPEG q90 → arquivos válidos (verificar com `file`/IrfanView)
- [ ] Loupe abre, navega, orientação EXIF correta

## PORT-07 CI
- [ ] Workflow com matriz (macos-latest, windows-latest, ubuntu-latest) verde
- [ ] Artifacts de instaladores anexados no run

## Registro
```
Plataforma: ____ · Data: ____ · Executor: ____
PORT-01..07: PASS/FAIL por item
Tempos: import30 ___s · cull30 ___s · export5 ___s
Problemas: ___
```
