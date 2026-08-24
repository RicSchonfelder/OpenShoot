# Auditoria e Melhorias Multi-Plataforma (Windows/Linux)

Data: 2026-08-24 · Origem: port do OpenShoot de macOS para Windows
Método: auditoria automatizada por agente (varredura de `src/`, `core/`, `scripts/`,
`electron-builder.yml`, `.github/`) seguida de fixes, build e testes no Windows 11
(Node 22 + Rust stable + MSVC).

## Resumo

O projeto nasceu macOS-only em vários pontos. Esta auditoria identificou 12 achados
(4 críticos) e todos os críticos + highs foram corrigidos. O core agora compila,
passa nos 60 testes (`cargo test`) e o addon nativo carrega no Windows
(`npm run smoke:core`), com culling via ONNX/DirectML funcional.

## Achados e correções

### Críticos

1. **`models_dir()` com caminho compilado** — `core/src/ml.rs` usava
   `env!("CARGO_MANIFEST_DIR")`, gravando o caminho absoluto da máquina de build
   no binário. Em qualquer outra máquina (e sempre em installs empacotados) os
   modelos ONNX "não existiam" e toda a IA morria silenciosamente.
   **Fix:** resolução em runtime — env `OPENSHOOT_MODELS_DIR` → diretório dev →
   busca por `core/models` subindo a partir do executável.
   `electron-builder.yml` ganhou `asarUnpack: core/models/**` (Rust não lê dentro
   de asar).

2. **Lixeira macOS-only** — `move_to_trash` (`core/src/lib.rs`) escrevia em
   `~/.Trash` e usava `fs::rename` (falha entre volumes no Windows; arquivos de
   `D:\` ou SD card não podiam ser apagados).
   **Fix:** crate `trash` — usa Recycle Bin (Windows), Finder/Native (macOS) e
   freedesktop (Linux). Testado no Windows: arquivo sai da pasta e vai p/ Recycle Bin.

3. **ONNX Runtime com feature `coreml` incondicional** — quebrava o build/link em
   plataformas Apple-less e deixava inferência 100% CPU fora do macOS.
   **Fix:** dependências por alvo no `Cargo.toml` (`coreml` só no macOS,
   `directml` no Windows) e registro de EP correspondente em `build_session()`
   (`#[cfg(target_os)]`). Culling ML verificado rodando no Windows com DirectML.

4. **Cache de thumbnails em `~/Library/Caches`** em todas as plataformas.
   **Fix:** `dirs::cache_dir()` — resolve para `%LOCALAPPDATA%` (Windows),
   `~/Library/Caches` (macOS), `$XDG_CACHE_HOME` (Linux). Mesmo fix aplicado ao
   cache CoreML.

### Highs

5. **`electron-builder.yml` sem `win:` nem `linux:`** — adicionado NSIS (x64) e
   AppImage. O glob `core/**/*` (que empacotaria `target/` inteiro) foi trocado por
   `core/*.node` + `core/models/**`.

6. **CI sem Windows** — job Rust convertido em matrix
   `macos/ubuntu/windows-latest`; clippy movido p/ ubuntu.

7. **Fallback de dados escrevendo na pasta de instalação** —
   `src/main/index.ts` fazia fallback para `join(app.getAppPath(), '.data')`
   (bloqueado pelo ACL de Program Files). Agora cai em `~/.openshoot-data`.

8. **`dist:win` ausente** — script adicionado ao `package.json`.

### Média

9. **`build-core.mjs` frágil sem rustc** — ENOENT cru ou crash em `.trim()` de
   undefined. Agora valida rustc com mensagem clara ("instale via rustup").

10. **Smoke test multi-plataforma** — novo `scripts/smoke-core.mjs`
    (`npm run smoke:core`): carrega o `.node` correto por plataforma, cataloga uma
    foto fixture (`core/fixtures/test.jpg`) e valida lixeira nativa.

## Verificado OK (sem mudança)

- `src/main/core.ts`: lookup de lib por `process.platform/arch` já correto.
- Renderer: atalhos já usam `metaKey || ctrlKey`; sem parsing manual de paths.
- Galeria web: URLs relativas — neutras.
- `window-all-closed` trata `platform !== 'darwin'`.
- Extensões lowercased no scan — case-safe.

## Como validar nesta máquina

```bash
npm install
npm run build:core     # cargo release + copia .node por plataforma
npm run typecheck      # tsc node + web
npm run smoke:core     # addon carrega, cataloga, lixeira
cd core && cargo test  # 60 testes
npm run dist:win       # instalador NSIS (opcional)
```

## Pendências conhecidas

- Prefix-match de paths em `catalog.rs` (`LIKE ?1 || '%'`) ainda é sensível a
  separadores mistos (`C:/Fotos` vs `C:\Fotos`) e `_` como wildcard SQL —
  funciona no fluxo normal, mas merece normalização central futura.
- Fixtures de teste Rust ainda usam strings `/tmp/...` (só DB, não tocam FS;
  não mascaram falhas hoje porque produção usa `temp_dir()`).
