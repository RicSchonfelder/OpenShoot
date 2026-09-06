# OpenShoot 📸

[![CI](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml/badge.svg)](https://github.com/RicSchonfelder/OpenShoot/actions/workflows/ci.yml) [![最新版本](https://img.shields.io/github/v/release/RicSchonfelder/OpenShoot)](https://github.com/RicSchonfelder/OpenShoot/releases/latest)

**语言:** [English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md)

> **面向摄影师的 AI 选片、编辑和修图工具——默认优先本地处理与离线使用。**

OpenShoot 是一款面向摄影师的桌面应用。主要照片分析默认在用户电脑上运行；可选的网络功能彼此独立，并且需要明确启用。原始照片会被保留，编辑元数据可以写入 XMP sidecar 文件。

## 下载

请根据你的操作系统选择最新版本：

| 操作系统 | 下载 |
|---|---|
| **Linux x86_64** | [AppImage](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-x86_64.AppImage) · [Debian/Ubuntu `.deb`](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Linux-amd64.deb) |
| **Windows x64** | [Windows 安装程序](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-Windows-Setup-x64.exe) |
| **macOS Apple Silicon** | [macOS arm64 DMG](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-arm64.dmg) |
| **macOS Intel** | [macOS x64 DMG](https://github.com/RicSchonfelder/OpenShoot/releases/latest/download/OpenShoot-0.1.0-macOS-x64.dmg) |

如果直接链接不可用，请打开[完整 Release 页面](https://github.com/RicSchonfelder/OpenShoot/releases/latest)，选择文件名中包含对应操作系统的安装包。

## 主要功能

- 使用美学评分、人脸检测、清晰度和曝光分析进行 AI 选片。
- 在选片、网格和放大视图中提示闭眼照片。
- 非破坏性批量编辑：曝光、白平衡、对比度、HSL 和曲线。
- 皮肤平滑、面部增强、背景虚化和选择性蒙版。
- 按人物对照片进行分组。
- 相册以及 IMPORT → CULL → EDIT → RETOUCH 工作流。
- 兼容 Lightroom Classic 和 Capture One 的 XMP sidecar。
- JPEG/PNG 导出以及 NEF、ARW、DNG、CR2、CR3 RAW 预览支持。

## 要求

- Linux x86_64（glibc ≥ 2.38）、Windows 10+ 或 macOS arm64/x64。
- 开发环境：Rust ≥ 1.88 和 Node.js ≥ 20。
- ONNX 模型应放在 `core/models/`，或放在 `OPENSHOOT_MODELS_DIR` 指定的目录中。

## 开发

```bash
git clone https://github.com/RicSchonfelder/OpenShoot.git
cd OpenShoot
npm install
npm run build:core
npm run dev
```

请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)、[docs/DESIGN.md](docs/DESIGN.md) 和 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 许可证

MIT。第三方组件和模型记录在 [THIRD_PARTY.md](THIRD_PARTY.md) 中。
