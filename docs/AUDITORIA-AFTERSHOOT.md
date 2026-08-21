# Auditoria funcional — AfterShoot × OpenShoot

> **Data:** 2026-08-18
> **Método:** auditoria funcional real (navegação por todos os menus + importação de
> 459 fotos reais + culling IA + painel de edição + painel de retoque), via árvore
> de acessibilidade. Este doc é a referência para melhorias do OpenShoot.
> **Nota:** a auditoria é funcional (não visual). Sempre cruzar com screenshots reais.

---

## 0. Sessão de auditoria 2026-08-20 (AfterShoot v2.21.4, Electron 31.7.7)

Observado em execução real (álbum "Documents" com IMG_CR3_001.CR3 e IMG_FACE_001.jpg):

### Modal "Iniciar Edição" (abre ao entrar num álbum)
- **Perfil AI Profissional** (Recomendado): "Carregue pelo menos **2.500** das suas
  próprias imagens editadas do Lightroom ou Capture One para treinar um Perfil de IA
  que reflita o seu estilo." → botão **Criar**.
- **Perfil AI Instantâneo**: "Carregue o seu **preset favorito do Lightroom** para
  criar um Perfil de IA em poucos minutos e editar as suas imagens no mesmo estilo."
  → botão **Criar**.
- Botão **Fechar** (sai do modal, volta ao álbum).

### Painel "Filtros de seleção" (modo CULL, lado direito) — seção recolhível + Reiniciar
- **Minhas Seleções** (contador).
- **Seleções da IA** (sub-expansível):
  - checkbox **"Seleções da IA"** (liga/desliga o grupo) + contador;
  - subitem **"selecionado"** (picks manuais) + contador;
  - subitem **"Destaques"** (top da IA) + contador — checkbox separado.
- **Para revisão** (fotos ambíguas p/ olhar humano) + contador.
- **Duplicatas** (agrupamento por hash) + contador.
- **Sem classificação** (unrated) + contador.
- **Rostos Principais (N)** — agrupamento por rosto com **slider de tolerância**
  (Value observado: 7) e ícone de rosto.
- **Duplicatas (N)** — sub-painel com thumbnails e botões (expandir/recolher),
  mensagem "Nenhuma duplicata encontrada" quando vazio.
- Botão **Reiniciar** (reset de todos os filtros).

### Dropdown "Outros" (toolbar do grid — mesmo filtro no IMPORT/EDIT/CULL)
- **AVISOS**: Todos / Com aviso / Sem aviso (fotos com flags de aviso, ex.: foco).
- **DUPLICADAS**: Todos / Com duplicatas / Sem duplicatas.
- **ROSTO**: All / With Faces / Without Faces.

### Modo IMPORT — rodapé "Começar"
Fluxo em etapas com 4 opções (cada uma com ícone + descrição):
1. **selecionando** — "A IA analisará e selecionará as melhores imagens, separando
   as demais para uma revisão mais fácil." (culling IA)
2. **Editando** — "Use um Perfil Profissional de IA ou um estilo de IA para cuidar
   da maior parte do seu trabalho de edição."
3. **Retoque** — "Dê os retoques finais nas suas fotos sem sair do aplicativo."
4. **Seleção e edição com um clique** — "Defina suas preferências, clique em iniciar
   e deixe o Aftershoot cuidar da seleção e edição." (pipeline completo)

### Topbar do workspace do álbum
- Modos: **IMPORT | CULL | EDIT | RETOUCH**.
- Ações: **Criar galeria**, **Exportar**.
- Toolbar da seleção (modo CULL/EDIT): estrelas **0-5**, **Editar status**,
  **Orientação**, **Informações...**, **Outros**, Tipo de Arquivo, zoom do grid,
  contador "X / Y".

### Tela "Meus Perfis de IA" (`#/profile/2`)
- Sub-navegação da conta: **Detalhes da conta / Meus Perfis de IA / Minha Assinatura /
  Indicação**.
