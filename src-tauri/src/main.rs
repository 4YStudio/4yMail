// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 必须在 WebKit 初始化之前设置（也就是必须在 run() 之前）：
    // WebKit 进程一旦起来就读不到这个变量了。
    configure_webkit_renderer();

    fourym_lib::run()
}

/// WebKitGTK ≥ 2.42 默认启用 DMABUF renderer（页面图层经 DMA-BUF 直接交给 GPU 合成）。
///
/// 在 AMD（RADV）/ NVIDIA 专有驱动 + Wayland 上，这条路径有缓冲区生命周期问题：
/// 图层的合成状态每变化一次（动画开始、动画结束、图层被提升/降级），就会短暂显示
/// 该图层**上一块表面**的内容约 1–2 帧。表现为「只要播动画就闪 / 播完消失再出现」，
/// 与动了哪个属性（transform、opacity、width）无关，浏览器里则完全正常。
///
/// 所以默认关掉它；关掉只是回落到共享内存渲染器，**加速合成仍然在**。
/// 代价是重绘成本与像素量成正比（2× 缩放下 1220×800 的窗口约 30–40 fps），
/// 因此动画时长都压在 160–220 ms，并且不要给静止元素挂常驻的
/// `will-change` / `translateZ(0)` / `contain: paint`（那样每帧都要重新上传整个图层，
/// 帧率反而更低）。
///
/// 逃生口（都不设时就是默认的「关 DMABUF」）：
///   FOURYMAIL_WEBKIT_RENDERER=dmabuf | gpu | default → 恢复 WebKit 默认的 DMABUF 路径
///   FOURYMAIL_WEBKIT_RENDERER=legacy                → 显式关 DMABUF（与默认相同）
///   FOURYMAIL_WEBKIT_RENDERER=soft                  → 再叠加 WEBKIT_DISABLE_COMPOSITING_MODE=1
///                                                     （整页 CPU 渲染，比闪烁还难用，仅排查用）
/// 用户自己 export 了 WEBKIT_DISABLE_DMABUF_RENDERER 时，完全尊重用户的值，不做覆盖。
///
/// 详见 docs/tauri-webkitgtk-flicker.md。
fn configure_webkit_renderer() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() {
        // 用户显式指定了，不再插手（`0` 表示保留 DMABUF，`1` 表示关掉）。
        return;
    }

    let mode = std::env::var("FOURYMAIL_WEBKIT_RENDERER")
        .unwrap_or_default()
        .to_ascii_lowercase();

    match mode.as_str() {
        // 恢复 WebKit 自己的默认（DMABUF）路径 —— 只用于排查/对比。
        "dmabuf" | "gpu" | "default" => {}
        // 兜底一档：连加速合成一起关掉，滚动/模糊/圆角全部回 CPU。
        "soft" => {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
        // 默认行为（含 legacy 及任意未知值）：只关 DMABUF，保留加速合成。
        _ => {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}
