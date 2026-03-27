<div align="center">
  <img src="./public/logo.png" alt="4yMail Logo" width="120" height="120" />
  <h1>4yMail</h1>
  <p><strong>A Modern, Elegant, and High-Performance Desktop Email Client</strong></p>
  <p><strong>一款现代、优雅、高性能的桌面邮件客户端</strong></p>

  <p>
    <img src="https://img.shields.io/badge/Vue-3.x-4fc08d?style=flat-square&logo=vue.js" alt="Vue" />
    <img src="https://img.shields.io/badge/Tauri-2.x-24C8DB?style=flat-square&logo=tauri" alt="Tauri" />
    <img src="https://img.shields.io/badge/Rust-2021-000000?style=flat-square&logo=rust" alt="Rust" />
    <img src="https://img.shields.io/badge/Vite-7.x-646CFF?style=flat-square&logo=vite" alt="Vite" />
    <img src="https://img.shields.io/badge/License-MIT-yellow?style=flat-square" alt="License" />
  </p>
</div>

---

### 📂 Project Structure / 项目结构

> [!IMPORTANT]
> 本项目目前正处于架构迁移阶段，建议新用户直接使用 Tauri 版本。

- **`/tauri` (Recommended / 推荐)**:  
  **New Refactored Version**. Built with Tauri 2.0 and Rust. Faster, smaller, and more secure.  
  **全新重构版**。基于 Tauri 2.0 和 Rust 构建。体积更小、速度更快、更安全。
  
- **`/` (Legacy / 旧版)**:  
  **Original Electron Version**. Maintenance only.  
  **初始 Electron 版本**。目前仅处于维护状态。

---

### ✨ Key Features / 核心特性

- **💎 Liquid Glass UI / 液态玻璃 UI**  
  A premium, translucent interface with dynamic gradients and glassmorphism effects.  
  极具质感的半透明界面，配合动态渐变背景与玻璃拟态效果。

- **🎬 Fluid Animations / 灵动动效**  
  Staggered list loading, smooth route transitions, and interactive micro-animations.  
  支持阶梯式列表加载、顺滑的路由转场以及细腻的交互微动效。

- **🚀 Multi-account Support / 多账号支持**  
  Manage multiple IMAP/SMTP accounts seamlessly with quick-switch capabilities.  
  无缝管理多个 IMAP/SMTP 账号，支持快速切换与统一视图管理。

- **📫 Background Services / 后台服务 (Tauri Only)**  
  Efficient mail polling and system-level notifications even when minimized.  
  高效的邮件轮询与系统级原生通知，支持最小化后台运行。

---

### 🛠 Tech Stack / 技术栈 (New Version)

- **Frontend**: Vue 3 (Composition API), Vite, Vanilla CSS
- **Core (Backend)**: Rust (Tauri 2.0)
- **Email Parser**: `mail-parser` (Rust), `imap` (Rust), `lettre` (Rust)
- **Typography**: Inter (Google Fonts)

---

### 🚀 Getting Started / 快速开始 (Tauri Version)

#### Prerequisites / 前置准备
- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://rust-lang.org/) (latest stable)
- [pnpm](https://pnpm.io/)

#### Setup / 初始化
```sh
cd tauri
pnpm install
```

#### Development / 开发环境
```sh
# Run Vite dev server and Tauri
pnpm tauri dev
```

#### Build / 打包构建 (Local)
> [!TIP]
> 建议使用 GitHub Actions 进行跨平台打包。

```sh
pnpm tauri build
```

---

### 📦 CI/CD / 自动化构建
本项目已配置 GitHub Actions，在每次推送至 `main` 或发布 `tag` 时会自动构建 Windows 和 Linux 版本的安装包。

---

### 📄 License / 开源协议

This project is licensed under the [MIT License](./LICENSE).  
本项目采用 [MIT 协议](./LICENSE) 授权。

---

<div align="center">
  <p>Built with ❤️ by Antigravity & USER</p>
</div>
