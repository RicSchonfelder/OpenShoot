# CHECKPOINT — Higiene documental (2026-08-25)

> Sessão: reconciliação de documentação sem apagar histórico.
> Branch: `agent/docs-hygiene` · Commit: `docs: reconcile project documentation` · **Sem push.**
> Nenhum arquivo de código (`src/`, `core/`, `scripts/` de build) foi alterado.

## O que foi feito

### 1. `docs/MULTIPLATAFORMA.md` criado (referência quebrada em 5 docs)
O arquivo era citado por `AGENTS.md`, `docs/ROADMAP.md`, `docs/AUDITORIA-LINUX.md`,
`docs/SESSAO.md` e `docs/PROGRESSO.md`, mas não existia — o documento original do
agente Windows (commit `9776371`) saiu do histórico local no re-baselinamento
(`2752c48`). A nova nota consolida **somente evidências presentes no repo**
(`core/Cargo.toml`, `electron-builder.yml`, `package.json`,
`.github/workflows/ci.yml`, `core/src/ml.rs`, `core/src/lib.rs`,
`src/main/index.ts`) e marca explicitamente o **não executado**:
validações Windows (evidência apenas textual), ARM Linux (`aarch64`), CUDA
opt-in, release workflow, notarização/auto-update macOS, falha intermitente do
CI Windows.

### 2. Referências a `docs/AUDITORIA-referência externa.md` → `docs/AUDITORIA-FUNCIONAL.md`
Corrigido em `docs/PROGRESSO.md` (§ Auditoria) e `docs/PARIDADE-FUNCIONAL.md`
(cabeçalho), com nota inline datada preservando o nome antigo (correção
rastreável). Semântica conferida: ambos os trechos descrevem "mapa completo +
gap analysis com 15 itens priorizados" = §6 do AUDITORIA-FUNCIONAL.md.

### 3. Banner de snapshot histórico em `docs/SESSAO.md`
Apenas adição de banner; nenhum fato reescrito. Aponta para fontes correntes
(ROADMAP/PROGRESSO/MULTIPLATAFORMA/AGENTS) e avisa que caminhos/commits citados
(`~/OpenShoot`, `229d8b6`–`b025000`, `9776371`) são anteriores ao re-baselinamento.

### 4. `docs/ROADMAP.md` alinhado só no item obviamente implementado
- `[ ]` → `[x]`: "`npm run dist:win` / `dist:linux` scripts" — evidência direta
  em `package.json` (linhas 19–20). Nota inline registra que a *execução*
  validada até agora é só Linux (`AUDITORIA-LINUX.md §6`).
- Nenhum outro item foi marcado (P2 olhos fechados, aarch64, release workflow,
  clippy `-D warnings` etc. permanecem abertos conforme evidência).

### 5. Checagem automática de referências — `scripts/check-doc-refs.sh`
Varre backtick-refs com `/` em README/CONTRIBUTING/AGENTS/THIRD_PARTY/docs/CI.
- **Antes:** 7 quebras reais (5× MULTIPLATAFORMA, 2× AUDITORIA-referência externa)
  após filtrar ruído intencional (globs, paths `~`, âncoras `#/rota`,
  placeholders `<...>`, artefato gitignored `core/*.node`, sufixos `:linha`).
- **Depois:** `OK: nenhuma referência a caminho inexistente.` (exit 0).
- Exclusões justificadas em comentários no próprio script.

## Não feito (fora de escopo / decisão consciente)
- Nenhuma reescrita de fatos históricos (SESSAO/AUDITORIA-LINUX preservados).
- `AGENTS.md` intocado — voltou a ficar consistente com a criação de
  MULTIPLATAFORMA.md.
- Shorthands contextuais sem `/` (ex.: `catalog.rs` dentro de "core/src/") não
  são quebras reais; arquivos existem (`core/src/catalog.rs`,
  `src/renderer/src/components/Gallery.tsx` etc.).

## Validação
```bash
bash scripts/check-doc-refs.sh   # → OK (exit 0)
```
Mudanças restritas a: `docs/*.md` (4 modificados, 1 novo), `scripts/check-doc-refs.sh`
(novo), `CHECKPOINT.md` (novo).
