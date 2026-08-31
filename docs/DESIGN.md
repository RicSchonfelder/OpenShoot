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

### Sistema visual e temas

A interface usa tokens CSS semânticos para superfícies, bordas, tipografia,
ações e estados. O renderer aplica o tema no atributo `data-theme` do elemento
raiz; componentes nunca devem introduzir cores de interface fixas fora desses
tokens. Cores de etiquetas fotográficas e controles HSL são exceções, pois
representam metadados e canais de cor, não a aparência do aplicativo.

O tema padrão é **Café**, pensado para uma área de trabalho de fotografia com
contraste suave e acento quente. O botão **Configurações** abre uma janela de
aparência que permite alternar entre os modos Escuro e Claro e entre Café,
Grafite, Oceano ou Floresta. As escolhas ficam somente no `localStorage` do
renderer (`openshoot-theme` e `openshoot-appearance`), persistem entre
aberturas e não são transmitidas nem sincronizadas com serviços externos.

O sistema de foco é visível para teclado, os controles compartilham altura,
bordas e estados de interação, e `prefers-reduced-motion` reduz animações. Ao
criar uma nova tela, usar os componentes/classes globais de botão e os tokens
`--canvas`, `--surface`, `--border`, `--text-*` e `--accent*` para manter a
coerência visual.

Fluxos de trabalho de tela cheia (exportação, lupa, pessoas e restauração)
possuem raiz visual própria: não podem ser renderizados sobre a galeria nem
deixar ações do fundo acessíveis. A exportação ocupa a janela inteira. No
culling, os indicadores do painel lateral são calculados no mesmo escopo do
álbum aberto, e não no catálogo global. A restauração local é a rota principal;
recursos de IA online ficam recolhidos, explicitamente opcionais e com o aviso
de possível cobrança antes de qualquer configuração ou uso.

Dentro de um álbum, a navegação primária é sempre a mesma e ocupa a posição
central do cabeçalho: **Importar, Seleção, Editar, Retoque e Exportar**. A
mesma barra também aparece nas visualizações de foto, exportação, pessoas e
restauração. Ferramentas contextuais (filtros, executar seleção, pessoas,
restauração, importação e configurações) ficam nas laterais e nunca substituem
ou deslocam essa navegação. A tela inicial de álbuns é a exceção deliberada,
por ser o seletor de sessão antes de existir um álbum ativo.

Na galeria, um clique seleciona a foto; abrir uma ferramenta é explícito
(duplo clique ou `Enter`). Em Seleção, essa abertura leva à lupa; em Editar e
Retoque, leva à edição da foto. Isso preserva o fluxo de culling e evita que
uma seleção simples troque de contexto. Aplicações em lote de edição usam
somente as fotos selecionadas e gravam os respectivos sidecars XMP — nunca o
catálogo inteiro por engano.

A exportação possui uma única área de configuração, seja para fotos visíveis
ou para a seleção atual; a tela declara esse escopo antes da confirmação. As
opções expostas refletem o pipeline atual: JPEG ou PNG, sRGB ou aproximação de
Display P3. Recursos não suportados não são apresentados como escolhas.

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

### Agrupamento de pessoas

`PeopleView` exibe uma capa por grupo facial. O core retorna, além da foto de
amostra, a caixa da face que corresponde ao embedding representativo do grupo
(`sample_face`, em coordenadas normalizadas). A UI usa esse enquadramento para
ampliar o rosto correto mesmo quando a foto contém várias pessoas; não deve
escolher simplesmente a maior face da imagem.
Quando aberta dentro de um álbum, a lista de IDs do álbum é enviada pelo IPC e
é usada tanto no agrupamento quanto na exportação; sem álbum aberto, o catálogo
completo continua sendo o escopo padrão.

O culling e os contadores do painel também recebem esse escopo. Assim, as
quantidades e as marcações de seleção não misturam fotos de outros álbuns.

> Modelos até ~100 MB baixados sob demanda (CRCs verificados) em `core/models/` — nunca viajar no repo.

---

## 6. Roadmap por Fases

### Bancada de restauração (experimental, local)

O app inclui uma tela independente de teste que combina o pipeline local já
existente (`preview_edit`) para nitidez, redução de ruído, exposição, cor e
alinhamento. A tela mostra original/prévia e salva somente uma cópia JPEG por
diálogo do Electron; o arquivo original nunca é sobrescrito. O modo local não
faz chamada de rede nem usa chave externa.

O modo de revelação ocupa a área principal com uma foto ampliada e um filmstrip
horizontal do álbum na parte inferior. A roda do mouse sobre o filmstrip move a
faixa lateralmente, e a foto ativa pode ser comparada lado a lado com a prévia
modificada enquanto os controles de edição permanecem disponíveis.

O modo experimental **IA online** é uma extensão separada: envia fotos
selecionadas ao endpoint `/v1/images/edits` da OpenAI usando um modelo selecionado
e validado entre `gpt-image-2`, `gpt-image-1` e `gpt-image-1-mini`, após
uma única confirmação explícita por lote. A chave é criptografada pelo Electron (`safeStorage`)
no perfil local e nunca é enviada ao renderer ou gravada no código. Esse modo
pode gerar cobrança e não altera o status do modo local, que continua padrão.
Cada tentativa online registra metadados técnicos sem chave, prompt ou pixels em
`openai-usage.jsonl` no perfil do usuário, com opção de exportação pela bancada.
O registro inclui um `clientRequestId` UUID próprio, o `x-request-id` devolvido
pela API quando presente e os cabeçalhos de limite de requisições/tokens, sem
armazenar a chave. Cada chamada tem timeout de 180 segundos. Timeout ou erro de
rede é tratado como resultado indeterminado: não há retry automático, porque a
requisição pode ter sido recebida e cobrada mesmo sem resposta chegar ao app.
Cada resultado é gravado imediatamente em `restoration-cache/` no perfil do
usuário, com validação de tamanho e data de modificação do original ao reabrir;
o processamento online usa até três requisições simultâneas para reduzir o
tempo do lote, mantendo o limite explícito e registrando `elapsedMs` por foto.
Antes da primeira requisição, a bancada exige a escolha da pasta de destino e
reutiliza essa pasta na exportação. A exportação usa criação exclusiva: se um
nome já existir, cria um sufixo numérico em vez de sobrescrever um arquivo.
Ao reabrir a bancada, resultados presentes no cache são marcados como prontos e
não são reenviados quando o usuário retoma o lote; requisições em andamento no
momento do fechamento não podem ser continuadas, pois a API de imagens não
oferece retomada parcial de uma requisição. O próximo passo de robustez é
persistir um manifesto de lote com pendências e destino, para tornar essa
retomada explícita e auditável.
Como alternativa, `OPENAI_API_KEY` pode ser fornecida no ambiente do processo;
ela tem precedência sobre a chave criptografada e nunca é persistida pelo app.

O preview da bancada mantém o mesmo zoom para original e resultado, aceita
zoom pela roda do mouse e oferece comparação deslizante com divisor arrastável.
O modo de edição do álbum oferece os modos original, modificada, lado a lado e
comparação deslizante, com fallback para a foto original quando não há edição.

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


## 9. Multi-plataforma (Windows/Linux)

O core resolve caminhos em runtime (modelos, cache, lixeira nativa via crate
`trash`, EP de ONNX por SO: CoreML no macOS, DirectML no Windows). Detalhes da
auditoria e dos fixes em docs/MULTIPLATAFORMA.md.
