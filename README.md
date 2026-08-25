# OpenShoot 📸

[![CI](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml/badge.svg)](https://github.com/RicSchonfelder/OpenShoot/blob/main/.github/workflows/ci.yml)

> **Open-source AI photo culling, editing & retouching — 100% local & offline**

OpenShoot is a desktop app for photographers that automates post-processing with
AI models running **entirely on your machine**. No cloud. No uploads. No
subscriptions. Your pixels never leave your computer.

## Why OpenShoot?

| | |
|---|---|
| 🔒 **100% local & offline** | All AI inference runs via ONNX Runtime (CoreML/Metal on macOS). Images are never uploaded anywhere. |
| ⚡ **AI-powered culling** | Aesthetic scoring (NIMA) + face detection (SCRFD) + sharpness analysis rate every photo automatically. |
| ✍️ **Batch editing** | Presets, tone curve, HSL, exposure/WB/contrast — non-destructive, applied to whole sessions. |
| 💄 **Retouching** | Skin smoothing, facial enhancement and background blur with selective masks. |
| 👤 **Face recognition grouping** | Cluster photos by person using MobileFaceNet embeddings. |
| 📁 **Albums & workflow** | Organize shoots into albums and work through an IMPORT → CULL → EDIT → RETOUCH pipeline. |
| 🔀 **Interoperable** | Writes Lightroom/Capture One-compatible XMP sidecars; exports JPEG/PNG. |
| ♻️ **Non-destructive** | Originals are never modified — ratings and edits live in sidecar files. |
| 🧩 **Truly open source** | MIT licensed, fully auditable code. No proprietary weights, no black boxes. |

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

- **Linux** x86_64 (glibc ≥ 2.38 — Ubuntu 24.04+, Debian 13+, Fedora 40+) · **macOS** (Apple Silicon) · **Windows** 10+
- **Rust ≥ 1.88**
- **Node.js ≥ 20**

Platform notes:

- **Inference**: CoreML on macOS (Apple Neural Engine / Metal); DirectML on Windows; CPU ONNX Runtime on Linux (CUDA planned as opt-in).
- **Paths**: caches follow each platform's convention (`~/.cache` on Linux, `~/Library/Caches` on macOS); trash is native on all platforms (crate `trash`).
- Linux build deps: `build-essential`, `pkg-config`, `libssl-dev`; runtime dep: `libgomp1`.

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
- [x] Linux support (CPU inference, XDG paths, AppImage/deb targets)
- [x] Windows support (DirectML inference, native Recycle Bin, NSIS installer)

## Contributing

Contributions welcome! Read [CONTRIBUTING.md](CONTRIBUTING.md), pick a
[`good first issue`](https://github.com/RicSchonfelder/OpenShoot/issues?q=is%3Aissue+label%3A%22good+first+issue%22),
and open a PR.

## License

MIT — see [LICENSE](LICENSE). Third-party components and models are listed in
[THIRD_PARTY.md](THIRD_PARTY.md).
