# OpenShoot — Roadmap de Ações Pendentes

> **Atualizado:** 2026-09-04 · Fonte única de tarefas pendentes (consolida PARIDADE-FUNCIONAL.md e sessões anteriores).
> **Estado geral:** paridade funcional com a referência ~completa. **P1 concluído** — app compila e roda em Windows (60 testes Rust, smoke test OK, culling IA via DirectML validado). Detalhes: docs/MULTIPLATAFORMA.md.
> **Ambiente principal de desenvolvimento:** Windows 11 (`D:\Programas\OpenShoot`). O macOS foi desativado da rede.

**Prioridades:** 🔴 alta · 🟡 média · 🟢 baixa

---

## ✅ P1 — Portabilidade multiplataforma (Windows / Linux) — CONCLUÍDO 2026-08-25

> **CONCLUÍDA (2026-08-24)** — dois agentes em paralelo: **Linux**
> (`docs/AUDITORIA-LINUX.md`, validado em Ubuntu 24.04 + regressão macOS) e
> **Windows** (`docs/MULTIPLATAFORMA.md`, validado no Windows 11 com DirectML).
> Integração dos dois trabalhos neste commit.

- [x] **EP de IA por plataforma** (`core/Cargo.toml` + `ml.rs`): `coreml` só no macOS, **DirectML no Windows** (testado), CPU no Linux. *(pendente: CUDA opt-in no Linux)*
- [x] **Caminhos de cache** (`dirs::cache_dir()`): thumbs + cache CoreML multiplataforma.
- [x] **Lixeira nativa**: crate `trash` (Recycle Bin / Finder / freedesktop) — corrige também `fs::rename` entre volumes.
- [x] **models_dir() em runtime**: `OPENSHOOT_MODELS_DIR` (main process) → dev → busca por ancestrais do exe; `asarUnpack: core/models/**`.
- [x] **electron-builder.yml**: NSIS (win) + AppImage/**deb** (linux) + scripts `dist:linux`/`dist:win`/`smoke:core`.
- [x] **CI com matriz**: `cargo test` em macos+ubuntu+windows (+ deps Linux); typecheck; clippy ubuntu. *(upload de instaladores permanece no P5)*
- [ ] Testar build do core nos 3 targets napi: `x86_64-pc-windows-msvc` ✓ (agente Windows) · `x86_64-unknown-linux-gnu` ✓ (agente Linux) · `aarch64-unknown-linux-gnu` ⬜ pendente.

## 🔴 P1.5 — Performance (gaps medidos com 459 fotos reais)

Benchmark completo e método de reprodução: `docs/TESTES/METRICAS-BASELINE.md`.

- [ ] **G1 — Import**: 15,6 s/foto com arquivos em iCloud Drive (~2 h para 459 fotos).
  Paralelizar scan (rayon); detectar placeholders `.icloud` e avisar/baixar antes;
  SHA-256 lazy (após import, em background).
- [x] **G2 — Face grouping**: ✅ **Feito (2026-08-25, Windows)**: `group_by_similarity`
  paralelizado com rayon (decode+letterbox em todos os cores; inferência segue
  serializada pelo Mutex da sessão ONNX) + **cache persistente de embeddings** no
  catálogo (`photos.face_embedding` BLOB, migração automática; formato
  `[count u32][f32...]` — todos os rostos da foto). Execuções repetidas pulam
  SCRFD+embedding das fotos já processadas. Sintético 20 fotos: 1,52 s → 0,30 s.
  Ganho real depende de fotos COM rostos (o baseline de 59 s/foto era Mac+RAW).
- [x] **G3 — Culling**: decodificar 1× em 640px e reusar nos 3 estágios. ✅ **Feito
  (2026-08-25, Windows)**: `cull_photos` agora faz decode único (`ml::load_rgb` 640)
  compartilhado por SCRFD + NIMA + heurística (nova `heuristic_score_rgb` em culling.rs,
  que reaproveita `gray_luma_from`). Sintético 20 fotos: 120→99 ms/foto; no conjunto
  real (RAW/459, onde o decode domina) o ganho esperado é maior.
- [x] **G4 — Summary do cull**: alinhar `picks` ao rating dos quantis. ✅ **Feito
  (2026-08-25)**: sem `target_picks`, picks = fotos com ★4+ (mesma regra do filtro
  da UI), não mais limiar fixo score≥70. Validado: 8 picks em 20 fotos (top 40%).

## 🔴 P2 — Olhos fechados integrado ao culling

As funções existem (`core/src/ml.rs`: `detect_faces_with_kps`, `eyes_open_score`) e já estão integradas parcialmente ao fluxo:

- [x] `cull_photos` (lib.rs): durante o culling, calcular `eyes_score` das fotos com rosto e persistir (SCRFD bnkps, mesma abordagem de keypoints usada pelo UniFace).
- [x] Filtro "Olhos fechados" no painel de filtros (fotos sem análise não entram no resultado).
- [x] Badge/ícone de olho fechado no grid e no loupe.
- [x] i18n das novas chaves.
- [ ] Validação E2E com fotos reais e calibração do limiar (bloqueada neste host pelo addon ORT incompatível e pela ausência de fixture facial; evidência em `docs/TESTES/rodadas/2026-09-04-olhos-fechados.md`).

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
- [x] Release workflow criado em `.github/workflows/release.yml`: tag → validação → build 3 plataformas → anexar instaladores na GitHub Release.
- [ ] Executar uma release de teste com uma tag, confirmar os três artefatos e registrar o readback da GitHub Release.
- [x] Scripts `npm run dist:linux` e `npm run dist:win`; build Linux validado em auditoria.

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

## 🟢 P9 — CodeFormer local (opt-in) — CONCLUÍDO 2026-09-04

Restauração de rostos via CodeFormer executada por **ponte CLI local do
usuário** (subprocesso sem shell, sem rede, opt-in OFF por padrão), integrada à
bancada de restauração. Setup, contrato CLI, nomes de pesos, limitações e
licença NTU S-Lab do upstream: `docs/CODEFORMER.md`.

- [x] Serviço isolado no main (`src/main/codeformer.ts`): settings opt-in
  (`codeformer-settings.json`), status acionável (`disabled|ready|error`),
  runner com timeout/validação.
- [x] Pesos compatíveis com `OPENSHOOT_MODELS_DIR` (fallback
  `OPENSHOOT_CODEFORMER_WEIGHTS_DIR`); app nunca baixa pesos.
- [x] Saída exclusiva validada (exatamente 1 JPEG/PNG, magic bytes); job em
  diretório temporário removido ao final; originais jamais sobrescritos.
- [x] IPC/preload/tipos strict (sem `any`) + seção opt-in em RestorerView.
- [x] Testes determinísticos sem pesos/GPU/rede (`npm run test:codeformer`),
  com ponte CLI simulada por scripts Node.
- [ ] Fatia futura: inferência nativa ONNX do CodeFormer no core Rust (exige
  pesos reais para validação; mantida fora por licença/opt-in).

---

## Registro de decisões que NÃO mudam

- **100% local/offline** — pixels nunca saem da máquina (P7 é opt-in explícito).
- **Não-destrutivo** — originais intocados; edições em sidecar/receita.
- **Open source MIT** — sem pesos proprietários.
- ~~macOS manual-trash~~ **SUPERADA (2026-08-24)** — lixeira nativa via crate
  `trash` nas 3 plataformas (corrige rename cross-volume; validada no Windows;
  sem relatos de permissão Finder com a API usada pelo crate).
