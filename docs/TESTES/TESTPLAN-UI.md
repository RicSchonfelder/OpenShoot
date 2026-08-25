# Plano de Testes E2E — OpenShoot (para agentes)

> **Para:** agentes de teste executando roteiros automatizados ou manuais.
> **Setup:** `cd ~/OpenShoot && npm run build:core && npm run typecheck && npx electron-vite preview`
> **Fotos:** `~/Pictures/test-459` (cópia local de Brotas/Editadas; criar se não existir).
> **Como testar UI:** Computer Use (get_app_state/click/type) no app "OpenShoot" ou
> "Electron"; como testar core: scripts node via NAPI (exemplos em METRICAS-BASELINE.md).
> **Regra:** cada caso tem ID; registre PASS/FAIL + evidência (screenshot ou output).
> NÃO commite — reporte os resultados.

## TP-01 Importação
- [ ] TP-01.1 Criar álbum "Teste E2E" na Lar → aparece card com contagem 0.
- [ ] TP-01.2 Entrar no álbum → aba IMPORT → Importar pasta → escolher `test-459`
      → opções: subpastas OFF, tipo Todos → Iniciar.
- [ ] TP-01.3 Aguardar; validar: toast de progresso aparece; contagem final 459;
      grid mostra thumbnails (amostrar 10 células).
- [ ] TP-01.4 Repetir import da MESMA pasta → esperado: updated 459, added 0
      (idempotente por SHA-256).
- [ ] TP-01.5 Import com filtro "RAW" numa pasta só-JPEG → esperado 0 importadas.
- [ ] TP-01.6 Tipo de sessão "Esportes" → reabrir álbum → sessão visível no card da Lar.

## TP-02 Culling
- [ ] TP-02.1 Aba CULL → botão "Selecionar" → aguardar. Validar: toast com
      processado/erros; grid com ratings ★1-5; flags verde/vermelho presentes.
- [ ] TP-02.2 Toolbar: contadores P/X/★IA/✔/?/U > 0 e somando ≤ total.
- [ ] TP-02.3 Meta de seleção: slider em 10 → rodar "Selecionar" → esperado exatamente
      10 fotos ★5 e contador ★IA = 10.
- [ ] TP-02.4 Teclado: clicar numa foto → P (fica ★5 verde), X (★1 vermelho),
      U (limpa), 1-5 (rating direto), setas navegam, Shift+seta estende seleção.
- [ ] TP-02.5 ⌘A seleciona todas; Del abre diálogo 3 opções; Cancelar não apaga.
- [ ] TP-02.6 "Remover só do catálogo" em 1 foto → arquivo continua no disco
      (verificar via Finder/ls); "Mover para Lixeira" → arquivo sai da pasta.
- [ ] TP-02.7 Filtros: cada item do painel lateral filtra corretamente (picks,
      rejects, unrated, review, destaques, selecionado, duplicatas, faces,
      retrato, paisagem, raw, jpeg, edited, unedited) — comparar contadores.
- [ ] TP-02.8 Dropdown Outros → "Reiniciar filtros" volta para Todos.

## TP-03 Loupe
- [ ] TP-03.1 Duplo clique numa foto abre loupe; contador "N / 459 · nome".
- [ ] TP-03.2 ← → navegam; P/X/U/1-5 aplicam e avançam; Esc fecha.
- [ ] TP-03.3 Foto em retrato aparece CORRETA (não deitada) — orientação EXIF.
- [ ] TP-03.4 Foto aparece POR INTEIRO (contain, sem corte) em janela redimensionada.
- [ ] TP-03.5 Zoom: Fit → 100% → +/− → wheel zooma no cursor; zoom>1 arrasta (pan);
      trocar de foto reseta zoom.
- [ ] TP-03.6 "Moldura do rosto" liga overlay verde nas faces (foto com pessoa).
- [ ] TP-03.7 Patch: com zoom=1, arrastar retângulo → "Remover região" → preview
      sem a distração.

## TP-04 Edição
- [ ] TP-04.1 Clique simples numa foto abre MODO EDIÇÃO em tela grande (foto inteira
      à esquerda, painel à direita); ←→ troca de foto; Esc volta à galeria.
