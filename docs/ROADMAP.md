# OpenShoot — Roadmap de Ações Pendentes

> **Atualizado:** 2026-08-24 · Fonte única de tarefas pendentes (consolida PARIDADE-FUNCIONAL.md e sessões anteriores).
> **Estado geral:** paridade funcional com a referência ~completa (álbuns, fluxo IMPORT→CULL→EDIT→RETOUCH, culling IA, edição completa, retoque, exportação, reconhecimento facial). 60 testes Rust, typecheck limpo, CI ativo.

**Prioridades:** 🔴 alta · 🟡 média · 🟢 baixa

---

## 🔴 P1 — Portabilidade multiplataforma (Windows / Linux)

> **CONCLUÍDA (2026-08-24)** — dois agentes em paralelo: **Linux**
> (`docs/AUDITORIA-LINUX.md`, validado em Ubuntu 24.04 + regressão macOS) e
> **Windows** (`docs/MULTIPLATAFORMA.md`, validado no Windows 11 com DirectML).
> Integração dos dois trabalhos neste commit.

- [x] **EP de IA por plataforma** (`core/Cargo.toml` + `ml.rs`): `coreml` só no macOS, **DirectML no Windows** (testado), CPU no Linux. *(pendente: CUDA opt-in no Linux)*
- [x] **Caminhos de cache** (`dirs::cache_dir()`): thumbs + cache CoreML multiplataforma.
- [x] **Lixeira nativa**: crate `trash` (Recycle Bin / Finder / freedesktop) — corrige também `fs::rename` entre volumes.
- [x] **models_dir() em runtime**: `OPENSHOOT_MODELS_DIR` (main process) → dev → busca por ancestrais do exe; `asarUnpack: core/models/**`.
- [x] **electron-builder.yml**: NSIS (win) + AppImage/**deb** (linux) + scripts `dist:linux`/`dist:win`/`smoke:core`.
- [x] **CI com matriz**: `cargo test` em macos+ubuntu+windows (+ deps Linux); typecheck; clippy ubuntu. *(pendente: upload de instaladores — P5)*
- [ ] Testar build do core nos 3 targets napi: `x86_64-pc-windows-msvc` ✓ (agente Windows) · `x86_64-unknown-linux-gnu` ✓ (agente Linux) · `aarch64-unknown-linux-gnu` ⬜ pendente.

## 🔴 P2 — Olhos fechados integrado ao culling

As funções existem (`core/src/ml.rs`: `detect_faces_with_kps`, `eyes_open_score`) mas **não estão no fluxo**:

- [ ] `cull_photos` (lib.rs): durante o culling, calcular `eyes_score` das fotos com rosto e persistir (coluna `eyes_score` já migrada pelo agent-02).
- [ ] Filtros "Com aviso / Sem aviso" no dropdown Outros (avisos: olhos fechados, foto tremida).
- [ ] Badge/ícone de olho fechado no grid e no loupe.
- [ ] i18n das novas chaves.

## 🟡 P3 — Filtros por cor de label

Labels (Red/Yellow/Green/Blue/Purple) já existem com menu de contexto no grid (agent-10), mas:

- [ ] Filtro por cor no dropdown "Outros" (precisa estender `list_photos` com filtro `label=?`).
- [ ] Mostrar a cor também no loupe e no EditPanel.
- [ ] Exportar label no XMP já funciona via rating; garantir label manual no sidecar.

## 🟡 P4 — Exportação: refinamentos

- [ ] **Aba "Outros Apps"**: handoff para Lightroom/Capture One/Photoshop (hoje só via XMP sidecars manuais) — abrir o app alvo com a pasta exportada.
- [ ] **Espaço de cor real**: Display P3 atual é aproximação (+5% saturação); implementar conversão ICC real (perfil embutido no JPEG exportado).
- [ ] Redimensionamento na exportação (long edge px) — a referência tem.
- [ ] Barra de progresso da exportação em lote (hoje é síncrono sem feedback por foto).

## 🟡 P5 — Empacotamento & distribuição

- [ ] **Assinatura + notarização macOS** (Apple Developer ID) para instalar sem avisos de Gatekeeper.
- [ ] **Auto-update** (electron-updater) com feed de releases do GitHub.
- [ ] Release workflow: tag → build 3 plataformas → anexar instaladores na GitHub Release.
- [ ] `npm run dist:win` / `dist:linux` scripts.

## 🟢 P6 — Qualidade & testes

- [ ] Testes E2E da UI (Playwright/WebDriver: importar → cull → editar → exportar).
- [ ] Testes de IPC (mock do preload).
- [ ] Corrigir flakiness residual: testes que compartilham o SQLite via OnceLock (isolar por teste ou rodar `--test-threads=1` no CI para o pacote de catálogo).
- [ ] Clippy limpo (`-D warnings`) — hoje allow-failure no CI.

## 🟢 P7 — Fase 6 original (opt-in, não afeta pixels)

- [ ] OpenRouter (chave do usuário no Keychain, OFF por padrão): legendas/descrições de álbum, sugestões de organização.
- [ ] Painel de configurações com toggle de privacidade explícito.

## 🟢 P8 — Polimento de UX (menores)

- [ ] Drag-and-drop de pastas na importação (hoje só diálogo).
- [ ] "Escolher entre as recentes" na importação (a referência tem).
- [ ] Comparação lado-a-lado de fotos no loupe.
- [ ] Tema claro (hoje só dark).
- [ ] Mais idiomas no i18n (estrutura pt-BR/en já pronta).

---

## Registro de decisões que NÃO mudam

- **100% local/offline** — pixels nunca saem da máquina (P7 é opt-in explícito).
- **Não-destrutivo** — originais intocados; edições em sidecar/receita.
- **Open source MIT** — sem pesos proprietários.
- ~~macOS manual-trash~~ **SUPERADA (2026-08-24)** — lixeira nativa via crate
  `trash` nas 3 plataformas (corrige rename cross-volume; validada no Windows;
  sem relatos de permissão Finder com a API usada pelo crate).
