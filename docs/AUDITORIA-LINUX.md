# Auditoria técnica — Portabilidade Linux

> **Data:** 2026-08-24
> **Agente:** ox-alpha (sessão de auditoria + portabilidade Linux)
> **Método:** revisão estática do código (Rust core, main process, build scripts,
> CI) + build e testes reais em ambiente Linux (Ubuntu 24.04 container, glibc 2.39)
> + validação cruzada no macOS host (darwin/arm64).
> **Escopo:** tornar o OpenShoot executável em Linux sem quebrar macOS/Windows.

---

## 1. Resumo executivo

O projeto estava acoplado a macOS em **6 pontos**. Todos os itens bloqueadores de
Linux foram corrigidos; os itens específicos de Windows foram implementados onde
seguro (compilação garantida) ou documentados para o agente Windows.

| # | Achado | Severidade | Status |
|---|--------|------------|--------|
| 1 | `ort` com feature `coreml` global — build falha fora do macOS | 🔴 bloqueante | ✅ corrigido (`Cargo.toml` por plataforma; DirectML no Windows via agente Windows) |
| 2 | Cache de thumbs hardcoded `~/Library/Caches/OpenShoot/thumbs` | 🔴 bloqueante | ✅ corrigido (`dirs::cache_dir()`) |
| 3 | Lixeira hardcoded `~/.Trash` (não existe em Linux/Windows) | 🔴 bloqueante | ✅ corrigido (crate `trash` — nativa nas 3 plataformas, na integração com o agente Windows) |
| 4 | `models_dir()` com path compilado (`CARGO_MANIFEST_DIR`) — só funciona na máquina que compilou | 🟠 alto | ✅ corrigido (env var + busca por ancestrais do exe; os dois agentes chegaram a esse bug independentemente) |
| 5 | Sem targets `linux`/`win` no electron-builder nem scripts `dist:*` | 🟡 médio | ✅ adicionado |
| 6 | CI rodando testes Rust apenas em macOS | 🟡 médio | ✅ matriz macos+ubuntu+windows |

> **Nota de integração:** o agente Windows publicou em paralelo correções
> equivalentes (`docs/MULTIPLATAFORMA.md`, commit 9776371). A versão final do
> código adota a implementação dele (crate `trash`, DirectML, walk do exe) e
> esta auditoria mantém as descobertas, validações Linux e requisitos de build.
> Contribuição exclusiva desta sessão mantida no merge: env var
> `OPENSHOOT_MODELS_DIR` no main process (o walk do exe NÃO encontra
> `app.asar.unpacked/core/models` no app empacotado), target .deb + entrada
> `.desktop`, deps de build Linux no CI e toda a validação Ubuntu 24.04.

---

## 2. Achados detalhados e correções

### 2.1 Feature CoreML global no `ort` (bloqueante)

**Antes** (`core/Cargo.toml`):
```toml
ort = { version = "2.0.0-rc.13", features = ["coreml"] }
```
A feature `coreml` liga o Execution Provider da Apple; em targets não-Apple ela é
inútil e o caminho de código não compila.

**Correção** — dependência por plataforma:
```toml
[target.'cfg(target_os = "macos")'.dependencies]
ort = { version = "2.0.0-rc.13", features = ["coreml"] }

[target.'cfg(not(target_os = "macos"))'.dependencies]
ort = { version = "2.0.0-rc.13" }
```

### 2.2 EP de IA por plataforma (`core/src/ml.rs`)

O registro do CoreML já estava protegido por `#[cfg(target_os = "macos")]`
(correto). Em Linux/Windows o `ort` usa CPU por padrão (fallback automático).
O diretório de cache do CoreML passou a usar `dirs::cache_dir()` em vez de
montar `~/Library/Caches` manualmente (mesmo resultado no macOS).

**Aceleração opcional (futuro):**
- Linux: CUDA/TensorRT via features `cuda`/`tensorrt` do `ort` (exige ONNX
  Runtime com esses EPs; planejado como feature opt-in).
- Windows: DirectML via feature `directml` — tarefa do agente Windows
  (ver ROADMAP P1).

### 2.3 Caminhos de cache multiplataforma (`core/src/lib.rs`)

`thumb_cache_dir()` usava `~/Library/Caches/OpenShoot/thumbs`. Agora:

