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
uma seleção simples troque de contexto. A ferramenta "Pessoas" é uma ação
estável do cabeçalho esquerdo do álbum, visível em Importar/Seleção/Editar/
Retoque, sem mudar de lugar. PeopleView recebe e reflete a seção ativa atual
no WorkspaceNav. No editor detalhado, acesso equivalente no cabeçalho.
Aplicações em lote de edição usam somente as fotos selecionadas e gravam os
respectivos sidecars XMP — nunca o catálogo inteiro por engano.

A galeria usa uma malha virtualizada responsiva: a largura da área efetivamente
disponível (já descontados filtros e painel de edição) é observada durante o
redimensionamento, e as colunas redistribuem essa largura sem ultrapassar a
viewport. Assim, a última miniatura não fica cortada nem muda de posição ao
alternar entre Importar, Seleção, Editar e Retoque.

A exportação possui uma única área de configuração, seja para fotos visíveis
ou para a seleção atual; a tela declara esse escopo antes da confirmação. As
opções expostas refletem o pipeline atual: JPEG ou PNG, sRGB ou aproximação de
Display P3. Recursos não suportados não são apresentados como escolhas.

### Persistência e armazenamento

O catálogo SQLite continua sendo a fonte canônica dos metadados. Por padrão,
ele fica no diretório de dados do usuário, mas a janela Configurações permite
escolher outro diretório. A alteração é salva em `storage-settings.json` e
entra em vigor após reiniciar o aplicativo; quando possível, o banco atual é
copiado para a nova localização sem apagar a origem. O cache de thumbnails tem
local configurável e pode ser apagado sem perda de dados.

O comando de exportação do catálogo gera um manifesto JSON versionado
(`format: openshoot-catalog`, `schema_version: 1`). O manifesto guarda caminhos,
hashes, metadados, álbuns, grupos de pessoas e bounding boxes, mas nunca pixels
ou thumbnails. A importação reconcilia fotos por caminho e, como fallback, por
SHA-256; arquivos que não existem no computador atual são reportados sem serem
criados artificialmente. Isso permite backup e transporte de organização sem
transformar JSON em banco de consulta.

Os originais permanecem referenciados no local escolhido pelo usuário. Copiar
originais para um projeto portátil é uma operação explícita de exportação e
não acontece ao criar um álbum.

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

#### Reconhecimento de pessoas — Fase 1 (persistência local)

O agrupamento facial agora persiste resultados em tabelas SQLite locais:

- **`person_groups`**: `id`, `album_id` (pertence ao álbum), `name` (editável),
  `threshold` (similaridade usada).
- **`photo_person_faces`**: `id`, `group_id` (FK), `photo_id` (FK),
  `bbox_x1/y1/x2/y2` (coordenadas normalizadas 0..1).

**Migrações idempotentes**: as tabelas são criadas via `CREATE TABLE IF NOT
EXISTS` no `ensure_schema`, garantindo que reaberturas do app não quebrem.

**Fluxo de agrupamento com persistência**:
1. `group_by_similarity_async(threshold, photo_ids, album_id)` detecta faces,
   gera embeddings, agrupa por cosseno e retorna `{ok, groups, grouped_faces,
   photos_scanned, photos_unavailable}`.
2. Cada `GroupedFace` traz `group_index`, `photo_id` e `bbox` preservada.
3. Se `album_id` é fornecido, os grupos são persistidos atomicamente
   (transação SQLite): os grupos antigos do álbum são substituídos pelos novos.
4. `photos.has_face` é atualizado somente para as fotos que puderam ser lidas
   (true/false). Se todas estiverem indisponíveis, a operação retorna erro
   acionável e preserva os grupos existentes; se apenas algumas falharem, a UI
   mostra um aviso com a contagem sem interromper os resultados válidos.

**APIs de consulta** (100% locais, offline):
- `listPersonGroups(albumId)` → lista grupos do álbum.
- `listFacesInGroup(groupId)` → lista faces de um grupo com bbox.
- `listFacesForPhoto(photoId)` → lista todas as faces de uma foto em todos os
  grupos.
