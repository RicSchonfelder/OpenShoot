# OpenShoot 📸

[![CI](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml/badge.svg)](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml) [![Latest Release](https://img.shields.io/github/v/release/RicSchonfelder/OpenShoot)](https://github.com/RicSchonfelder/OpenShoot/releases/latest)

**Languages:** [English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md)

> **AI photo culling, editing and retouching for photographers — local-first and offline by default.**

OpenShoot is a desktop app for photographers. Its core photo analysis runs on the user's computer by default, with optional network features kept separate and opt-in. Originals are preserved and editing metadata can be written to XMP sidecars.

## Download

Choose your operating system from the latest GitHub Release:

| Operating system | Download |
|---|---|
| **Linux x86_64** | [AppImage](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-x86_64.AppImage) · [Debian/Ubuntu `.deb`](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-amd64.deb) |
| **Windows x64** | [Windows installer](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Windows-Setup-x64.exe) |
| **macOS Apple Silicon** | [macOS arm64 DMG](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-arm64.dmg) |
| **macOS Intel** | [macOS x64 DMG](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-x64.dmg) |

If a direct link is unavailable, open the [complete Release page](https://github.com/RicSchonfelder/OpenShoot/releases/latest) and choose the file whose name contains your operating system.

## Why OpenShoot?

| | |
|---|---|
| 🔒 **Local-first** | Core photo analysis runs on the computer and does not require a cloud account. Optional text features are separate and opt-in. |
| ⚡ **AI-powered culling** | Aesthetic scoring, face detection, sharpness and exposure analysis rate photos automatically. |
| ✍️ **Batch editing** | Presets, tone curve, HSL, exposure, white balance and contrast applied non-destructively. |
| 💄 **Retouching** | Skin smoothing, facial enhancement, background blur and selective masks. |
| 👤 **Face grouping** | Group photos by person using face embeddings. |
| 📁 **Albums & workflow** | IMPORT → CULL → EDIT → RETOUCH workflow for photo sessions. |
| 🔀 **Interoperable** | Lightroom/Capture One-compatible XMP sidecars and JPEG/PNG export. |
| ♻️ **Non-destructive** | Originals are not modified; ratings and edits are stored separately. |
| 🧩 **Open source** | MIT-licensed code with third-party models and licenses documented. |

## Features

### AI Culling
- **NIMA (MobileNet aesthetic)** predicts an aesthetic score per photo.
- **SCRFD** detects faces (multi-scale decode + NMS); photos containing sharp,
  well-exposed faces get boosted.
- Laplacian sharpness, exposure and histogram heuristics combined with ML scores.
- Automatic star ratings (★1–5), picks/rejects, duplicate grouping.
- One-click bulk XMP sidecar export for your rated picks.

### Batch Editing
- Non-destructive edit engine: exposure, white balance, contrast, saturation,
  shadows/highlights, brightness.
- Tone curve and HSL adjustments.
- Save/load presets and apply them across entire albums in one pass.

### Retouching
- Skin smoothing (YCbCr skin segmentation + selective blur).
- Facial retouching tools.
- Background blur with subject separation.
- Distraction removal via inpainting.

### Face Recognition Grouping
- MobileFaceNet embeddings group photos of the same person across a session —
  browse your shoot by people.

### Albums & Workflow Tabs
- Create albums, import folders (including subdirectories), and assign session
  types.
- Four workflow tabs guide you through **IMPORT → CULL → EDIT → RETOUCH**.

### Export & Interop
- **XMP sidecars** compatible with Adobe Lightroom Classic and Capture One
  (ratings, color labels, keywords).
- **JPEG/PNG export** of edited images.
- RAW support: NEF, ARW, DNG, CR2 and CR3 (embedded preview extraction).

## Architecture

```
┌─────────────────────┐        ┌──────────────────────────┐        ┌─────────────────────────┐
│  Electron + React   │ ⇄ IPC ⇄ │   Rust core (napi-rs)    │ ⇄ GPU ⇄ │ ONNX Runtime / CoreML   │
│  UI (src/)          │        │   catalog · AI · XMP     │        │ NIMA · SCRFD · FaceNet  │
└─────────────────────┘        │   (core/)                │        └─────────────────────────┘
                               └──────────────────────────┘
```

- **UI**: Electron + React (`src/`) — virtualized grid, loupe view, edit panels.
- **Core**: Rust compiled as a native addon via [napi-rs](https://napi.rs)
  (`core/`) — SQLite catalog, RAW decoding, ML inference, edit/retouch engines,
  XMP writer.
- **Inference**: [ort](https://github.com/pykeio/ort) with the CoreML execution
  provider (Apple Neural Engine / Metal), automatic CPU fallback.

See [docs/DESIGN.md](docs/DESIGN.md) for the full design document.

## Requirements

- **Linux** x86_64 (glibc ≥ 2.38 — Ubuntu 24.04+, Debian 13+, Fedora 40+) · **macOS** (Apple Silicon) · **Windows** 10+ (core compiles; installer experimental)
- **Rust ≥ 1.88**
- **Node.js ≥ 20**

Platform notes:

- **Inference**: CoreML on macOS (Apple Neural Engine / Metal); CPU ONNX Runtime on Linux/Windows (CUDA/DirectML planned as opt-in).
- **Paths**: caches and trash follow each platform's convention (`~/.cache`, XDG Trash on Linux; `~/Library/Caches`, `~/.Trash` on macOS).
- Linux build deps: `build-essential`, `pkg-config`, `libssl-dev`; runtime dep: `libgomp1`. Requires glibc ≥ 2.38 (Ubuntu 24.04+ / Debian 13+ / Fedora 40+).
- Windows SmartScreen: o instalador atual é distribuído sem certificado Authenticode; consulte [docs/WINDOWS-SMARTSCREEN.md](docs/WINDOWS-SMARTSCREEN.md) para configurar assinatura.
## Getting Started

```bash
git clone https://github.com/RicSchonfelder/OpenShoot.git
cd OpenShoot
npm install         # approve Electron/esbuild/fsevents install scripts if prompted
npm run build:core  # compiles the Rust core -> core/*.node
npm run dev         # launches the app
```

Other useful commands:

```bash
npm run typecheck   # TypeScript checks (main/preload + renderer)
npm test            # Rust tests (cargo)
npm run dist:mac    # build a packaged .app/.dmg into release/
npm run dist:linux  # build AppImage/.deb into release/
npm run dist:win    # build NSIS installer into release/
```

## ONNX Models

OpenShoot needs three models in `core/models/`:

| File | Purpose | License |
|---|---|---|
| `scrfd_2.5g_bnkps.onnx` (~3.1 MB) | Face detection | Apache-2.0 ([InsightFace SCRFD](https://github.com/deepinsight/insightface/tree/master/detection/scrfd)) |
| `nima_mobilenet_aesthetic.onnx` (~12.3 MB) | Aesthetic scoring | Google Research NIMA (non-commercial — see [THIRD_PARTY.md](THIRD_PARTY.md)) |
| `mobilefacenet.onnx` (~3.8 MB) | Face embeddings/grouping | See [THIRD_PARTY.md](THIRD_PARTY.md) |

Download links:

```bash
cd core/models
curl -L -o scrfd_2.5g_bnkps.onnx \
  https://huggingface.co/RuteNL/SCRFD-face-detection-ONNX/resolve/main/2.5g_bnkps.onnx
curl -L -o nima_mobilenet_aesthetic.onnx \
  https://huggingface.co/cromsc/nima-mobilenet-aesthetic/resolve/main/nima_mobilenet_aesthetic.onnx
# mobilefacenet.onnx: any standard MobileFaceNet ONNX export works
```

If `core/models/` already contains the `.onnx` files (e.g. after a fresh clone),
you're ready to go.

## Roadmap

- [x] Phase 0 — Electron + React skeleton, napi-rs IPC bridge
- [x] Phase 1 — SQLite catalog, RAW decode, thumbnails, virtualized grid
- [x] Phase 2 — AI culling (NIMA + SCRFD), ratings, filters, bulk XMP export
- [x] Phase 3 — Batch editing engine, presets, tone curve/HSL
- [x] Phase 4 — Retouching (skin smoothing, inpainting, background blur)
- [x] Albums, workflow tabs (IMPORT/CULL/EDIT/RETOUCH), face recognition grouping
- [ ] CR3 dimensions from BMFF header for photos without EXIF
- [ ] Opt-in text features (keywords/captions via user-provided API key)
- [x] Linux support (CPU inference, XDG paths/trash, AppImage/deb targets)
- [x] Windows installer (x64) and native Recycle Bin support
- [x] macOS installers (Apple Silicon and Intel)
- [x] Multilingual README links (English, Português, Español, 简体中文)

## Contributing

Contributions welcome! Read [CONTRIBUTING.md](CONTRIBUTING.md), pick a
[`good first issue`](https://github.com/RicSchonfelder/OpenShoot/issues?q=is%3Aissue+label%3A%22good+first+issue%22),
and open a PR.

## License

MIT — see [LICENSE](LICENSE). Third-party components and models are listed in
[THIRD_PARTY.md](THIRD_PARTY.md).
