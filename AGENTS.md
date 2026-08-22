# OpenShoot — guia para agentes de IA trabalhando neste repo

## Contexto

Aplicativo desktop open-source para fotógrafos: culling/edição/retoque de fotos
com IA local. Inspirado em uma referência externa: UI Electron/React + core Rust (napi-rs) +
ONNX Runtime. Repo público: https://github.com/RicSchonfelder/OpenShoot

## Pilares invioláveis

- **100% local/offline**: pixels nunca saem da máquina. IA de visão roda via ONNX
  na GPU (Metal no macOS). Nenhum upload de imagem.
- **Open source (MIT)**: todo código público; zero pesos/segredos proprietários.
- **Chaves externas pertencem ao usuário**: serviços externos (ex: OpenRouter,
  Fase 6) são opt-in, chave do usuário no Keychain, NUNCA hardcoded.
- **Não-destrutivo**: nada sobrescreve os arquivos originais; edições/ratings vão
  para XMP sidecars.

## Arquitetura

UI (Electron/React, `src/`) ⇄ IPC (`electron-vite`, `src/main`) ⇄ core Rust
(`core/`, crate `openshoot-core`) via napi-rs. O addon é compilado para
`core/openshoot_core.<platform>.<arch>.node` por `scripts/build-core.mjs`.

## Comandos

```bash
npm install           # instala deps (aprovar scripts do Electron: electron, esbuild, fsevents)
npm run build:core    # cargo build --release + copia para core/*.node
npm run dev           # roda o app em desenvolvimento
npm run typecheck     # tsc para main/preload e renderer (AMBOS devem passar)
npm test              # cargo test no core
```

## Regras para agentes

1. **Nunca commitar**: `core/*.node` (binário local), `node_modules/`, `out/`,
   `target/`, `.env*` (exceto `.env.example`).
2. **Sempre rodar** `npm run typecheck` e `npm test` antes de declarar uma tarefa
   concluída.
3. **Manter o DESIGN.md (`docs/DESIGN.md`) atualizado**: mudança de arquitetura ou
   feature nova DEVE refletir no design document antes/simultaneamente ao código.
4. **Novo código Rust** exige testes (`#[cfg(test)]`) e novo código TS exige
   typecheck limpo.
5. **CSP**: não relaxar o Content-Security-Policy do renderer sem justificativa.
6. **Privacidade**: qualquer envio de dados para rede exige: feature flag ON por
   padrão OFF, aviso claro ao usuário, e nada automático em silêncio.

## Estado atual

Fase 0 concluída (esqueleto + ponte IPC). Próximas: Fase 1 (catálogo + RAW decode
+ thumbnails) e Fase 2 (culling com IA local). Ver roadmap em docs/DESIGN.md.
