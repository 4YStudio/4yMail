// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 这里**不要**设置任何 WEBKIT_* 环境变量，走 Tauri / WebKitGTK 的默认配置。
    //
    // 曾经在这里默认注入过 WEBKIT_DISABLE_DMABUF_RENDERER=1 来压动画闪烁，那是误判：
    // 当时的现象是「任何动画首尾闪一帧旧画面」，被归因为 WebKitGTK 的 DMABUF renderer，
    // 实际上是我这台机器（Debian 13 + KDE 6 + Wayland）上 WebKitGTK webview 的问题，
    // 不是应用、也不是 Tauri 的问题。用环境变量去绕，代价是丢掉硬件加速合成、全局掉帧，
    // 换一台环境就完全不适用，所以已经回退。
    //
    // 如果某台 Linux 机器上确实遇到类似的合成/闪烁问题，用环境变量临时验证即可，
    // 不要写回代码里：
    //   WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev
    //   WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 pnpm tauri dev
    //   WEBKIT_DISABLE_COMPOSITING_MODE=1 pnpm tauri dev   # 整页 CPU 渲染，仅排查用
    fourym_lib::run()
}
