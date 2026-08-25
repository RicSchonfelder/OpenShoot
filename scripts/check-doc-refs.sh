#!/usr/bin/env bash
# Checagem de higiene documental: lista referências a caminhos do repositório
# (em `backticks`) que não existem no repo.
#
# Uso:  bash scripts/check-doc-refs.sh
# Saída: linhas "MISSING (<arquivo>): <caminho>" — vazia = nenhuma quebra.
#
# Escopo deliberado: apenas referências que parecem caminhos reais do repo
# (docs/, core/, src/, scripts/, build/, .github/ + extensão de arquivo,
# sem globs). Ficam fora de propósito: paths com ~ ou absolutos, âncoras
# (#/rota), globs (*), snippets de código e shorthands sem "/".
set -uo pipefail
cd "$(dirname "$0")/.."

files=(README.md CONTRIBUTING.md AGENTS.md THIRD_PARTY.md docs/*.md .github/workflows/*.yml)

missing=0
for f in "${files[@]}"; do
  [ -f "$f" ] || continue
  while IFS= read -r ref; do
    case "$ref" in
      *"*"*|*"://"*) continue ;;
      *"<"*) continue ;;                       # placeholders: core/openshoot_core.<platform>...
    esac
    ref="${ref%% §*}"                          # corta apontadores de seção: "...md §3"
    ref="${ref%:[0-9]*}"                       # corta sufixo de linha: "lib.rs:1124"
    case "$ref" in
      core/openshoot_core.*.node) continue ;;  # artefato local gitignored (regra 1 do AGENTS.md)
      docs/AUDITORIA-referência\ externa.md) continue ;;
      # ↑ nome antigo citado APENAS em notas de correção rastreável (2026-08-25,
      #   docs/PROGRESSO.md e docs/PARIDADE-FUNCIONAL.md); substituído por
      #   docs/AUDITORIA-FUNCIONAL.md. Se voltar a aparecer fora dessas notas, é quebra real.
    esac
    case "$ref" in
      docs/*|core/*|src/*|scripts/*|build/*|.github/*)
        if [ ! -e "$ref" ]; then
          echo "MISSING ($f): $ref"
          missing=$((missing + 1))
        fi
        ;;
    esac
  done < <(
    LC_ALL=C grep -oE '`[^`]*`' "$f" \
      | sed 's/^`//; s/`$//' \
      | LC_ALL=C grep '/' \
      | sort -u
  )
done

echo "---"
if [ "$missing" -eq 0 ]; then
  echo "OK: nenhuma referência a caminho inexistente."
else
  echo "$missing referência(s) inexistente(s)."
fi
exit "$missing"
