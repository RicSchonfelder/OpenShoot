# OpenShoot 📸

[![CI](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml/badge.svg)](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml) [![Última versão](https://img.shields.io/github/v/release/RicSchonfelder/OpenShoot)](https://github.com/RicSchonfelder/OpenShoot/releases/latest)

**Idiomas:** [English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md)

> **Seleção, edição e retoque de fotos com IA — priorizando processamento local e offline por padrão.**

O OpenShoot é um aplicativo desktop para fotógrafos. A análise principal das fotos roda no computador do usuário por padrão. Recursos de rede opcionais ficam separados e dependem de ativação explícita. Os originais são preservados e os metadados podem ser gravados em sidecars XMP.

## Download

Escolha seu sistema operacional na versão mais recente:

| Sistema operacional | Download |
|---|---|
| **Linux x86_64** | [AppImage](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-x86_64.AppImage) · [Debian/Ubuntu `.deb`](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-amd64.deb) |
| **Windows x64** | [Instalador do Windows](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Windows-Setup-x64.exe) |
| **macOS Apple Silicon** | [DMG macOS arm64](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-arm64.dmg) |
| **macOS Intel** | [DMG macOS x64](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-x64.dmg) |

Se um link direto não funcionar, abra a [página completa da Release](https://github.com/RicSchonfelder/OpenShoot/releases/latest) e escolha o arquivo que contém seu sistema no nome.

## Principais recursos

- Culling com pontuação estética, detecção facial, nitidez e exposição.
- Aviso de olhos fechados no culling, grid e loupe.
- Edição em lote não destrutiva: exposição, balanço de branco, contraste, HSL e curvas.
- Retoque de pele, melhoria facial, desfoque de fundo e máscaras seletivas.
- Agrupamento de fotos por pessoa.
- Álbuns e fluxo IMPORTAR → CULL → EDITAR → RETOQUE.
- Sidecars XMP compatíveis com Lightroom Classic e Capture One.
- Exportação JPEG/PNG e suporte a prévias de RAW NEF, ARW, DNG, CR2 e CR3.

## Requisitos

- Linux x86_64 (glibc ≥ 2.38), Windows 10+ ou macOS arm64/x64.
- Para desenvolvimento: Rust ≥ 1.88 e Node.js ≥ 20.
- Os modelos ONNX ficam em `core/models/` ou no diretório indicado por `OPENSHOOT_MODELS_DIR`.

## Desenvolvimento

```bash
git clone https://github.com/RicSchonfelder/OpenShoot.git
cd OpenShoot
npm install
npm run build:core
npm run dev
```

Consulte [CONTRIBUTING.md](CONTRIBUTING.md), [docs/DESIGN.md](docs/DESIGN.md) e [THIRD_PARTY.md](THIRD_PARTY.md).

## Licença

MIT. Componentes e modelos de terceiros estão documentados em [THIRD_PARTY.md](THIRD_PARTY.md).
