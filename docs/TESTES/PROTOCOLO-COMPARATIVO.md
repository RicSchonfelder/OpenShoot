# Protocolo Comparativo — AfterShoot × OpenShoot (usuário final)

> **Objetivo:** comparar os dois apps realizando AS MESMAS tarefas com AS MESMAS
> fotos, como um fotógrafo real, e registrar parâmetros (tempos, cliques, resultados).
> **Quem executa:** agente de teste (via Computer Use para o AfterShoot; UI real ou
> CLI/NAPI para o OpenShoot) ou humano.
> **Duração estimada:** 60-90 min por rodada completa.

## 0. Preparação (antes de começar)

1. **Fotos de teste:** 459 JPEGs em
   `~/Library/Mobile Documents/com~apple~CloudDocs/Desktop/2026-08-15 - Brotas/Editadas`
   (evento noturno, mix de retratos/grupos/paisagens).
   - ⚠️ Copie a pasta para o **disco local** (ex.: `~/Pictures/test-459`) ANTES de
     medir import — iCloud baixa on-demand e contamina a medição.
2. **AfterShoot:** app instalado em `/Applications/AfterShoot.app` (v2.21.4), álbum
   de teste existente ou criar novo.
3. **OpenShoot:** `cd ~/OpenShoot && npm run build:core && npx electron-vite preview`
   (ou app empacotado em /Applications/OpenShoot.app).
4. **Registrar em planilha/log:** para cada tarefa anote `tempo`, `cliques/passos`,
   `resultado esperado vs obtido`, `observações de UX`.

## 1. Importação

| Passo | AfterShoot | OpenShoot |
|---|---|---|
| 1.1 | Lar → Criar álbum → nomear | Lar → + Criar álbum → nomear |
| 1.2 | IMPORT → navegar até a pasta → selecionar | IMPORT → Importar pasta → escolher → opções (subpastas/tipo/sessão) → Iniciar |
| 1.3 | Cronometrar até o grid mostrar todas as fotos | idem |
| 1.4 | Verificar: contagem de fotos = 459; thumbnails visíveis | idem |

**Registrar:** tempo total, passos/cliques, contagem final, se RAW+JPEG misto funciona.

## 2. Culling

| Passo | AfterShoot | OpenShoot |
|---|---|---|
| 2.1 | CULL → "Iniciar seleção da IA" → aguardar | CULL → botão "Selecionar" (ou "Um clique") |
| 2.2 | Cronometrar o processamento (o app mostra ETA) | cronometrar via log/toast |
| 2.3 | Revisar: abrir loupe, navegar ←→, marcar P/X com teclado | idem (P/X/U, 1-5, Enter abre loupe, Esc fecha) |
| 2.4 | Filtrar: picks / duplicatas / sem classificação | filtros no painel lateral + dropdown Outros |
| 2.5 | Ajustar meta: "quantas fotos selecionar" | slider "Meta de seleção" + rodar cull de novo |

**Registrar:** tempo de culling, nº de picks da IA, se olhos fechados/blur foram
detectados, fluidez do loupe (fps subjetivo), atalhos funcionando.

## 3. Edição

| Passo | AfterShoot | OpenShoot |
|---|---|---|
| 3.1 | EDIT → escolher Perfil de IA → "Editar 459 Fotos" | EDIT → escolher preset → "Aplicar em lote" |
| 3.2 | Cronometrar a edição em lote | idem |
| 3.3 | Ajustes manuais: exposição, WB, contraste em 1 foto → comparar antes/depois | idem via sliders + preview |
| 3.4 | Curva de tom / HSL (Personalizar perfil) | sliders de Curva de tom / HSL |
| 3.5 | Horizonte IA / Recorte IA em 1 foto torta | botões "Ajustar horizonte (IA)" / "Recorte por IA" |

**Registrar:** tempo do lote, se o preview acompanha o slider em tempo real,
qualidade subjetiva do resultado (fotos noturnas escuras ficam boas?).

## 4. Retoque

| Passo | AfterShoot | OpenShoot |
|---|---|---|
| 4.1 | RETOUCH → selecionar foto com rosto → slider Acne 50% | RETOUCH → slider "Manchas (acne)" 50% |
| 4.2 | Slider Clarear olhos 40% | slider "Clarear olhos" 40% |
| 4.3 | PATCH: marcar uma distração e remover | loupe → arrastar sobre a distração → "Remover região" |
| 4.4 | "Colar" o retoque em outras fotos | "Aplicar retoque em lote (Colar)" → escolher pasta |
| 4.5 | Sujeito/fundo (desfocar fundo) | botão "Máscara de sujeito" |

**Registrar:** tempo por foto, realismo do resultado (sem manchas artificiais?),
se a moldura do rosto aparece, tempo do lote.

## 5. Exportação

| Passo | AfterShoot | OpenShoot |
|---|---|---|
| 5.1 | Selecionar 20 fotos (⌘A = todas) | selecionar 20 (⌘A) |
| 5.2 | Exportar → Destino → JPEG → Quality 90 → Exportar | Exportar → pasta → JPEG → qualidade 90 → Exportar |
| 5.3 | Cronometrar; abrir 3 fotos exportadas e comparar com o original editado | idem |
| 5.4 | Testar conflito de nome (exportar 2× na mesma pasta) | idem (esperado sufixo _1) |
| 5.5 | Testar naming com contador/data (OpenShoot: `{n}_{original}`) | — |

**Registrar:** tempo, resolução dos arquivos (deve ser nativa), se a edição foi
aplicada no arquivo final, nomes gerados.

## 6. Reconhecimento facial / pessoas

| Passo | AfterShoot | OpenShoot |
|---|---|---|
| 6.1 | (nativo no fluxo de culling: agrupa por rosto) | botão "Pessoas" → "Agrupar por pessoa" (threshold 0.5) |
| 6.2 | Verificar agrupamentos de ciclistas/pessoas | conferir grupos: mesma pessoa no mesmo grupo? |
| 6.3 | Exportar por pessoa | "Exportar por pessoa em pastas" → validar pastas Pessoa N |

**Registrar:** tempo (⚠️ OpenShoot: ~59 min/60 fotos — ver METRICAS-BASELINE G2),
precisão dos grupos (mesma pessoa agrupada corretamente?), falsos positivos.

## 7. Critérios de aprovação (por rodada)

- [ ] Nenhuma tarefa crasha; erros mostram mensagem amigável.
- [ ] Contagem de fotos consistente em todas as etapas (459).
- [ ] Export preserva resolução nativa e edição.
- [ ] Tempos dentro de 2× do AfterShoot (exceto gaps documentados G1-G3 em METRICAS-BASELINE).
- [ ] Fluxo completo executável sem consultar documentação (UX intuitiva).

## 8. Registro de rodada (preencher e salvar em docs/TESTES/rodadas/)

```
Rodada: #N · Data: ____ · Executor: ____
Import: ___s · Cull: ___s (picks ___) · Edit lote: ___s
Export 20: ___s · Face group 60: ___s · Retouch 5: ___s
Bugs encontrados: ___
UX notas: ___
Veredicto: ___
```