- `renamePersonGroup(groupId, newName)` → renomeia um grupo.
- `exportPersistedPeopleAlbum(albumId, outDir)` → exporta os grupos persistidos
  para pastas nomeadas (nomes sanitizados, sem sobrescrever colisões). Retorna
  `{ok, out_dir, groups, exported}`.

Todas as APIs retornam `{error}` preservando a mensagem em caso de falha.

#### Reconhecimento de pessoas — Fase 2 (UI integrada)

A UI de pessoas agora opera dentro do escopo do álbum aberto:

- **PeopleView** recebe `albumId` obrigatório e `activeSection` (reflete a seção
  ativa no WorkspaceNav). Carrega grupos persistidos no mount via
  `listPersonGroups(albumId)` + `listFacesInGroup` para cada grupo. Sair e voltar
  mantém os grupos (persistidos no SQLite). Estado de carregamento/erro do escopo
  de fotos do álbum é tratado explicitamente: ações ficam desabilitadas enquanto
  carrega ou quando o álbum está vazio.
- **Botão "Identificar pessoas"** (pt-BR) / "Identify people" (en) executa o
  agrupamento. Um aviso persistente informa que reanalisar substitui os grupos/
  nomes atuais.
- **Exportação** usa `exportPersistedPeopleAlbum(albumId, outDir)` — exporta os
  grupos persistidos com nomes sanitizados para pastas, sem reexecutar agrupamento
  nem ignorar nomes renomeados. Não sobrescreve colisões (sufixo numérico).
  Feedback de sucesso exibido como toast, sem `window.alert`.
- **Renomear** inline com Enter/Escape, usando `renamePersonGroup`. Erros são
  visíveis e editáveis.
- **Cards acessíveis** abrem detalhes do grupo com miniaturas de todas as fotos.
  Cada foto tem ações "Abrir em Editar" e "Abrir em Retoque" que abrem a foto
  no modo correspondente sem trocar o modo pedido.
- **Navegação**: "Pessoas" é uma ferramenta contextual dentro de Seleção,
  ao lado do comando de culling. Não aparece como uma etapa independente do
  processo principal; ao abrir, a aba Seleção permanece ativa como contexto.
- **Revisão visual**: os cards usam o `bbox` persistido para recortar o rosto
  representativo, em vez de repetir a foto inteira. Abrir um card mostra os
  rostos associados às fotos do grupo; "Nomear pessoa" torna a confirmação do
  nome explícita antes da exportação.
- **EditViewPhoto** ganha toggle "Mostrar pessoas" que desenha bbox e
  `group_name` na imagem, acompanhando zoom/pan. Imagem e overlay estão na
  mesma camada (`editview-media-layer`) com o mesmo transform. Quando não há
  faces, o toggle fica desabilitado. O botão 1:1 calcula zoom de 1 pixel do
  preview para 1 pixel de tela (naturalWidth/contain). No slider, overlay fica
  oculto com título explicativo.
- **EditPanel** ganha seção "Pessoas" somente leitura com nomes únicos das
  faces detectadas. Quando vazio, orienta analisar em Seleção.

O App carrega `listFacesForPhoto` para a foto aberta e passa os dados a
`EditViewPhoto` (overlay de bbox) e `EditPanel` (lista de nomes).

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

### Fase 7 — Reconhecimento de pessoas (local)
- [x] Detecção SCRFD + embeddings MobileFaceNet + agrupamento por cosseno
- [x] Persistência: `person_groups` + `photo_person_faces` com bbox normalizada
- [x] Substituição atômica por álbum (`replace_person_groups`)
- [x] Atualização de `photos.has_face` para todas as fotos analisadas
- [x] APIs: listar grupos, listar faces, renomear grupo, faces por foto
- [x] IPC/preload/tipos TS explícitos (sem `any`)
- [x] Testes Rust: migration idempotente, roundtrip, renomeação, has_face

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