- [ ] TP-04.2 Slider Exposição +0.5 → preview atualiza em ~200ms; foto escura clareia.
- [ ] TP-04.3 Temperatura 8000K → tom mais quente; Tint funciona.
- [ ] TP-04.4 Curva de tom: Sombras -50 → escurece só as sombras.
- [ ] TP-04.5 HSL: cor Red saturação -100 → vermelhos dessaturados; Green matiz muda.
- [ ] TP-04.6 Nitidez 60 → bordas mais definidas; Ruído 60 → ruído suaviza.
- [ ] TP-04.7 "Ajustar horizonte (IA)" numa foto torta → rotação aplicada no preview.
- [ ] TP-04.8 "Recorte por IA" numa foto com pessoa → recorte centrado no sujeito.
- [ ] TP-04.9 Presets: salvar preset "Teste" com ajustes → aparece na lista; carregar
      em outra foto aplica; deletar remove. Badges RAW/JPEG visíveis se definidos.
- [ ] TP-04.10 "Importar preset do Lightroom" com um .xmp crs: → importa e aplica.
- [ ] TP-04.11 "Aplicar em lote" → toast com N fotos; reabrir outra foto → receita
      aplicada (preview consistente).
- [ ] TP-04.12 "Aprender perfil" após editar ≥2 fotos → cria preset "Perfil aprendido".

## TP-05 Retoque
- [ ] TP-05.1 Suavização de pele 50% → pele suave sem plástico (foto com rosto).
- [ ] TP-05.2 Acne 50% / Clarear olhos 40% / Dentes 30% / Cabelo 30% → mudanças
      localizadas na região certa do rosto.
- [ ] TP-05.3 "Máscara de sujeito" → fundo desfocado, pessoa nítida.
- [ ] TP-05.4 "Aplicar retoque em lote (Colar)" com pele 50% + acne 30% em 3 fotos
      selecionadas → pasta escolhida contém 3 JPEGs retocados em resolução nativa.

## TP-06 Exportação
- [ ] TP-06.1 Selecionar 20 fotos → Exportar → escolher pasta → JPEG q90 →
      20 arquivos, resolução nativa, edição aplicada (comparar 3 com o preview).
- [ ] TP-06.2 Exportar de novo na mesma pasta → sufixos _1 sem sobrescrever.
- [ ] TP-06.3 Naming `{n}_{original}` → arquivos com prefixo numérico.
- [ ] TP-06.4 PNG → arquivos .png válidos.
- [ ] TP-06.5 Dica "⌘A" aparece quando só 1 foto selecionada.
- [ ] TP-06.6 "Exportar XMP" → sidecars .xmp ao lado dos originais; abrir no
      Lightroom (se disponível) e conferir rating.

## TP-07 Pessoas (reconhecimento facial)
- [ ] TP-07.1 Botão "Pessoas" → Agrupar por pessoa (threshold 0.5) → grupos listados
      com capa e contagem. ⚠️ LENTO (59min/60 fotos — medir e registrar; se >10min,
      usar amostra menor e registrar no relatório).
- [ ] TP-07.2 Conferir 3 grupos aleatórios: as fotos do grupo mostram a mesma pessoa?
      Registrar taxa de acerto (X/3).
- [ ] TP-07.3 "Exportar por pessoa em pastas" → pastas Pessoa N com as fotos certas.

## TP-08 Galeria web
- [ ] TP-08.1 Selecionar 10 fotos → criar galeria (título "Teste") → pasta com
      index.html + photos/ + thumbs/.
- [ ] TP-08.2 Abrir index.html no navegador → dark theme, grid, lightbox funciona.

## TP-09 Álbuns e navegação
- [ ] TP-09.1 "← Meus Álbuns" volta à Lar; reabrir álbum mantém fotos.
- [ ] TP-09.2 Deletar álbum → fotos continuam no catálogo (outra sessão/álbum).
- [ ] TP-09.3 Abas IMPORT/CULL/EDIT/RETOUCH mudam o layout corretamente.

## TP-10 Idioma e robustez
- [ ] TP-10.1 Sistema em pt-BR → UI em português ("Selecionar", não "Cull");
      forçar en (`defaults write -g AppleLanguages (en)`) → UI em inglês.
- [ ] TP-10.2 Pasta vazia importada → mensagem amigável, sem crash.
- [ ] TP-10.3 Foto corrompida na pasta → aparece como "sem preview", import segue.
- [ ] TP-10.4 Reiniciar o app → álbum, ratings e presets persistem (SQLite).
- [ ] TP-10.5 Limpar cache → thumbs regeneram ao navegar.

## Relatório
Para cada caso: `TP-XX.Y: PASS|FAIL|BLOCKED — evidência — observação`.
Salvar em `docs/TESTES/rodadas/e2e-AAAA-MM-DD.md`.
