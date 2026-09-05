# Rodada de validação — olhos fechados no culling

**Data:** 2026-09-04  
**Escopo:** verificar compilação, carregamento do addon Linux e disponibilidade de fixture para validação facial.

## Resultado

| Verificação | Resultado | Evidência |
|---|---:|---|
| `npm run typecheck` | PASS | TypeScript main/preload/renderer sem erros |
| `cargo check --manifest-path core/Cargo.toml --tests` | PASS | 0 erros; 5 warnings preexistentes em `edit.rs`/`grouping.rs` |
| `npm run test:codeformer` | PASS | 16/16 testes |
| `git diff --check` | PASS | nenhum erro de whitespace |
| `npm run smoke:core` | BLOCKED | addon não carrega: símbolo ausente `_M_replace_cold` |
| Fixture facial | BLOCKED | `core/fixtures/test.jpg` é um gradiente azul, sem pessoa/rosto |

## Bloqueio técnico

O addon `core/openshoot_core.linux.x64.node` e o link de testes dependentes do ORT pré-compilado exigem símbolos de `libstdc++`/glibc não disponíveis neste host. Erro observado ao carregar o addon:

```text
undefined symbol: _ZNSt7__cxx1112basic_stringIcSt11char_traitsIcESaIcEE15_M_replace_coldEPcmPKcmm
```

Isso impede executar o culling NAPI e medir `eyes_score` com uma foto real nesta máquina. Não é evidência de falha do algoritmo; é uma limitação do artefato/runtime local. A alternativa documentada é compilar/usar ORT dinâmico compatível via `ort-load-dynamic`, ou validar em Ubuntu 24.04/GitHub Actions.

## Próxima evidência necessária

1. Obter um pequeno conjunto autorizado de fotos locais com rostos e casos de olhos abertos/fechados.
2. Recompilar o core em runtime compatível.
3. Executar culling real em diretório temporário.
4. Comparar visualmente as fotos filtradas e registrar falsos positivos/negativos.
5. Ajustar o limiar `0.40` somente após essa amostra.

Nenhuma foto de produção foi alterada nesta rodada.