- Header: "Criar um novo perfil de IA" + botões **Novo Perfil** e **Marketplace**.
- Seção **MERCADO** (perfis prontos da Aftershoot), cards com nome + tipo + tags de
  estilo e botões **Iniciar Edição** / **Ajustar perfil**:
  - **Almond Twist** (JPEG — Film, Warm), **Butter Pecan** (JPEG — Matte, True to Life),
    **Hazel Harmony** (JPEG — Low Key, Vintage), **True to Life** (RAW — Most Used),
    **Graicard** (RAW — Editorial, Vintage), **JPEG Graicard** (JPEG — Editorial, True to Life),
    **Brownies** (RAW — Editorial, Film).
- **FAQ de treinamento** (regras do Perfil Profissional):
  - Um perfil é treinado em **um único tipo de arquivo** (RAW *ou* JPEG) e **um tipo de
    cor** (Colorido *ou* Preto e Branco).
  - Usa **um único tipo de catálogo** (Lightroom *ou* Capture One — não mistura).
  - Botão "Continuar" desabilitado → falta preferências de edição e/ou **Filtros**
    aplicados aos catálogos (filtros por **Etiquetas, Estrelas, Bandeiras, Tipo de
    Câmera, Palavras-chave** no Capture One).

### Modo RETOUCH — verificado na versão atual (bate com seção 5)
- Toolbar adicional: botão **"Filtros de s..."** com badge de filtros ativos (ex.: 1),
  **Auto**, estrelas 0-5, Editar status, Tipo de Arquivo, Orientação, Informações, Outros.

---

## 1. Mapa de navegação do AfterShoot (menus e telas)

### Barra lateral esquerda (persistente)
- **Lar** — lista de álbuns (projetos).
- **Meus Perfis de IA** — perfis de edição do usuário + criar novo + marketplace.
- **Mercado** — perfis/estilos de terceiros.
- **Suporte** — chat Intercom (com artigos "How to Build Your First Professional AI
  Profile", "How to Use Aftershoot Retouching", "Get Started with Aftershoot Culling",
  "How to AI Edit Your Images With Aftershoot").
- **Profile Pic / avatar** — Detalhes da conta, Meus Perfis, Minha Assinatura, Indicação.

### Tela Lar (`#/projects`)
- Card "Criar Novo Álbum" → botão **Criar álbum**.
- Lista de **ÁLBUNS** com: thumbnail, nome, contagem de imagens, botão de ação
  contextual ("Cull Now" / "Retouch Now").
- Botão **Excluir** (multisseleção de álbuns).
- Card promocional "Criar perfil de IA".

### Wizard de projeto (topo da janela, 4 passos)
`IMPORT → CULL → EDIT → RETOUCH`
- Abas desabilitadas até o passo anterior ser concluído.
- Na tela de import, o passo selecionado fica ativo.

---

## 2. IMPORT (`#/project/import`)

### Entrada de fotos
- **Arrastar e soltar** pastas/imagens, OU
- Botão **Navegar** (abre open-panel nativo do macOS), OU
- Botão **Escolher entre as recentes**.
- Suporta **RAW / JPEG / TIFF** (fluxo completo: CULL, EDIT, RETOUCH) OU
  **catálogos Lr/C1** (apenas edições; visualizar no Lightroom/Capture One).

### Configurações de Importação (seção recolhível)
- **Tipo de fotos para importar**: CULL/EDIT/RETOUCH, RAW, JPEG/TIFF, APENAS EDIÇÕES.
- **Incluir subpastas** (toggle Sim/Não).

### Importar Definições (seção recolhível)
- **Baixar e fazer backup de fotos** (ingestão de cartão de memória antes de processar).
- **Ajude a melhorar a IA do Aftershoot** (compartilhar dados — opt-in de privacidade).

### Pós-seleção da pasta: "Que tipo de sessão é esta?"
- Botão **Sugerir um estilo** (IA detecta o tipo).
- Cards de gênero: **Casamentos E Noivados, Retrato E Fotos De Cabeça, Retratos De
  Família, Retrato Escolar, Recém-Nascidos, Esportes, Eventos Escolares, Boudoir,
  Algo Diferente**.
- "Sugerir um estilo" abre modal **"Solicitação de novo gênero"** (o usuário pode
  pedir que adicionem um gênero novo — campo de nome + descrição + Enviar).

