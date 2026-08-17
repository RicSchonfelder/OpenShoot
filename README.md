# OpenShoot 📸

> Open-source AI culling, editing & retouching for photographers.

Aplicativo desktop para fotógrafos que automatiza o pós-processamento de fotos —
**100% local e offline**. Arquitetura inspirada no AfterShoot (UI Electron/React +
core Rust + ONNX Runtime na GPU), mas **totalmente aberto** (MIT): o usuário tem
acesso a todo o código-fonte.

## Pilares

| Pilar | Compromisso |
|---|---|
| 🔒 **100% local** | Imagens nunca saem da sua máquina. Processamento na sua GPU (ONNX/Metal). |
| 🧩 **Open source** | MIT. Todo o código é público e auditável. |
| 🔑 **Chaves são suas** | Qualquer serviço externo (ex: OpenRouter, Fase 6) é opt-in e usa a SUA chave, guardada no Keychain. |
| ♻️ **Não-destrutivo** | Nunca altera o original. Edições/ratings vão para XMP sidecars. |

## Roadmap

- **Fase 0 — Esqueleto** ✅ `Electron (React) ⇄ Rust (napi-rs) via IPC`
- **Fase 1 — Catálogo & RAW** Importação, SQLite, decode RAW + previews
- **Fase 2 — Culling (IA)** Facas, nitidez, score, duplicatas, XMP
- **Fase 3 — Edição em lote** Presets + "aprender" estilo
- **Fase 4 — Retoque** Pele, remoção de distrações, fundo
- **Fase 5 — Export** JPEG/TIFF + XMP p/ Lightroom/Capture One/Photoshop
- **Fase 6 — Texto (opt-in)** OpenRouter p/ keywords/descrições (chave do usuário)

## Desenvolvimento

Pré-requisitos: Node ≥ 18, Rust ≥ 1.70, macOS (arm64/x86_64).

```bash
npm install        # instala dependencias (pode exigir aprovar scripts do Electron)
npm run build:core # compila core Rust -> core/*.node
npm run dev        # roda o app
```

Comandos de validação:

```bash
npm run typecheck  # TypeScript (main/preload + renderer)
npm test           # testes Rust (cargo)
```

## Arquitetura

```
UI (Electron/React)  ⇄  [IPC via napi-rs]  ⇄  core Rust (ONNX na GPU)
```

Veja [docs/DESIGN.md](docs/DESIGN.md) para o design completo.

## Contribuindo

Leia [CONTRIBUTING.md](CONTRIBUTING.md). Toda ajuda é bem-vinda — problemas,
ideias e PRs.

## Licença

MIT — veja [LICENSE](LICENSE).
