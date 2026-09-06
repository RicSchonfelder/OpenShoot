# OpenShoot 📸

[![CI](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml/badge.svg)](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml) [![Última versión](https://img.shields.io/github/v/release/RicSchonfelder/OpenShoot)](https://github.com/RicSchonfelder/OpenShoot/releases/latest)

**Idiomas:** [English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md)

> **Selección, edición y retoque fotográfico con IA — procesamiento local prioritario y sin conexión por defecto.**

OpenShoot es una aplicación de escritorio para fotógrafos. El análisis principal de las fotos se ejecuta en el ordenador del usuario por defecto. Las funciones de red opcionales están separadas y requieren activación explícita. Los originales se conservan y los metadatos pueden escribirse en sidecars XMP.

## Descargar

Elige tu sistema operativo en la versión más reciente:

| Sistema operativo | Descarga |
|---|---|
| **Linux x86_64** | [AppImage](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-x86_64.AppImage) · [Debian/Ubuntu `.deb`](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-amd64.deb) |
| **Windows x64** | [Instalador de Windows](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Windows-Setup-x64.exe) |
| **macOS Apple Silicon** | [DMG macOS arm64](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-arm64.dmg) |
| **macOS Intel** | [DMG macOS x64](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-x64.dmg) |

Si un enlace directo no funciona, abre la [página completa de la Release](https://github.com/RicSchonfelder/OpenShoot/releases/latest) y elige el archivo que incluya tu sistema operativo en el nombre.

## Funciones principales

- Culling con puntuación estética, detección facial, nitidez y exposición.
- Aviso de ojos cerrados en culling, grid y loupe.
- Edición por lotes no destructiva: exposición, balance de blancos, contraste, HSL y curvas.
- Retoque de piel, mejora facial, desenfoque de fondo y máscaras selectivas.
- Agrupación de fotos por persona.
- Álbumes y flujo IMPORTAR → CULL → EDITAR → RETOQUE.
- Sidecars XMP compatibles con Lightroom Classic y Capture One.
- Exportación JPEG/PNG y soporte para previsualizaciones RAW NEF, ARW, DNG, CR2 y CR3.

## Requisitos

- Linux x86_64 (glibc ≥ 2.38), Windows 10+ o macOS arm64/x64.
- Para desarrollo: Rust ≥ 1.88 y Node.js ≥ 20.
- Los modelos ONNX deben estar en `core/models/` o en el directorio indicado por `OPENSHOOT_MODELS_DIR`.

## Desarrollo

```bash
git clone https://github.com/RicSchonfelder/OpenShoot.git
cd OpenShoot
npm install
npm run build:core
npm run dev
```

Consulta [CONTRIBUTING.md](CONTRIBUTING.md), [docs/DESIGN.md](docs/DESIGN.md) y [THIRD_PARTY.md](THIRD_PARTY.md).

## Licencia

MIT. Los componentes y modelos de terceros están documentados en [THIRD_PARTY.md](THIRD_PARTY.md).