### Durante a importação
- Barra de progresso ("Importação em andamento... %").
- Grid com as fotos já carregadas + contador "X / Y".
- Filtros de grid: **Tipo de Arquivo, Orientação, Informações...**.
- Botão **Adicionar Mais Imagens**.
- Rodapé com escolha de fluxo: **selecionando** (culling), **Editando**, **Retoque**,
  cada um com descrição e ícone.

---

## 3. CULL / seleção (`#/project/cull`)

### Modal "Preferências de seleção automatizada por IA"
- Tipo de sessão já detectado (ex.: "Retrato e Fotos de Cabeça").
- **Quantidade de fotos selecionadas** (slider).
- **Personalizar** → Classificação de estrelas e cores.
- **Sobrescrever classificações em arquivos XMP** (checkbox, on por padrão).
- Botão **Iniciar seleção**.

### Tela de culling
- Mensagem: "O AFTERSHOOT SELECIONA. VOCÊ ESCOLHE." / "O Aftershoot identifica
  **imagens desfocadas, olhos fechados e cria conjuntos de duplicados**. Você revê e finaliza."
- Botão **Iniciar seleção da IA**.
- Durante: progresso + tempo estimado (ex.: "11m 16s") + botão Cancelar.

### Grid do culling
- Cada foto: **5 botões de opção de estrelas (★1-5)**.
- Barra de filtros por rating: botões **0, 1, 2, 3, 4, 5**.
- Botões: **Criar galeria**, **Exportar**, **Adicionar Mais Imagens**.
- Filtros: Tipo de Arquivo, Orientação, Informações.
- Zoom do grid (slider) + contador "X / Y".

---

## 4. EDIT (`#/project/edit`)

### Painel "Preferências de Edição" (lateral direita)
- **Perfil de IA** — seletor de perfil (ex.: "Almond Twist JPEG Ajustado") +
  **Explorar perfis** + **Personalizar perfil**.
- **Recorte por IA** — opções **Suave** / **Padrão**.
- **Ajuste de Horizonte com IA**.
- **Máscara de IA**.
- Botão **Editar N Fotos** (aplica em lote).

### Modal "Personalizar perfil" (edição completa tipo Lightroom)
Contém **image strip** (várias fotos p/ comparar o ajuste) + seções com sliders:

1. **WHITE BALANCE** — Temperatura, Matiz.
2. **TOM** — Exposição, Contraste, Destaques, Sombras, Brancos, Pretos.
3. **PRESENÇA** — Textura, Nitidez, Desembaçar, Saturação, Vibração.
4. **NITIDEZ** — Quantidade, Raio, Detalhe, Máscara de IA.
5. **REDUÇÃO DE RUÍDO** — Luminância (+ Detalhe, Contraste), Cor (+ Detalhe, Suavidade).
6. **CURVA DE TOM** — Destaques, Luzes, Escuros, Sombras.
7. **MATIZ (HSL)** — Vermelho, Laranja, Amarelo, Verde, Azul-claro, Azul, Roxo, Magenta.
8. **SATURAÇÃO (HSL)** — mesmas 8 cores.
9. **LUMINÂNCIA (HSL)** — mesmas 8 cores.
- Botões: **Redefinir**, **Salvar e Continuar**.

### Grid do EDIT
- Mesmos filtros de rating (0-5), + **Editar status** (filtro de status de edição),
  Tipo de Arquivo, Orientação, Informações.
- Cada foto com 5 estrelas.

---

## 5. RETOUCH (`#/project/retouch`)

### Ferramentas (painel direito)
- Modos: **SUJEITO**, **FUNDO BETA**, **PATCH**.
- Filtro de sujeito: **Todas, Masculino, Feminino, Idoso, Criança, Individual, Selecionar individual**.
- **Mostrar Moldura do Rosto** (toggle).
- **Predefinições Recomendado** (cabeçalho recolhível).

### Sliders de retoque (por categoria)
- **Manchas do rosto** — Acne, Mancha, Sardas.
- **Rugas do rosto** — Testa, Sorriso.
- **Dentes** — Claro, Branco.
- **Realçar o rosto** — Claro, Alisa, Opaco.
- **Olhos** — Reflexo no vidro, Clarear olhos, Olheiras.
- **Cabelo** — Fios.
- **Corpo** — Alisa, Mancha, Desamassar roupa.
- Botões: **Colar**, **Redefinir**.
- Toolbar: **Auto**, Editar status, Tipo de Arquivo, Orientação, Informações, estrelas.

