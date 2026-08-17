# Contribuindo com o OpenShoot

Obrigado por querer ajudar! 🚀

## Como começar

1. **Rode o projeto** seguindo o `README.md`.
2. **Procure por issues** com a label `good first issue` no
   [GitHub Issues](https://github.com/RicSchonfelder/OpenShoot/issues).
3. **Converse primeiro**: qualquer mudança grande (nova feature, mudança de
   arquitetura) deve ser discutida em uma issue ou no DESIGN.md antes do PR.

## Padrões de código

- **Commit atômico**: um commit por tarefa lógica. Mensagem clara em inglês.
- **Testes**: código novo em Rust deve ter `cargo test`. Correção de bug vem
   acompanhada de teste que reproduz o bug.
- **TypeScript**: `npm run typecheck` deve passar. Sem `any` desnecessário.
- **Privacidade**: NUNCA envie imagem para serviço externo sem opt-in explícito
  do usuário naquela sessão. Chaves de API jamais no código — sempre no Keychain.
- **Não-destrutivo**: nenhum código pode sobrescrever os arquivos originais do
  usuário. Edições vão para sidecars (XMP).

## Fluxo do PR

1. Fork + branch `feat/...` ou `fix/...`.
2. Faça o commit com mensagem descritiva (`feat: add RAW thumbnail pipeline`).
3. Abra o PR contra a `main`, descrevendo o que mudou e como testar.
4. CI roda typecheck + testes. PR só merge com CI verde e review.

## Estrutura

```
docs/DESIGN.md   Fonte da verdade da arquitetura e roadmap
src/main/        Processo principal do Electron (IPC + core)
src/preload/     Ponte segura (contextBridge)
src/renderer/    UI React
core/            core Rust (napi-rs) — decode RAW, IA, XMP
```

Dúvidas? Abra uma issue — respondemos rápido.
