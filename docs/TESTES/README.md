# Testes OpenShoot

Diretório de testes para agentes e humanos.

| Arquivo | Uso |
|---|---|
| `METRICAS-BASELINE.md` | Números medidos no macOS (benchmark automatizado) + gaps G1-G5 vs referência |
| `PROTOCOLO-COMPARATIVO.md` | Protocolo passo-a-passo: mesmas tarefas nos dois apps, como usuário final |
| `TESTPLAN-UI.md` | Roteiros E2E por funcionalidade (TP-01..TP-10) com critérios de PASS/FAIL |
| `CHECKLIST-PLATAFORMAS.md` | Validação Windows/Linux após portabilidade (PORT-01..07) |
| `rodadas/` | Registros de cada rodada de teste (template no fim do protocolo) |

## Ordem de execução sugerida para um agente

1. Ler `METRICAS-BASELINE.md` (contexto e gaps conhecidos).
2. Rodar `TESTPLAN-UI.md` completo (TP-01..TP-10) — registrar PASS/FAIL.
3. Rodar `PROTOCOLO-COMPARATIVO.md` (requer AfterShoot instalado + Computer Use).
4. Após portabilidade (ROADMAP P1): `CHECKLIST-PLATAFORMAS.md` por plataforma.
5. Salvar relatórios em `rodadas/` e NÃO commitar (reportar ao orquestrador).

## Comandos rápidos

```bash
cd ~/OpenShoot
npm run build:core && npm run typecheck   # sanidade antes de testar
npx electron-vite preview                  # abrir o app
cargo test --manifest-path core/Cargo.toml # testes de core (60)
```
