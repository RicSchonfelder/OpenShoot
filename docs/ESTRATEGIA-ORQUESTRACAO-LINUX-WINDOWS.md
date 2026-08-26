# Estratégia de trabalho — OpenShoot

**Estado:** plano operacional verificável  
**Cópia operacional Linux:** `/home/schon/Programas/OpenShoot`  
**Branch de integração:** `main`  
**Último commit integrado:** `fbd0c3a6401a207d64f734786b2973df0b25808a`

## Objetivo

Concluir o OpenShoot com alterações aplicadas na aplicação, mantendo rastreabilidade por branch/worktree, testes independentes e validação final no Windows. O macOS ficará para a etapa presencial de comparação visual com AfterShoot.

## Estado atual

- Linux local final 102: disponível.
- Windows final 94: rede SSH responde, mas autenticação ainda falha (`Permission denied`).
- macOS final 70: sem rota SSH no momento.
- A cópia em `Programas/OpenShoot` está limpa e corresponde ao commit integrado.
- `npm run typecheck`: PASS na origem integrada.
- Validação JSON de i18n: PASS.
- `npm test`: bloqueado no link do ONNX Runtime contra a glibc/libstdc++ do host Linux; não é considerado teste aprovado.

## Fases

### Fase 0 — preflight obrigatório

Antes de qualquer agente remoto:

1. Confirmar SSH autenticado no Windows com `hostname`, usuário, versão do OpenCode e caminho do projeto.
2. Confirmar que `C:\Users\ricar\Programas\OpenShoot` existe ou criar uma pasta nova sem sobrescrever conteúdo.
3. Inventariar branch, commit, arquivos modificados e dependências no Windows.
4. Confirmar `opencode --version`, Node/npm e Rust/cargo quando aplicável.
5. Registrar o resultado como `reachable+authenticated`, `reachable+unauthenticated` ou `unreachable`.

Sem essa fase, não declarar cópia, agente ou teste Windows como executado.

### Fase 1 — cinco lanes no Linux

Cada lane usa worktree próprio, branch própria e commit obrigatório:

1. **core/import:** progresso IPC, importação, filtros e testes unitários focados.
2. **core/culling:** olhos fechados, score facial e escopo por álbum.
3. **ui/export:** galeria web, exportação, seleção e i18n.
4. **platform/build:** ONNX Runtime, addons nativos, empacotamento e CI.
5. **qa/docs:** matriz de aceitação, documentação, smoke tests e evidências.

Regras:

- nenhum agente trabalha diretamente na `main`;
- arquivos fora do escopo são proibidos;
- cada agente deve retornar commit, lista de arquivos e comandos executados;
- Hermes revisa o diff e repete os testes antes do cherry-pick;
- falha de ambiente não pode ser relatada como falha funcional nem como sucesso.

### Fase 2 — integração Linux

Gates antes de integrar:

- `git diff --check`;
- `npm run typecheck`;
- validação de JSON/YAML;
- testes Rust focados;
- `npm test` ou falha reproduzível documentada;
- smoke test do core;
- revisão de alterações no lockfile;
- ausência de credenciais e artefatos pessoais no commit.

Depois da integração, a cópia operacional deve ser atualizada somente por fast-forward ou clone verificado. Nunca usar `reset --hard`, `git clean` ou sobrescrita ampla para “sincronizar”.

### Fase 3 — cópia e execução Windows

Com SSH autenticado:

1. Criar backup versionado da instalação existente, se houver.
2. Copiar a `main` integrada para uma pasta separada no Windows.
3. Executar `npm ci` com Node compatível com os engines declarados.
4. Executar build do addon nativo para Windows x64.
5. Executar `npm run typecheck` e testes focados.
6. Empacotar o instalador Windows somente após o addon carregar.
7. Abrir a aplicação no desktop Windows e executar o smoke test humano.
8. Registrar caminho do instalador, hash, versão, commit e resultado de cada teste.

O teste Windows final deve cobrir: importar pasta, progresso, filtros all/picks/rejects/unrated, seleção, culling, criação de galeria web, exportação e reabertura da aplicação.

### Fase 4 — macOS presencial

Quando Ricardo retornar:

- iniciar AfterShoot e OpenShoot no Mac;
- executar o mesmo roteiro humano com o mesmo conjunto de fotos;
- capturar diferenças observáveis, erros e tempos sem presumir equivalência;
- validar macOS arm64 e, se necessário, x64 separadamente;
- testar o addon nativo e a galeria/exportação;
- anexar evidências ao relatório consolidado.

## Protocolo de sincronização

A fonte de verdade desta etapa é a branch `main` local em `/home/schon/Programas/OpenShoot`. Agentes remotos só devem trabalhar em clones/worktrees derivados do commit explicitamente informado. Nenhum agente deve fazer push para produção ou sobrescrever a pasta do usuário.

Formato de checkpoint:

```text
host=<linux|windows|macos>
agent=<lane>
branch=<branch>
base_commit=<hash>
commit=<hash ou NONE>
changed_files=<lista>
tests=<comando:resultado>
status=<implemented|verified|partial|blocked>
blocker=<descrição>
```

## Critério de conclusão

O OpenShoot só será considerado concluído quando:

- as correções integradas tiverem testes independentes aprovados;
- o build do addon passar no Linux e Windows;
- o instalador Windows abrir e executar o roteiro humano;
- a aplicação funcionar sem regressão nos fluxos de importação, culling, filtros e exportação;
- o Mac permitir comparação real com AfterShoot;
- cada resultado tiver commit, comando, artefato ou evidência correspondente.

Até esses gates, o estado correto é **em implementação/verificação**, não “finalizado”.
