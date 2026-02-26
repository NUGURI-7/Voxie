# 🎙️ Voxie

**A lightweight floating voice-to-text app for macOS & Windows**
**轻量级浮窗语音转文字应用，支持 macOS 和 Windows**

---

## ✨ Features · 功能特性

| | English | 中文 |
|---|---|---|
| 🎤 | Local transcription via Whisper | 本地 Whisper 语音识别（离线）|
| ☁️ | Cloud transcription (OpenAI / Aliyun / Custom) | 云端识别（OpenAI / 阿里云 / 自定义）|
| 🌐 | Translation: zh ↔ en, 简 ↔ 繁 | 中英互译、简繁转换 |
| 📋 | Auto-copy result to clipboard | 识别结果自动复制到剪贴板 |
| 🕹️ | Global shortcut to toggle window | 全局快捷键呼出/隐藏浮窗 |
| 🎨 | 7 color themes | 7 款主题配色 |
| 🗂️ | Transcription history | 历史记录管理 |
| 🖥️ | Always-on-top floating window | 桌面常驻悬浮窗口 |
| 🔋 | Unload model to free RAM | 一键卸载模型释放内存 |

---

## 🖼️ Screenshots · 截图

> Coming soon · 截图待添加

---

## 🚀 Download · 下载

Go to [Releases](../../releases) to download the latest version.

前往 [Releases](../../releases) 下载最新版本。

| Platform · 平台 | File · 文件 |
|---|---|
| macOS (Apple Silicon) | `Voxie_aarch64.dmg` |
| macOS (Intel) | `Voxie_x86_64.dmg` |
| Windows | `Voxie_x64-setup.exe` |

---

## 🛠️ Local Development · 本地开发

**Prerequisites · 前置依赖**

- [Node.js 20+](https://nodejs.org/)
- [Rust](https://rustup.rs/)
- macOS: Xcode Command Line Tools
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + [LLVM](https://llvm.org/builds/)

```bash
# Install dependencies · 安装依赖
npm install

# Run in dev mode · 开发模式运行
npm run tauri dev

# Build · 构建
npm run tauri build
```

---

## 🤖 Local Models · 本地模型

Powered by [whisper.cpp](https://github.com/ggerganov/whisper.cpp). Models are downloaded on demand inside the app.

使用 [whisper.cpp](https://github.com/ggerganov/whisper.cpp) 推理引擎，模型在应用内按需下载。

| Model | Size | Speed | Accuracy |
|-------|------|-------|----------|
| tiny | ~75 MB | ⚡⚡⚡⚡ | ★★☆☆ |
| base | ~142 MB | ⚡⚡⚡ | ★★★☆ |
| small | ~466 MB | ⚡⚡ | ★★★★ |
| medium | ~1.5 GB | ⚡ | ★★★★ |
| large-v3 | ~3.1 GB | 🐢 | ★★★★★ |

---

## 🏗️ Tech Stack · 技术栈

- **Frontend:** Vue 3 + TypeScript + Vite
- **Backend:** Rust + Tauri 2
- **Inference:** whisper-rs (whisper.cpp bindings)
- **GPU:** Metal (macOS Apple Silicon)

---

## 📄 License · 许可证

MIT