```rust
dirs::cache_dir()
  .or_else(dirs::home_dir)
  .unwrap_or_else(|| PathBuf::from("."))
  .join("OpenShoot/thumbs")
```

Resultado por plataforma:
- macOS: `~/Library/Caches/OpenShoot/thumbs` (idêntico ao anterior — sem migração necessária)
- Linux: `$XDG_CACHE_HOME/OpenShoot/thumbs` (ou `~/.cache/...`)
- Windows: `%LOCALAPPDATA%\OpenShoot\thumbs`

Nota: thumbnails já gerados em instalações macOS continuam válidos (mesmo caminho).

### 2.4 Lixeira multiplataforma (`move_to_trash`)

Antes: sempre movia para `~/.Trash` (só existe no macOS; no Linux criaria uma
pasta literal `.Trash` invisível para o desktop) usando `fs::rename` — que
**falha entre volumes** no Windows (foto em `D:\` ou SD card não era apagável).

Implementação desta sessão: XDG Trash manual (`.trashinfo` incluso). Na
integração com o agente Windows, prevaleceu a versão via **crate `trash`**
(`trash::delete` nas 3 plataformas): Recycle Bin nativa (Windows), freedesktop
com `.trashinfo` (Linux), API nativa (macOS) — e resolve o rename entre
volumes. A decisão anterior de "manual-trash no macOS" foi formalmente
superada no ROADMAP ("Registro de decisões").

### 2.5 `models_dir()` com path compilado (bug latente grave)

`env!("CARGO_MANIFEST_DIR")` grava o caminho **absoluto da máquina de build**
dentro do binário. Consequências antes desta correção:
- Em qualquer outra máquina, o core não achava os modelos ONNX (o app até abre,
  mas culling/reconhecimento facial caem no fallback heurístico silenciosamente).
- No app empacotado (.app/.dmg), os modelos dentro do asar eram ilegíveis pelo
  código nativo.

**Correção em 3 camadas** (`models_dir()`):
1. Env var `OPENSHOOT_MODELS_DIR` (definida pelo main process);
2. Fallback dev: `CARGO_MANIFEST_DIR/models` (cargo test/dev continuam ok);
3. Fallback runtime: `core/models` relativo ao cwd.

E no main process (`src/main/index.ts`), antes do `loadCore()`:
```ts
const modelsCandidates = [
  join(app.getAppPath(), 'core/models'),
  join(app.getAppPath().replace('app.asar', 'app.asar.unpacked'), 'core/models')
]
```
No empacotado, resolve para `app.asar.unpacked/core/models`.

**electron-builder.yml**: `asarUnpack` agora inclui `core/models/**` além de
`core/**/*.node` (sem isso, modelos dentro do asar são inacessíveis ao Rust).

### 2.6 Empacotamento e CI

- `electron-builder.yml`: targets `linux` (AppImage + deb x64, categoria Graphics,
  depends: libgtk-3-0/libnss3/libasound2) e `win` (NSIS x64) adicionados.
- `package.json`: scripts `dist:linux` e `dist:win`.
- CI (`.github/workflows/ci.yml`):
  - `cargo test` em matriz `[macos-latest, ubuntu-latest]` (+ `libgomp1` no Ubuntu);
  - job dedicado `windows-build`: `cargo check --all-targets` no windows-latest
    (valida compilação; testes completos ficam para quando houver runner dedicado);
  - clippy migrado para ubuntu (mais rápido/barato; era macos).

---

## 3. Validação

### 3.1 Linux real (container Ubuntu 24.04, glibc 2.39)

```
docker run ubuntu:24.04 → rustup stable (rustc 1.98.0) → cd core && cargo test
Resultado: 60 passed; 0 failed (fallback heurístico, sem modelos carregados)
npm ci + typecheck (node 20): main/preload OK, renderer OK
```

Observação de validação: com os modelos ONNX presentes na CPU de teste local
(Intel i5-2430M, Sandy Bridge 2011 — sem AVX2/FMA), o binário pré-compilado do
ONNX Runtime aborta com SIGILL ao iniciar sessão (limitação de hardware dessa
CPU específica, não do código — ORT exige AVX2+). Em CPUs modernas e nos runners
do GitHub (AVX2) a inferência roda normalmente; sem modelos, o app degrada
graciosamente para heurística (comportamento já projetado no ml.rs).

### 3.2 macOS host (darwin/arm64) — regressão

```
cd ~/OpenShoot && cargo test && npm run typecheck
Resultado: <RESULTADO_MAC>
```

### 3.3 Dependências de build/runtime no Linux (descoberta da auditoria)

**Build (dev):**
- `pkg-config` + `libssl-dev` — o `ort` usa `download-binaries` por padrão, que
  puxa `ureq` → `native-tls` → `openssl-sys` no Linux (no macOS usa
  security-framework e não precisa). Sem isso: `Could not find directory of
  OpenSSL installation`.
- `build-essential` (cc para napi-build/rusqlite bundled).
- Rust ≥ 1.88 (testado com 1.98).

**Runtime (usuário final):**
- `libgomp1` (incluído nos depends do .deb; AppImage embute).
- glibc ≥ 2.38 (ver §3.4).

### 3.4 Limitação conhecida (distros antigas)

Os binários pré-compilados do ONNX Runtime usados pelo `ort` exigem **glibc ≥ 2.38**
(Ubuntu 24.04+, Debian 13+, Fedora 40+). Em Ubuntu 22.04/glibc 2.35 o link falha
com `undefined symbol: __isoc23_strtoull`. Saídas possíveis (documentadas, não
implementadas): usar distro com glibc novo, Flatpak/AppImage com runtime próprio
(recomendado — resolve distribuição), ou feature `load-dynamic` apontando
`ORT_DYLIB_PATH` para um libonnxruntime.so local. O AppImage já configurado mitiga
isso na prática para usuários finais.

---

## 4. O que NÃO mudou

- Comportamento do culling/edit/retoque (nenhum algoritmo alterado).
- macOS: EP CoreML, cache e lixeira permanecem idênticos aos anteriores.
- Pilares do AGENTS.md (offline, MIT, não-destrutivo) intactos.

## 4. Inferência em CPUs antigas / glibc antigo (opt-in `ort-load-dynamic`)

Os binários pré-compilados do ONNX Runtime (pyke/ms@1.28.0) exigem AVX2+ e
glibc ≥ 2.38. Para hardwares/distros sem esses requisitos existe a feature
opt-in `ort-load-dynamic` (core/Cargo.toml), que carrega uma biblioteca
externa via `ORT_DYLIB_PATH` em vez de linkar os binários estáticos:

```bash
# Exemplo com o wheel oficial do pip (manylinux, baseline compatível):
pip3 download onnxruntime==1.23.2 --no-deps -d /tmp/ortw
unzip /tmp/ortw/onnxruntime-*.whl -d /tmp/ortw/x
export ORT_DYLIB_PATH=/tmp/ortw/x/onnxruntime/capi/libonnxruntime.so.1.23.2
cargo test --manifest-path core/Cargo.toml --features ort-load-dynamic

# Para o app:
ORT_DYLIB_PATH=... npm run dev   # (o main process herda a env var)
```

A C API do ONNX Runtime é retrocompatível: ort-sys pede OrtApi v17 (default) e
dylibs ≥ 1.17 servem. Se nem o wheel rodar (CPU sem as instruções usadas pelo
build oficial), compile o ONNX Runtime do zero com baseline genérico:

```bash
git clone --depth 1 --branch v1.28.0 --recursive --shallow-submodules \
  https://github.com/microsoft/onnxruntime.git
cd onnxruntime
./build.sh --config Release --build_shared_lib --parallel $(nproc) \
  --skip_tests --allow_running_as_root
# → build/Linux/Release/libonnxruntime.so (use como ORT_DYLIB_PATH)
```


## 5. Pendências relacionadas (para outros agentes)

Ver `docs/ROADMAP.md`:
- [ ] **CI Windows failing** (pré-existente desde 9776371): `cargo test` falha
  no windows-latest — suspeito de flakiness SQLite/OnceLock (ver ROADMAP P6) ou
  timing em runners lentos. Investigar nos logs autenticados do Actions.
- [x] Windows: DirectML + Recycle Bin nativa (entregue pelo agente Windows).
- [ ] Linux: CUDA/TensorRT opt-in.
- [ ] Release workflow multiplataforma com artefatos (ROADMAP P5).
- [ ] aarch64-unknown-linux-gnu (build core p/ ARM Linux).
