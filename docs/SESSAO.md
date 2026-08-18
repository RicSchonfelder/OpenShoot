# OpenShoot — Contexto de Sessão (para retomada após reinício)

> Este documento é o **ponto de retomada**. Leia primeiro antes de continuar
> qualquer trabalho. Complementa `DESIGN.md` (arquitetura) e `PROGRESSO.md`
> (histórico técnico/detalhes).

**Criado em:** 2026-08-18 (sessão de desenvolvimento da Fase 2)
**Estado:** todos os commits da sessão foram feitos e **pushed** ao GitHub.

---

## 1. O que ESTE documento resolve

A sessão anterior foi longa. Este doc captura: onde estamos, o que já está feito,
como recompor o ambiente, onde paramos e **o que fazer em seguida** — para que
você (ou um agente) continue sem perder contexto.

---

## 2. Repositório e endereço

- **Repo público (GitHub):** `https://github.com/RicSchonfelder/OpenShoot`
- **Local:** `~/OpenShoot` (caminho absoluto: `/Users/schon/OpenShoot`)
- **Remote:** `origin` → `git@github.com:RicSchonfelder/OpenShoot.git`
- **Branch:** `main`
- **Issues abertas:** 9 (fases 1–6 + good-first-issues). Ver em
  `https://github.com/RicSchonfelder/OpenShoot/issues`

---

## 3. O projeto em uma frase

App desktop **open-source (MIT)** para fotógrafos: culling/edição/retoque de fotos
com **IA local**. Inspirado no AfterShoot, mas 100% aberto.

**Arquitetura (3 camadas):**
```
UI (Electron/React) ⇄ [IPC via napi-rs] ⇄ core Rust (ONNX na GPU via CoreML)
```

**Pilares invioláveis:** 100% local/offline (pixels nunca saem da máquina) ·
open source (MIT) · chaves externas são do usuário (opt-in) · não-destrutivo.

---

## 4. O que já está CONCLUÍDO e validado

| Fase | Status | Destaque |
|---|---|---|
| Fase 0 | ✅ | Esqueleto Electron/React + Rust (napi-rs) + ponte IPC |
| Fase 1 | ✅ | Catálogo SQLite, decode RAW (NEF/ARW/DNG/CR2), thumbnails, grid virtualizado |
| Fase 2 (heurístico) | ✅ | Culling: Laplacian (nitidez) + exposição + histograma; ratings ★1-5 |
| Fase 2 (ML local) | ✅ | ONNX via `ort` + CoreML: **NIMA** (estética) + **SCRFD** (faces) |
| Fase 2 (XMP) | ✅ | Sidecar XMP compatível Lightroom/Capture One (rating/label/keywords) |

**Validado E2E na máquina:** app abre, importa pasta, grid virtualizado com
thumbnails, botão "Cull" roda IA local (NIMA+SCRFD) em ~5.6s/30 fotos.

**Qualidade:** 10 testes Rust passando ✓ · typecheck limpo ✓.

---

## 5. Como recompor o ambiente após RESTART

Pré-requisitos já instalados nesta máquina: Rust 1.97.1, Node 24.18, `gh` logado
(RicSchonfelder), `git` configurado.

```bash
cd ~/OpenShoot

# 1. Instalar dependências npm (APROVAR scripts do Electron se pedir):
npm install

# 2. Compilar o core Rust -> core/openshoot_core.<plat>.<arch>.node
#    (o 1º build do ort baixa o binário ONNX ~9MB; pode demorar alguns min)
npm run build:core

# 3. Rodar o app
npm run dev

# Validações
npm run typecheck    # TS (main/preload + renderer)
npm test             # testes Rust (cargo)
```

> **Importante:** o `npm install` pode pedir aprovação de scripts de
> install (`electron`, `esbuild`, `fsevents`) — aprovar todos.

---

## 6. ONNX/Modelos — o que precisa estar presente

Os modelos ONNX estão **commitados** em `core/models/` (funcionam out-of-box):
- `nima_mobilenet_aesthetic.onnx` (12.3MB, Apache/MIT-ish — licença não declarada,
  usar com cautela comercial)
- `scrfd_2.5g_bnkps.onnx` (3.1MB, Apache-2.0)

