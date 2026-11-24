# Tauri + WebKitGTK：动画抽搐/闪烁的真正成因与修法

> 本文来自云语聚合网盘（Yunyu Drive，Tauri 2 + Vue 3 + WebKitGTK）的实际排查记录。
> 现象是「**只要播放动画就闪**」，在浏览器里完全正常、装成 deb 后在 Tauri 里必现。
> 结论只有一条：**关掉 WebKitGTK 的 DMABUF renderer**。下面把怎么走到的、踩了哪些坑、
> 代价是什么全部写清楚，免得后人（包括我自己）再从 CSS 里刨一遍。

## 症状

- 任何动画都会闪：菜单展开、弹层、路由切换、列表行入场、进度条，甚至只是一个
  opacity 过渡。
- 表现为**动画结束的那一帧附近，短暂显示该元素（或其祖先图层）的旧内容**：
  看起来像「动画播完 → 元素消失 → 又出现」，或者「先跳回原位再滑到目标位」。
- `pnpm dev` 在 Chrome / Firefox 里看**完全正常**；`.deb` 安装版、`pnpm tauri dev`
  的 WebKitGTK 里**每次必现**。
- 去掉所有动画（`prefers-reduced-motion` 或干脆删掉）后一切正常 —— 这正是误导人的地方：
  它让人以为是 CSS/JS 动画写错了。

**第一条判据最重要**：浏览器正常、WebKitGTK 异常，说明不是样式写错，而是合成器行为不同。
不要在 CSS 里堆 `z-index`、`will-change`、`transform: translateZ(0)`，见下面「四条错误的路」。

## 根因

WebKitGTK ≥ 2.42 默认启用 **DMABUF renderer**：页面图层以 DMA-BUF 缓冲区直接交给 GPU
合成（省掉一次内存拷贝，正常情况下更快、更省电）。

在 **AMD（RADV）或 NVIDIA 专有驱动 + Wayland** 上，这条路径存在缓冲生命周期问题：
图层的**合成状态每变化一次**（动画开始、动画结束、图层被提升/降级、元素被加进/移出层树），
就会短暂显示该图层**上一块表面**的内容约 1–2 帧。

动画恰恰是「反复改变图层合成状态」的操作，所以表现成「只要动就闪」，与动了什么属性
（transform、opacity、width、position）无关 —— 这一点是区分「CSS 写法问题」与「渲染器问题」
的关键证据：**换成纯 opacity 过渡、换成更高的合成层、换动画库，全都照闪**。

验证方式（不需要装任何工具）：

```bash
# 1) 复现闪烁
pnpm tauri dev

# 2) 只在关掉 DMABUF 后重启，同一个动画
WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev
```

如果第 2 步不闪了，就是这个问题，不用再排查别的东西。

## 修法

在 **WebKit 初始化之前**把它写进环境变量。云语里放在 `src-tauri/src/main.rs`：

```rust
fn main() {
    // WebKitGTK ≥2.42 默认走 DMABUF renderer（页面图层经 DMA-BUF 直接交给 GPU 合成）。
    // 在 AMD/NVIDIA + Wayland 上，它会在图层合成状态变化时（任何动画的首尾）短暂显示
    // 该图层的旧表面，表现为「动画一播就闪 / 播完消失再出现」。
    // 关掉它只是让 WebKit 回落到共享内存渲染器，图形加速合成仍在，代价是别的层面
    // 的重绘成本更高（见下面的帧率一节）。环境变量留作逃生口，方便临时开回来。
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    yunyu_drive_lib::run()
}
```

要点：

- **必须在 `run()` 之前**。WebKit 进程一旦起来就读不到这个变量了。
- **只关 DMABUF，不要关加速合成**。`WEBKIT_DISABLE_COMPOSITING_MODE=1` 会把整个渲染
  压到 CPU（滚动、模糊、圆角全部重算），比闪烁还难用。这两个变量差一个字，效果差一个量级。
- 用 `if ... is_none()` 包一层，让用户/开发者可以用环境变量临时改回去排查问题。
- `.deb` 安装版同样生效，因为这是应用进程内的环境变量，不依赖启动器脚本。

## 代价：动画会掉到 30–40 帧

关掉 DMABUF 后 WebKit 回落到共享内存 + Cairo 路径：每帧把脏区域画进 CPU 内存，再整块
上传给合成器。成本与**像素量**成正比。

云语的实测环境（KDE Plasma Wayland，2× 缩放，窗口 1220×800）：

| 项 | 数值 |
| --- | --- |
| 窗口 CSS 像素 | 1220 × 800 |
| 实际渲染像素（2×） | 2440 × 1600 ≈ 390 万 |
| 动画帧率（观感） | 30–40 fps |

这是渲染路径的物理上限，不是代码问题。能做的事：

1. **把动画时长压短**（云语：列表 0.16 s、路由 0.18 s、弹层 0.22 s）。160–220 ms 的过渡
   在 30 fps 下也还有 5–7 帧，肉眼是顺的；反过来 400 ms 的慢动画在这个帧率下会明显顿。
2. **减少同时在动的像素**：一次只让一个容器动（云语 `.stage` 同一时刻只挂一个视图），
   不要给整页做位移/缩放。
