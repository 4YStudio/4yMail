# 更新日志 / Changelog

本文件记录 4yMail 的所有重要变更。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。
更早的版本见 [GitHub Releases](https://github.com/4YStudio/4yMail/releases)。

## [1.2.0] - 2026-11-24

本次发布是一次结构收敛 + 一个长期困扰的渲染问题的根治。

### 修复

- **界面动画闪烁（打包版必现）**：任何动画（菜单展开、弹层、路由切换、列表入场、
  进度条，甚至一个 opacity 过渡）在动画首尾都会短暂显示该元素的旧内容，而
  `pnpm dev` 在 Chrome/Firefox 里完全正常。根因是 **WebKitGTK ≥ 2.42 默认启用的
  DMABUF renderer** 在 AMD（RADV）/ NVIDIA 专有驱动 + Wayland 上会在图层合成状态
  变化时显示上一块表面约 1–2 帧，与动画改动哪个属性无关，也不是 CSS 写错。
  现在应用启动时（`src-tauri/src/main.rs` 的 `configure_webkit_renderer()`，在 WebKit
  初始化之前）默认设置 `WEBKIT_DISABLE_DMABUF_RENDERER=1`，只关 DMABUF，**加速合成
  仍然保留**。`.deb` 安装版同样生效。
  - 想要回 WebKit 默认路径：`FOURYMAIL_WEBKIT_RENDERER=dmabuf`（或 `gpu` / `default`）
  - 排查兜底：`FOURYMAIL_WEBKIT_RENDERER=soft`，额外关掉加速合成（整页 CPU 渲染）
  - 自己 export 了 `WEBKIT_DISABLE_DMABUF_RENDERER` 时以你的值为准
  - 代价：共享内存 + Cairo 路径下每帧成本与像素量成正比（2× 缩放下约 30–40 fps），
    因此动画时长统一压在 160–220 ms

### 变更

- **仓库改为纯 Tauri 结构**：移除已废弃的 Electron 版本（`electron/`、旧的 Electron
  渲染层 `src/`、`release/`、`dist/`），`tauri/` 下的 `src`、`src-tauri`、`public`、
  `scripts`、`tauri-bridge.js` 整体上提到仓库根，`package.json` 重写为 Tauri 项目。
  模块路径与 `src-tauri/`、`public/`、`dist/` 相关脚本、CI 配置均已同步。
- 包管理器固定为 `pnpm@12.14.0`；新增 `pnpm-workspace.yaml` 的 `allowBuilds.esbuild`
  （pnpm 12 的构建脚本许可机制），不再依赖已失效的 `package.json#pnpm` 字段。
- `tauri.conf.json` 的 `beforeDevCommand` / `beforeBuildCommand` 由 `npm run` 改为 `pnpm`。
- **CSS 动画清理**（配合上面的帧率预算）：去掉 `.app-view-host` 上常驻的
  `transform: translateZ(0)` 与四段「恒等 transform」常驻图层提升——静止元素常驻图层
  会让 WebKitGTK 每帧重新上传整个图层，帧率反而更低；路由进场 0.32 s → 0.18 s，
  列表入场 `translate3d(0,4px,0)` → `translateY(4px)`，进场动画不再带 opacity。
- 视图容器统一为绝对定位盒子（`.app-view-host > .view.view`），切换期间不再参与
  父级 flex 重排。

### 新增

- **GitHub Actions 五平台安装包**：推送 `main`、开 PR、手动触发都会构建并上传
  artifact（保留 14 天）；打 `v*` tag 时汇总成一个 draft Release。
  | 平台 | 目标 | 产物 |
  | --- | --- | --- |
  | Windows x64 | `x86_64-pc-windows-msvc` | NSIS `.exe` |
  | Windows arm64 | `aarch64-pc-windows-msvc` | NSIS `.exe` |
  | Linux x64 | `x86_64-unknown-linux-gnu` | `.deb` + `.rpm` |
  | Linux arm64 | `aarch64-unknown-linux-gnu`（原生 arm 运行器） | `.deb` + `.rpm` |
  | macOS | `universal-apple-darwin` | `.dmg` |
- `docs/tauri-webkitgtk-flicker.md`：动画闪烁的完整排查记录——症状、根因、修法、
  四条错误路径、帧率代价与量化验证手段，避免后人再从 CSS 里刨一遍。

### 已知事项

- `tauri-bridge.js` 仍保留 `window.electronAPI` 兼容层，前端组件中的
  `isElectron` / `window.electronAPI.*` 名称暂未改名（纯命名遗留，不影响行为）。
- WebKitGTK 上 CSS `transition` 不是逐帧推进（实测 190 ms 只产出 3 个不同帧），
  路由/入场动画后续可改用 Web Animations API（`element.animate()`）以获得更顺的
  逐帧表现；该改动不涉及闪烁问题，留待后续版本。

### 验证

- `cargo check` 通过，`pnpm build`（vite）通过。
- 本地 `pnpm tauri build --target x86_64-unknown-linux-gnu --bundles deb,rpm` 产出
  `4YMail_1.2.0_amd64.deb` 与 `4YMail-1.2.0-1.x86_64.rpm`。

## 早期版本

- `1.1.0` 及更早：见 [GitHub Releases](https://github.com/4YStudio/4yMail/releases)。
