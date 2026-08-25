# Third-Party Notices

OpenShoot is MIT licensed (see [LICENSE](LICENSE)) and builds on excellent
open-source work. This file lists third-party code, crates, libraries and AI
models used by the project, together with their licenses.

> **License verification note:** Licenses below were verified against each
> project's official repository/crate metadata as of 2026-08. Always double-check
> the license of a specific model weight or crate version before commercial use —
> in particular, the NIMA and InsightFace *model weights* carry research-only
> restrictions that differ from their source-code licenses.

---

## AI Models

| Model | Used for | Source | License |
|---|---|---|---|
| **SCRFD** (`scrfd_2.5g_bnkps.onnx`) | Face detection | [InsightFace](https://github.com/deepinsight/insightface) (research project) | Source code: **MIT**. ⚠️ Official pretrained models are released for **non-commercial research** purposes only — verify terms before commercial deployment. The ONNX conversion used here follows Apache-2.0 per its distribution page; re-check the exact artifact you download. |
| **NIMA / MobileNet aesthetic** (`nima_mobilenet_aesthetic.onnx`) | Aesthetic quality scoring | Paper: *"NIMA: Neural Image Assessment"* — Google Research ([arXiv:1709.05424](https://arxiv.org/abs/1709.05424)); ONNX community conversions on Hugging Face | ⚠️ Reference implementations are research code; the paper's models have **no explicit open license**. Treat as non-commercial/research use unless you obtain clarity from Google. |
| **MobileFaceNet** (`mobilefacenet.onnx`) | Face embeddings for person grouping | Paper: *"MobileFaceNets: Efficient CNNs for Accurate Real-Time Face Recognition"*; common conversions via [InsightFace](https://github.com/deepinsight/insightface) | Source code: MIT (InsightFace). ⚠️ Pretrained weights inherit InsightFace's **non-commercial research** restriction. |

## Rust Crates (core/)

| Crate | Purpose | License |
|---|---|---|
| [ort](https://crates.io/crates/ort) | ONNX Runtime bindings (inference, CoreML EP) | Apache-2.0 |
| [image](https://crates.io/crates/image) | Image decoding/encoding (JPEG, PNG, WebP, TIFF) | MIT |
| [rusqlite](https://crates.io/crates/rusqlite) | SQLite catalog (bundled SQLite: public domain) | MIT |
| [kamadak-exif](https://crates.io/crates/kamadak-exif) | EXIF metadata reading | BSD-2-Clause |
| [imageproc](https://crates.io/crates/imageproc) | Image processing primitives | MIT |
| [walkdir](https://crates.io/crates/walkdir) | Recursive directory traversal | MIT (MIT/Apache-2.0 dual) |
| [sha2](https://crates.io/crates/sha2) | SHA-2 hashing (dedupe/cache keys) | Apache-2.0 OR MIT |
| [rayon](https://crates.io/crates/rayon) | Data parallelism | MIT OR Apache-2.0 |
| [tokio](https://crates.io/crates/tokio) | Async runtime | MIT |
| [ndarray](https://crates.io/crates/ndarray) | Tensor/array math for ML pipelines | MIT OR Apache-2.0 |
| [napi-rs](https://crates.io/crates/napi) (+ `napi-derive`, `napi-build`) | Rust → Node.js native addon bridge | MIT |
| [chrono](https://crates.io/crates/chrono) | Date/time handling | MIT OR Apache-2.0 |
| [dirs](https://crates.io/crates/dirs) | Platform data/config directories | MIT OR Apache-2.0 |
| [serde](https://crates.io/crates/serde) (+ `serde_json`) | Serialization | MIT OR Apache-2.0 |
| [base64](https://crates.io/crates/base64) | Base64 encoding | MIT OR Apache-2.0 |

Dev-only: [tempfile](https://crates.io/crates/tempfile) (MIT OR Apache-2.0),
[anyhow](https://crates.io/crates/anyhow) (MIT OR Apache-2.0).

## JavaScript / Node

| Package | Purpose | License |
|---|---|---|
| [react](https://react.dev) / react-dom | UI framework | MIT |
| [react-window](https://github.com/bvaughn/react-window) (+ react-virtualized-auto-sizer) | Virtualized photo grid | MIT |
| [electron](https://www.electronjs.org) | Desktop app shell | MIT |
| electron-vite, vite, typescript, eslint, @vitejs/plugin-react | Build tooling | MIT |
| electron-builder | Packaging (.app/.dmg) | MIT |

## Runtime Dependencies (system)

- **ONNX Runtime** — bundled via the `ort` crate build. Microsoft ONNX Runtime
  is licensed under the **MIT License**.
- **SQLite** — public domain (bundled by `rusqlite`).