3. **不要给静止元素挂常驻的 `will-change` / `contain: paint`**：它们会把图层长期提升，
   每帧都要重新上传整个图层（实测帧率反而更低，而且更容易闪）。只在动画期间加，
   结束就摘掉 —— 用 `element.animate()` 时 WebKit 会自动处理，手写 CSS 类则要自己收尾。
4. 真要更高帧率，只能在**降低渲染倍率**（比如把系统缩放从 2× 调到 1.5×）上做取舍，
   这是唯一真正换回帧率的手段。

## 四条错误的路（都试过，都没用，别再走）

这些方案在「图层合成状态变化」这个根因面前必然失败，但过程中确实能观察到一些现象，
容易被误读成「有效果」：

1. **换动画属性**。transform / opacity / width / position 全试过，一样闪。
   有一段推理是「动画结束停在 `transform: none` 会让图层被拆掉重绘」，于是给静止态补
   `transform: translateY(0)`、补 `translateZ(0)`、补 `isolation`、补 `contain: paint` ——
   异常帧数从 22 降到 7，但**没消失**。
2. **换成 Web Animations API（`element.animate`）**，加 `fill: "both"`。这确实修掉了
   **另一个**问题（见下面「CSS transition 在 WebKitGTK 上不是逐帧推进」），但闪烁照旧。
3. **换动画库**（motion-v / framer-motion 的 Vue 端口）。库只负责「怎么算值」，
   不控制合成器怎么把图层交出去，所以同样照闪。
4. **逐帧调 CSS 时序**（缩短时长、换 easing、拆成多段 keyframes）。只是把闪烁挪到别的时刻。

顺带一个**真实存在、值得修**的问题（和闪烁无关，但会被误认为闪烁）：

> **CSS `transition` 在 WebKitGTK 上不是逐帧推进的。** 云语早期用
> `get_snapshot` 1100×720 @2× 抓帧实测：一个 190 ms 的 CSS transition 只产出
> **3 个不同帧**、最大帧间隔 210 ms，还拖着重绘 21k–43k px；同一动画改用
> `element.animate()` 后是 **10 个不同帧**、最大间隔 26 ms。
>
> 所以 WebKitGTK 上的入场/位移动画应该走 Web Animations API（或基于它的库），
> 不要用 CSS `transition`。这条修完动画会流畅，但闪烁仍在 —— 直到关掉 DMABUF。

## 如何量化验证（如果你要自己复现）

不要靠眼睛，抓帧算术：

```bash
# 录屏（Wayland + KWin 下可用 obs / wf-recorder；KWin 不支持 wlr-screencopy 时用 obs）
# 然后用 ffmpeg 逐帧 diff，找「跳变帧」和「相同帧连跑」
ffmpeg -i capture.mp4 -vf scale=426:240 -f rawvideo -pix_fmt rgb24 -
```

云语用的脚本简化版（逐帧算平均绝对差、亮度、变化像素数，标出 `diff > 3×均值` 的跳变帧，
并统计「最长相同帧连跑时长」）：

```
changed frames / mean diff / max diff / 最长相同帧连跑（ms）
```

- 闪烁 → 出现 1–2 帧的孤立跳变（旧内容），中间夹着几乎静止的帧。
- 掉帧 → 大量帧完全相同（连跑 30–60 ms），最后突然跳到终点。

另外一个好用的辅助手段：在页面里用 `requestAnimationFrame` 逐帧采样
`getComputedStyle` 的 `opacity` / `transform`、`anim.currentTime`、`node.isConnected`，
通过 Tauri 的 `invoke` 打到 Rust 侧日志（WebKitGTK 的 `console.log` 不会写进任何日志文件，
`WEBKIT_INSPECTOR_SERVER` 也不是 CDP，不能当 Chrome DevTools 用 —— 只能走 `invoke` 这条路）。
云语保留了 `src/lib/probe.ts` + `#[cfg(debug_assertions)] debug_probe` 命令做这件事：

```ts
import { probeAnimation } from "@/lib/probe";
probeAnimation("pop", el, anim); // 每帧记 ct / op / tr / conn / dt
```

用它可以直接区分「动画值没推进」和「值推进了但屏幕上没画出来」——前者是 JS/引擎问题，
后者才是合成器问题。云语当年就是靠它证明「每帧 dt=16ms、ct 从 12 平滑到 190、末帧
op=1、图层仍连接」，从而把怀疑从动画代码转到渲染器上。

## 环境信息（复现参考）

| 项 | 值 |
| --- | --- |
| 桌面 | KDE Plasma，Wayland |
| 缩放 | 2×（屏 2133×1200 @scale 2 → 4266×2400） |
| WebKitGTK | `webkit2gtk-4.1` / `javascriptcoregtk-4.1` 2.54.0 |
| GPU | AMD（Mesa 26.1.6，RADV） |
| Tauri | 2.x，窗口 1220×800，`decorations: false` |

同机 Chromium 的同一个页面完全正常 —— 这就是前面那条「第一条判据」。

## 一句话总结

**动画闪烁 = WebKitGTK 的 DMABUF renderer 在 AMD/Wayland 上显示陈旧表面，用
`WEBKIT_DISABLE_DMABUF_RENDERER=1` 关掉它；付出 30–40 fps 的代价，把动画时长压到
200 ms 左右即可；不要在 CSS 里找原因。**
