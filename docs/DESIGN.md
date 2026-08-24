# OpenShoot — Design Document

> **Inspiração:** referência externa (Electron + React UI, backend Rust "Cromatela", ONNX Runtime na GPU).
> **Diferença:** OpenShoot é **open-source** (MIT), usa apenas **modelos de IA abertos** — nenhum peso proprietário, e o usuário tem acesso total ao código-fonte.

**Pilares inegociáveis:**
1. **100% local & offline** — pixels nunca saem da máquina (IA de visão roda na GPU via ONNX).
2. **Open source** — diferente da referência externa (repo privado + modelos criptografados), todo o código é público e auditável.
3. **Chaves de serviços externos pertencem ao usuário** — se houver uso de nuvem (ex: OpenRouter p/ texto), é **opt-in** e a chave é do usuário, armazenada no Keychain, nunca no código.

**Status:** Planejamento · **Alvo:** macOS (arm64 + x86_64) · **Data:** 2026-08-17

---

## 1. Visão Geral

Aplicativo desktop para fotógrafos que automatiza o pós-processamento de fotos:

1. **Culling** — ranquear milhares de fotos e selecionar as melhores automaticamente.
2. **Edição** — aplicar estilo/ajustes em lote de forma consistente.
3. **Retoque** — remoção de distrações, pele, troca de fundo.
4. **Exportação** — XMP sidecars + export p/ Lightroom/Capture One/PS.

Processamento 100% local, não-destrutivo, offline.

---

## 2. Arquitetura (3 camadas)

```
┌─────────────────────────────────────────────────────────────┐
│ 1. UI  Electron (TypeScript + React + Tailwind)             │
│    - Galeria / Grid / Loupe / Comparação / Edição manual    │
│    - Progresso de jobs (IPC)                                │
└───────────────▲─────────────────────────────────────────────┘
                │ Bundled NAPI (napi-rs) — IPC de alto desempenho
┌───────────────┴─────────────────────────────────────────────┐
│ 2. Backend Core  Rust (crate "openshoot-core")              │
│    - RAW decode (LibRaw/RawSpeed)                           │
│    - Pipeline de IA (ONNX Runtime, GPU via Metal)           │
│    - Culling scorer, estilos, retoque, XMP writer           │
│    - Fila de jobs + cache de thumbnails/previews            │
└───────────────▲─────────────────────────────────────────────┘
                │ FFI/NAPI
┌───────────────┴─────────────────────────────────────────────┐
│ 3. Modelos  Arquivos .onnx (open) em assets/                │
│    - Face/olhos: SCRFD ou DSFD (detecção)                   │
│    - Qualidade/score: NIMA (classifier) ou BRISQUE+heurística│
│    - Parsing/skin: MediaPipe Iris/SelfieSegmentation ONNX   │
│    - Inpainting/upscale: Real-ESRGAN / LaMa (opcionais Fase 5)│
└─────────────────────────────────────────────────────────────┘
```