**Se o diretório `core/models/` estiver vazio** (ex.: clone recente que ignora
binários), baixar de:
- SCRFD: `https://huggingface.co/RuteNL/SCRFD-face-detection-ONNX/resolve/main/2.5g_bnkps.onnx`
- NIMA: `https://huggingface.co/cromsc/nima-mobilenet-aesthetic/resolve/main/nima_mobilenet_aesthetic.onnx`

O `ort` com feature `coreml` baixa o ONNX Runtime pré-compilado no build
(usa GPU/Neural Engine no macOS, fallback CPU automático).

---

## 7. Notas técnicas críticas (evita re-descoberta)

- **NIMA = NHWC** `(1,224,224,3)`; **SCRFD = NCHW** `(1,3,640,640)`. Erro
  `"invalid dimensions for input"` = layout errado (troca HWC↔CHW).
- **`Session` do ort**: não clona e `run(&mut self)`. Com `OnceLock` imutável,
  usar **`Mutex<Session>`**.
- **`ComputeUnits::All`** (não `ALL`). `session.inputs()/outputs()` são **métodos**
  (campos privados). `name()` é método.
- **`Tensor::from_array(Array4)`** precisa da feature `ndarray` do ort. **Usar
  `ndarray = "0.17"`** (não 0.16!) para alinhar com a versão do ort (evita
  duplicação de tipos).
- `try_extract_tensor::<f32>()` retorna `(&Shape, &[T])`; `try_extract_array`
  retorna `ArrayViewD`.
- **react-window é v2.x** (API nova): `Grid` + `cellComponent`/`cellProps`,
  `defaultWidth`/`defaultHeight`, `onResize` (não usa width/height como props).
- **userData do app (preview):** `~/Library/Application Support/openshoot/catalog.db`
  (nome do package.json). `setup()` usa `OnceLock` por processo.
- **`jpgfromraw-lib` rejeitado** (build.rs exige nasm/CMake, panica). CR3 (HEIF)
  ainda não suportado (futuro: parser BMFF PRVW/THMB).

---

## 8. ONDE PARAMOS → PRÓXIMO PASSO SUGERIDO

O **pipeline de culling ML funciona** e a **detecção de faces SCRFD está refinada e
validada** (decisão multi-escala + NMS). Detalhes:

- **SCRFD implementado corretamente**: outputs reais `score_s [1,N,1]` (fg),
  `bbox_s [1,N,4]`, com N = (640/stride)² × 2 anchors (stride 8/16/32). Decodificação
  por escala + **NMS (IOU)**. Corrigido normalização por `scale*width/height`.
- **Validado**: detecta 1 rosto na imagem Lena, 0 em gradientes (discriminativo).
  A foto com rosto recebe bônus no score (+1.7 vs média) e aparece no topo do grid.
- **Debug NAPI**: `detectFacesInPath(path)` retorna faces detectadas (útil p/ testes).

**Próximos passos (ordem sugerida):**
1. **UI de picks/filtro + export XMP em massa** ✅ FEITO — filtros all/picks/rejects/
   unrated no header, botão Exportar XMP em massa (31 sidecars validado E2E).
2. **Melhorias Fase 1**: **CR3** ✅ FEITO (parser BMFF em core/src/cr3.rs, validado).
   Resta: dimensões via cabeçalho p/ imagens sem EXIF.
3. **Fase 3: edição em lote** ✅ FEITO — motor não-destrutivo (exposição/WB/contraste/saturação/sombras/realces/brilho) + painel de sliders com preview em tempo real + aplicar em lote (32 fotos validado).
4. **Fase 4**: retoque básico (segmentação de pele via ONNX).
5. **Fase 6**: OpenRouter opt-in (chave do usuário no Keychain).

---

## 9. Chamadas rápidas de verificação pós-reboot

```bash
# Confirma que está tudo no lugar
rtk ls ~/OpenShoot/core/models/                      # modelos ONNX
cmd git -C ~/OpenShoot status --short                # deve estar limpo (sem uncommitted)
cd ~/OpenShoot && node -e "console.log(require('electron/package.json').version)"   # electron ok
cd ~/OpenShoot && cargo test --manifest-path core/Cargo.toml 2>&1 | tail -1          # 10 passed
```

---

## 10. Deixando a sessão

Todos os commits foram feitos e **pushed** para `main`:
`229d8b6 → b025000`. Nada pendente no working tree.

**Ponto de partida ao voltar:** ler `docs/DESIGN.md` (arquitetura) e este doc
(seção 8 para o próximo passo).