---

## 6. Gap analysis — OpenShoot vs AfterShoot (priorizado)

| # | Área | AfterShoot | OpenShoot (hoje) | Gap / sugestão |
|---|---|---|---|---|
| 1 | **Perfis de IA / estilos** | criar perfil, mercado, personalizar, salvar | 8 sliders manuais, "aplicar em lote", sem presets salvos | Criar sistema de **presets nomeados** (salvar/carregar receita JSON) + biblioteca de estilos |
| 2 | **Importação** | wizard (tipo fotos, subpastas, backup, tipo de sessão) + filtros de grid | diálogo nativo → grid direto | Adicionar wizard: tipo de fotos, incluir subpastas, escolha de tipo de sessão |
| 3 | **Filtros de grid** | Tipo de Arquivo, Orientação, rating 0-5, editar status | all/picks/rejects/unrated | Adicionar filtro por orientação, câmera e rating numérico |
| 4 | **Estrelas por foto** | ★1-5 clicável em cada célula | flag P/X + score | Adicionar classificação ★1-5 por foto no grid |
| 5 | **Culling: meta de quantidade** | pergunta "quantas fotos selecionar" | culla tudo | Adicionar meta de nº de picks |
| 6 | **Culling: duplicados** | agrupa conjuntos de duplicatas + filtro "Com/Sem duplicatas" | não agrupa | Embedding + agrupamento de duplicatas/semelhantes |
| 7 | **Culling: olhos fechados** | detecta olhos fechados + avisos (Com aviso/Sem aviso) | não detecta | Detecção de olhos (SCRFD/landmarks) + flag de aviso |
| 8 | **Edição tonal** | curva de tom + HSL (8 cores) + nitidez + ruído | 8 sliders básicos | Expandir: curva, HSL, nitidez, redução de ruído |
| 9 | **Recorte / horizonte / máscara IA** | recorte por IA (suave/padrão), ajuste de horizonte, máscara de IA | não tem | Roadmap: crop com IA, endireitar horizonte |
| 10 | **Retoque facial** | acne, rugas, dentes, olhos, cabelo, corpo | suavização de pele + remover distração (bbox central fixa) | Detecção de rosto + **seleção por arrasto no patch** + sliders faciais |
| 11 | **Seleção de região p/ inpainting** | patch com seleção | bbox fixa central (MVP marcado no código) | Implementar seleção por arrasto |
| 12 | **Culling: "Para revisão"** | bucket de fotos ambíguas p/ revisão humana | não tem | Marcar fotos com score limítrofe para revisão humana |
| 13 | **Culling: "Destaques" vs "Selecionado"** | separa picks da IA (destaques) de picks manuais | só rating | Adicionar bucket "destaques IA" + "selecionado manual" |
| 14 | **Culling: "Rostos Principais"** | agrupa por rosto + slider de tolerância | detecta rosto p/ score, não agrupa | Agrupar fotos por rosto detectado (landmarks/embedding facial) |
| 15 | **Filtros avançados** | AVISOS / DUPLICADAS / ROSTO (dropdown "Outros") | só P/X/U | Dropdown com filtros por aviso, duplicata e presença de rosto |

---

## 7. O que o OpenShoot já faz BEM (não regredir)
- Grid virtualizado (react-window) + atalhos P/X/U/1-5 + loupe com navegação.
- Culling ML local (NIMA + SCRFD multi-escala + NMS) validado E2E.
- Edição não-destrutiva em lote com preview + XMP compatível LR/C1.
- Retoque local (pele, inpainting) + empacotamento macOS funcionando.

---

## 8. Referências de código no OpenShoot
- UI: `src/renderer/src/App.tsx`, `components/Gallery.tsx`, `components/EditPanel.tsx`,
  `components/LoupeView.tsx`.
- Core (Rust): `core/src/` — `catalog.rs`, `lib.rs`, `culling.rs`, `xmp.rs`, `imageproc.rs`, `cr3.rs`.
- Docs: `docs/DESIGN.md`, `docs/PROGRESSO.md`, `docs/SESSAO.md`.