### Comunicação UI ⇄ Backend
- Usar [**napi-rs**](https://napi.rs) (Rust → Node, ABI estável, `.node` bundle) — é o mesmo mecanismo que a referência externa usa (`rs.darwin-arm64.node` presente no instalado).
- Chamadas: `import { OpenShootCore } from '../core'` → spawn `Job` async e emite eventos de progresso via callback NAPI.

---

## 3. Stack

| Camada | Tecnologia | Justificativa |
|---|---|---|
| UI | Electron 31+, React 18, TypeScript, Tailwind, Vite/(electron-forge webpack) | mesmo modelo de referência |
| Bridge | napi-rs (Rust) | IPC eficiente, sem servidor HTTP |
| Backend | Rust (edition 2021) | performance p/ processamento de imagem |
| RAW | `libraw` (crate `libraw-rs` / bindings) e/ou `rawspeed` | decode de CR3/NEF/ARW/DNG |
| ML runtime | `onnxruntime` crate (via `ort` — bindings Rust ONNX) | executa modelos com acelerador Metal/CUDA |
| Visão | `image`, `imageproc` (Rust) + OpenCV C++ (opcional) | pré-processamento e filtros clássicos |
| Persistência | SQLite (crate `rusqlite`) c/ sidecar XMP | catálogo + metadata |
| Tarefas | `tokio` + rayon (pool p/ CPU) | paralelismo |

---

## 4. Estrutura de Pastas

```
OpenShoot/
├─ package.json            # Electron app + scripts
├─ electron/
│  ├─ main.ts              # janela, lifecycle, ativa o NAPI
│  └─ preload.ts
├─ src/                    # React UI
│  ├─ gallery/  editor/  jobs/  settings/
│  └─ components/          # design-system (Storybook)
├─ core/                   # Rust crate (openshoot-core)
│  ├─ Cargo.toml
│  ├─ src/
│  │  ├─ lib.rs            # exports NAPI
│  │  ├─ raw.rs            # decode RAW (LibRaw)
│  │  ├─ culling.rs        # scoring (nitidez, faces, blur, dupe)
│  │  ├─ edit.rs           # estilos/ajustes em lote
│  │  ├─ retouch.rs        # inpainting/skin/background
│  │  ├─ xmp.rs            # XMP sidecar writer
│  │  ├─ job.rs            # fila + cancelamento + progresso
│  │  └─ db.rs             # SQLite catálogo
│  ├─ models/              # .onnx (baixados no primeiro setup)
│  └─ assets/              # filtros clássicos, LUTs
└─ docs/
```

---

## 5. Modelos Open-Source (substituindo o `.jumbled` proprietário)

| Recurso | Modelo aberto | Formato | Notas |
|---|---|---|---|
| Detecção de face/olhos | SCRFD / RetinaFace (ONNX export) | .onnx | ótimo p/ faces em ações |
| Qtd/qualidade img | NIMA (Google, ONNX) | .onnx | score estético 1–10 |
| Nitidez/blur | heurística Laplacian (OpenCV/imageproc) | — | rápido, sem rede |
| Duplicatas/percepção | embedding via CLIP ViT (ONNX) | .onnx | similaridade p/ agrupamento |
| Segmentação p/ retoque | SelfieSegmentation (MediaPipe) · BiSeNet | .onnx | máscara de pele/fundo |
| Upscale/denoise | Real-ESRGAN x2/x4 (ONNX) | .onnx | Fase 5, opcional |
| Inpainting (remover objeto) | LaMa | .onnx | Fase 5, opcional |

> Modelos até ~100 MB baixados sob demanda (CRCs verificados) em `core/models/` — nunca viajar no repo.

---

## 6. Roadmap por Fases

### Fase 0 — Esqueleto (fundação)
- [ ] Workspace electron-forge + crate Rust, napi-rs integrado
- [ ] Janela aberta, hello-world IPC (UI ⇄ Rust)
- [ ] CI/`cargo test` para core

### Fase 1 — Catálogo & RAW
- [ ] Import de pastas → SQLite (caminho, hash, câmera, EXIF)
- [ ] Decode RAW + geração de thumbnails/previews (LibRaw)
- [ ] Grid/Loupe na UI com virtualização

### Fase 2 — Culling (MVP de IA)
- [ ] Detecção de faces (SCRFD) + olhos fechados/nitidez de rosto
- [ ] Score: nitidez global (Laplacian) + composição + qualidade (NIMA)
- [ ] Ranqueamento + Picks/Rejects com XMP rating/label
- [ ] Agrupamento de duplicatas por embedding CLIP

### Fase 3 — Edição em lote
- [ ] Presets (exposição, balanço, contraste, sombras, LUTs)
- [ ] Aplicação não-destrutiva em lote + preview em tempo real
- [ ] "Aprender" estilo por amostras (média de tom/matriz) — simples v1

### Fase 4 — Retoque básico
- [ ] Máscara de pele (SelfieSegmentation) + suavização/limpeza
- [ ] Redução de brilho de óculos (detecção de glare)
- [ ] Remoção de distrações via inpainting LaMa

### Fase 5 — Export & Delivery
- [ ] Export JPEG/TIFF com ajustes aplicados
- [ ] XMP sidecars (ratings, labels, keywords) p/ LR/C1/PS
- [x] (Opcional) Galeria web estática p/ compartilhamento — `create_web_gallery`
  (`core/src/gallery.rs` + IPC `core:createWebGallery`, UI `GalleryExport.tsx`):
  copia as fotos para `<dest>/photos/`, gera thumbs 400px em `<dest>/thumbs/`
  e escreve um `index.html` self-contained (dark theme, grid responsivo,
  lightbox CSS `:target`, zero dependências externas, 100% offline).

### Fase 6 — Material opcional via OpenRouter (opt-in)
> **Não afeta os pixels.** Só texto/organização. Chave = do usuário (Keychain), nunca no código/env.

- [ ] Auto-keywords, títulos e descrições de álbuns a partir de EXIF + tags de cena locais
- [ ] Sugestão de organização/comparação de ensaio (casamento, newborn, esportes)
- [ ] Legendas/e-mails para clientes
- [ ] Modo "descreva previews" (somente thumbnails que o usuário optar por enviar)

---

## 7. Decisões de Risco

1. **Modelos não igualam a qualidade da referência externa** — resolvido por transparência: medidas objetivas + presets do usuário.
2. **ONNX + Metal**: `ort` precisa de feature `metal`. Fallback CPU se GPU indisponível.
3. **RAW decode completo** (não só thumbnail) é caro — pipeline em dois níveis: preview rápido + processamento full-res sob demanda.
4. **Licenças**: preferir modelos Apache-2.0/MIT (Real-ESRGAN BSD-3, SCRFD MIT-ish, LaMa Apache-2.0). Registrar cada licença em `THIRD_PARTY.md`.
5. **Privacidade (OpenRouter)**: módulo 100% opt-in; chave do usuário no Keychain; CSP bloqueia exfiltração acidental; aviso claro antes de qualquer envio. Etiqueta padrão: "nunca envie fotos sem consentimento explícito".

---

## 8. Definição de Pronto (MVP — Fase 0–2)

- [ ] O app roda no macOS desta máquina, importa uma pasta de JPG/RAW e mostra grid.
- [ ] Botão "Cull" ranqueia as fotos em <30s p/ 500 imagens; picks viram XMP.
- [ ] `cargo test` verde + `tsc --noEmit` limpo.
- [ ] Nenhuma imagem sai da máquina.
