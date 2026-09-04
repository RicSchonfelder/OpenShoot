# CodeFormer local (opt-in) — Status da implementação

> **Worktree:** `feat/codeformer-openshot` · **Data:** 2026-09-04
> Este é o **status local** do repositório (registro honesto de sessão; fontes
> canônicas de arquitetura: `docs/DESIGN.md` e `docs/CODEFORMER.md`).

## Estado: implementação completa e validada (sem pesos/rede/GPU)

Integração opt-in da restauração de rostos CodeFormer via **ponte CLI local do
usuário**, 100% offline, sem distribuição de pesos/binários/segredos.

## Escopo entregue

| Item | Estado | Local |
|---|---|---|
| Serviço main isolado (settings opt-in, status acionável, runner com timeout) | ✅ | `src/main/codeformer.ts` |
| Tipos compartilhados strict (sem `any`) | ✅ | `src/types/codeformer.ts` |
| IPC (`get/saveCodeFormerSettings`, `getCodeFormerStatus`, `codeFormerRestore`) | ✅ | `src/main/index.ts`, `src/preload/index.ts` |
| Seção opt-in recolhida na bancada de restauração | ✅ | `src/renderer/src/components/RestorerView.tsx` |
| Documentação (contrato CLI v1, licença S-Lab, limitações) | ✅ | `docs/CODEFORMER.md`, `docs/DESIGN.md`, `docs/ROADMAP.md` (P9) |
| Testes determinísticos (ponte simulada por Node; sem pesos/GPU/rede) | ✅ | `tests/codeformer/`, script `npm run test:codeformer` |

## Validação executada nesta sessão

- `npm run typecheck` — **passa** (node + web).
- `npm run test:codeformer` — **16/16 passam** (compilação TS + `node --test`).
- `npm test` (cargo test no core) — **falha no linking, pré-existente e
  alheio a esta fatia**: `rust-lld: undefined symbol __isoc23_strtoll /
  __isoc23_strtoull / std::…wchar_t…_M_replace_cold` ao linkar `ort_sys`
  (ONNX Runtime) — incompatibilidade de libstdc++/glibc do ambiente com os
  objetos do `ort-sys` compilados no `target/`. **Nenhum arquivo Rust foi
  alterado** por esta fatia (o diff não toca `core/`), então o erro não é
  causado por ela. Recompilar `ort-sys` do zero ou usar um container com
  libstdc++ compatível resolve; fora do escopo aqui.
- `git diff --check` — limpo (sem conflitos de whitespace).

## Segurança / privacidade (pilares respeitados)

- Opt-in **OFF por padrão** (`enabled: false`); nada roda sem ativação.
- Subprocesso **sem shell** (argv direto), `HF_HUB_OFFLINE=1` +
  `TRANSFORMERS_OFFLINE=1` propagados; o app não faz rede para esta fatia.
- **Zero pesos/binários/segredos** commitados; `.gitignore` cobre
  `tests/codeformer/out/` (artefatos compilados de teste).
- Job em diretório temporário `0700` sob `userData/codeformer-jobs`, validado
  (exatamente 1 JPEG/PNG por magic bytes) e removido; originais intocados.
- Settings gravados atomicamente (tmp + rename, modo `0600`).

## Pendências fora do escopo desta fatia

- [ ] Validação E2E com pesos reais + ponte real (requer download manual do
  usuário; licença NTU S-Lab do upstream restringe redistribuição).
- [ ] Inferência nativa ONNX no core Rust (futura; exige pesos para validar).
- [ ] Reavaliar `npm test` em ambiente com toolchain/libstdc++ compatível.
