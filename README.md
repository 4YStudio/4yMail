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
> 项目已全面迁移到 Tauri 2.0，旧的 Electron 版本已从仓库移除。

```
.
├── src/                  # Vue 3 前端
├── src-tauri/            # Rust 后端（Tauri）
│   ├── src/main.rs       # 入口（含 WebKitGTK 渲染器设置）
│   ├── src/lib.rs        # 命令、IMAP/SMTP、后台轮询
│   └── tauri.conf.json   # 窗口/打包配置
├── public/               # 静态资源（图标、提示音）
├── scripts/tauri.js      # 打包脚本入口（pnpm tauri ...）
└── docs/                 # 排查记录
```

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

### 🚀 Getting Started / 快速开始

#### Prerequisites / 前置准备
- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://rust-lang.org/) (latest stable)
- [pnpm](https://pnpm.io/)

#### Setup / 初始化
```sh
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
# 全量打包（按 tauri.conf.json 的 targets 生成 deb/rpm/appimage）
pnpm tauri build

# 只生成指定安装包；可选 appimage、deb、rpm、nsis、dmg
pnpm tauri build --bundles deb

# 指定目标架构（交叉/CI 用）
pnpm tauri build --target x86_64-unknown-linux-gnu --bundles deb,rpm
```

> [!NOTE]
> **关于 Linux 上曾有过的「动画闪烁」**：以前这个应用在启动时会默认设置
> `WEBKIT_DISABLE_DMABUF_RENDERER=1` 来压动画闪烁。那是误判（当时的环境是 Debian 13 +
> KDE 6 + Wayland，问题出在该环境的 WebKitGTK webview，与本应用无关），已经回退：
> 现在**不设任何 `WEBKIT_*` 变量**，走 Tauri / WebKitGTK 默认配置，保留硬件加速合成。
> 如果某台 Linux 机器上自己遇到类似现象，用环境变量临时验证即可（不要写回代码）：
>
> ```sh
> WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev
> WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 pnpm tauri dev
> ```
>
> 历史排查记录保留在 [docs/tauri-webkitgtk-flicker.md](./docs/tauri-webkitgtk-flicker.md)
> （开头已标注结论不成立）。

---

### 🌐 连不上邮件服务器？（993 / 465 被拦截）

如果连 QQ / 163 / Gmail 等邮箱时提示「连接被重置」「TLS 握手失败」，先不要怀疑账号密码。
这类**隐式 TLS** 邮件端口（IMAP 993、SMTP 465、POP3 995）在国内的校园网、公司网、
部分运营商上会被中间设备直接 RST：TCP 能建立，但一发 TLS 握手就被掐断。

一条命令就能确认（在终端里跑，不是应用的问题就一定也被重置）：

```sh
# 正常：打印证书信息；被拦截：write:errno=104
openssl s_client -connect imap.qq.com:993 -servername imap.qq.com </dev/null | head -5
# 对照：587 / 143 一般不受影响
openssl s_client -connect smtp.qq.com:587 -starttls smtp -servername smtp.qq.com </dev/null | head -5
```

应用内置的应对：

1. **网络代理**：账号编辑页底部「网络代理（可选）」支持 **SOCKS5** 与 **HTTP CONNECT**，
   IMAP 与 SMTP 都会经代理连接（带用户名 / 密码认证）。填上代理软件的本地混合端口即可，
   例如 Clash Verge / FlClash 的 `127.0.0.1:7890`（需在代理软件里开启「混合端口」或「允许局域网」）。
2. **端口回退**：SMTP 465 失败会自动再试 587（STARTTLS）；IMAP 993 失败会再试 143
   （STARTTLS，QQ 不支持但 163 / Gmail / 自建服务器大多支持）。
3. **报错会说人话**：被 RST 时直接给出「连接被网络重置」和解决思路，而不是一句 `Connection reset by peer`。

> 代理软件的「系统代理」模式只影响读 `http_proxy` 环境变量的程序（浏览器、curl 等），
> 不会接管邮件客户端的裸 TCP 连接。要么用本应用内置的代理设置，要么开 TUN / 虚拟网卡模式。

---

### 📦 CI/CD / 自动化构建

GitHub Actions 会在推送 `main`、开 PR、手动触发以及打 `v*` tag 时构建 5 个平台的安装包：

| 平台 | 目标 | 产物 |
| --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` | NSIS 安装程序（`.exe`） |
| Windows arm64 | `aarch64-pc-windows-msvc` | NSIS 安装程序（`.exe`） |
| Linux x64 | `x86_64-unknown-linux-gnu` | `.deb` + `.rpm` |
| Linux arm64 | `aarch64-unknown-linux-gnu`（原生 arm 运行器） | `.deb` + `.rpm` |
| macOS | `universal-apple-darwin`（Intel + Apple Silicon 合一） | `.dmg` |

每次都作为 workflow artifact 上传（保留 14 天）；打 `v*` tag 时还会汇总成一个 **draft Release**。

> Linux arm64 使用 GitHub 的原生 arm 运行器（`ubuntu-22.04-arm`），公开仓库免费；
> 私有仓库需要改用交叉编译（`cargo-zigbuild` + `--target aarch64-unknown-linux-gnu`
> 并安装 arm64 的 webkit2gtk 开发包），或删掉 `linux-arm64` 那一行。

---

### 📝 Changelog / 更新日志

见 [CHANGELOG.md](./CHANGELOG.md)。当前版本 **1.2.1**：新增网络代理（SOCKS5 / HTTP）与
SMTP 465→587、IMAP 993→143 端口回退，连接被重置时给出明确原因；修复说明见 1.2.0。

---

### 📄 License / 开源协议

This project is licensed under the [MIT License](./LICENSE).  
本项目采用 [MIT 协议](./LICENSE) 授权。
