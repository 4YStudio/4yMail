# Tauri + WebKitGTK：动画抽搐/闪烁的真正成因与修法

> ## ⚠️ 结论已被推翻（2026-11）
>
> **本文的归因是错的，不要照着改。**
>
> 当时的现象：（动画首尾闪一帧旧画面）确实是 WebKitGTK 合成的表现，但把原因定成「DMABUF
> renderer 在 AMD/NVIDIA + Wayland 上会显示陈旧表面」并让应用默认注入
> `WEBKIT_DISABLE_DMABUF_RENDERER=1`，是误判。再深查后确认：**问题出在我这台机器上的
> webview 环境（Debian 13 + KDE 6 + Wayland）**，不是应用自身、也不是 Tauri 的通用行为。
>
> 后果：那条开关在所有 Linux 发行版上都会把硬件加速合成关掉，牺牲全局帧率去换一个只在
> 特定环境下才需要的规避，换台机器就完全不适用。**4YMail 已回退：`src-tauri/src/main.rs`
> 不再设置任何 `WEBKIT_*` 环境变量，走 Tauri / WebKitGTK 默认配置。**
>
> 保留本文只作为排查过程存档（尤其是「不要在 CSS 里找原因」这个方向、以及 CSS transition
> 在 WebKitGTK 上不逐帧推进的实测数据）。如果新环境上再遇到同类现象，先用环境变量临时验证，
> 不要写回代码：
>
> ```sh
> WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev
> WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 pnpm tauri dev
> ```

> 本文来自云语（Yunyu Drive / 4YMail / 小云 PC Agent，均为 Tauri 2 + Vue 3 + WebKitGTK）的
> 实际排查记录，三个仓库共用同一份。现象是「**只要播放动画就闪**」，在浏览器里完全正常、
> 在 Tauri 窗口里必现。结论：**是 WebKitGTK 的合成器回归，不是 CSS 写错，也不是你的机器坏了**。
> 下面把怎么走到的、踩了哪些坑、代价是什么、以及开关该怎么投递写清楚，免得再从 CSS 里刨一遍。

## 症状

- 任何动画都会闪：菜单展开、弹层、路由切换、列表行入场、进度条，甚至只是一个
  opacity 过渡。
- 表现为**动画结束的那一帧附近，短暂显示该元素（或其祖先图层）的旧内容**：
  看起来像「动画播完 → 元素消失 → 又出现」，或者「先跳回原位再滑到目标位」。
- `pnpm dev` 在 Chrome / Firefox 里看**完全正常**；打包版、`pnpm tauri dev`
  的 WebKitGTK 里**每次必现**。
- 去掉所有动画（`prefers-reduced-motion` 或干脆删掉）后一切正常 —— 这正是误导人的地方：
  它让人以为是 CSS/JS 动画写错了。

**第一条判据最重要**：浏览器正常、WebKitGTK 异常，说明不是样式写错，而是合成器行为不同。
不要在 CSS 里堆 `z-index`、`will-change`、`transform: translateZ(0)`，见「四条错误的路」。

第二条判据：**把 `WEBKIT_DISABLE_DMABUF_RENDERER=1` 加到启动命令前面，闪烁消失**
（详见下一节）。这条成立，就说明问题在渲染器，与页面代码无关。

## 根因

两层原因叠加，都出在 WebKitGTK 侧：

1. **DMABUF renderer（≥ 2.42 默认开启）**：页面图层以 DMA-BUF 缓冲区直接交给 GPU 合成
   （省掉一次内存拷贝）。图层的**合成状态每变化一次**（动画开始、结束、图层被提升/降级、
   元素被加进/移出层树），就会短暂显示该图层**上一块表面**的内容约 1–2 帧。
2. **2.54 换掉了 web process 的合成器**：WebKitGTK 2.54.0（2026-09-16）把原来的
   TextureMapper 换成 Skia 合成器并默认开启（上游 PR "Enable skia compositor by default"）。
   这是 2.52.x → 2.54 之间**新出现**的变量：同一套页面在 2.52.6 上不闪。

配套的上游记录：

| 位置 | 内容 | 状态 |
| --- | --- | --- |
| WebKit bug 327524 | `[GTK] REGRESSION(2.52.6→2.54.1): Flickering and stuttering in CSS animations/transitions with transparency` | `NEW`，`Assignee: Nobody`，仅 CC 到 cgarcia（WebKitGTK 图形维护者）。**截至 2.54.3 未修** |
| WebKit bug 326841 | `[GTK] Animations flickering / jittering / stuttering in Google Calendar since WebKitGTK 2.54.x` | 开着。Fedora 上 AMDGPU + Mesa 的**台式机**和 Intel Kabylake **笔记本**都复现 → 不是 AMD/Wayland 专属 |
| tauri-apps/tauri#13493 | 同类报告 | FabianLars 回复 "webkitgtk is acting up more on every release… Since there's nothing we can do about this I'll close this as not planned" |

所以：**Tauri 在 Linux 上没有别的选择**（Wry 的 Linux 后端只有 WebKitGTK），只能靠应用侧开关规避。
同机 Chromium 正常，是因为它根本不用 WebKitGTK。

## 30 秒判定

```bash
# 1) 复现闪烁
pnpm tauri dev

# 2) 加一个环境变量重启，同一个动画
WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm tauri dev
```

第 2 步不闪了，就是这个问题，不用再排查别的东西。

再进一步，按下面的档位逐个试（三个项目通用，变量名见「三个项目的现状」）：

```bash
TAURI_WEBKIT_RENDERER=native  pnpm tauri dev   # 回 WebKit 默认（应该闪）
TAURI_WEBKIT_RENDERER=shm     pnpm tauri dev   # 2.54 起首选：保留 DMABUF 传输，只用共享内存缓冲
TAURI_WEBKIT_RENDERER=norenderer pnpm tauri dev  # 关掉 DMABUF renderer（历史验证过的一档）
```

## 修法

在 **WebKit 初始化之前**写进环境变量：也就是 `src-tauri/src/main.rs` 的 `main()` 里、`run()` 之前。

```rust
fn main() {
    configure_webkitgtk_renderer();
    yunyu_drive_lib::run()   // 4YMail: fourym_lib::run()；小云: app_lib::run()
}

/// 必须在 `run()` 之前：WebKit 的 web process 启动后就读不到后来改的环境变量了。
/// 只在 Linux 上有意义 —— macOS 走 WKWebView、Windows 走 WebView2(Chromium)，
/// 谁都不读 `WEBKIT_*`，所以用 cfg 显式挡掉，避免误导成"这段也影响 mac/win"。
#[cfg(target_os = "linux")]
fn configure_webkitgtk_renderer() {
    use std::env;

    // 用户/开发者自己 export 了，就完全尊重，不做覆盖。
    if env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() {
        return;
    }

    match env::var("TAURI_WEBKIT_RENDERER").unwrap_or_default().to_ascii_lowercase().as_str() {
        // 走 WebKit 自己的默认（2.54.x 上会闪，只用于排查/对比）
        "native" | "dmabuf" | "gpu" | "default" => {}
        // 2.54 起的首选档：DMABUF renderer 保留，但缓冲改成共享内存
        "shm" => env::set_var("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1"),
        // 兜底：连加速合成一起关，整页 CPU 渲染，比闪烁还难用，仅排查
        "soft" => {
            env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
        // 默认（含 norenderer 及任意未知值）
        _ => env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_webkitgtk_renderer() {}
```

要点：

- **必须在 `run()` 之前**。
- **只用前两档，永远不要默认 `soft`**。`WEBKIT_DISABLE_COMPOSITING_MODE=1` 会把整个渲染
  压到 CPU（滚动、模糊、圆角全部重算）。这两个变量差几个字，效果差一个量级。
- **留 `is_none()` 逃生口**，让用户能临时改回去复现问题。
- **加 `#[cfg(target_os = "linux")]`**。不加也不会坏（没有任何引擎读这些名字），但下一个人
  很容易误以为它在影响 macOS/Windows 版本，而且 `set_var` 会影响之后 spawn 的子进程环境。

### 2.54 起的一个语义变化（重要）

`WEBKIT_DISABLE_DMABUF_RENDERER=1` 在 2.54 上**已经不再回落到共享内存路径**了：它把
renderer 的 transport mode 留成空，走一条边缘路径（上游 bug 324730 记录的就是这条路径下
`startViewTransition()` 的空指针崩溃；block/buzz#3654 踩到后改用了 FORCE_SHM）。

新档位 `WEBKIT_DMABUF_RENDERER_FORCE_SHM=1` 的意思是：DMABUF renderer 照样启用，但缓冲
走共享内存而不是 GBM —— 保留传输通道，去掉出问题的那一段。Handy 等项目在 2.54.2 上就是
用它把闪烁修掉的。

本机 `libwebkit2gtk-4.1.so.0`（2.54.3）里 `strings` 出来的相关开关（都是运行期开关，不是编译期）：

```
WEBKIT_USE_SKIA_FOR_COMPOSITION      WEBKIT_DMABUF_RENDERER_FORCE_SHM
WEBKIT_DISABLE_DMABUF_RENDERER       WEBKIT_DISABLE_DMABUF_ATLAS
WEBKIT_FORCE_DMABUF_RENDERER         WEBKIT_DMABUF_RENDERER_BUFFER_FORMAT
WEBKIT_SKIA_ENABLE_CPU_RENDERING     WEBKIT_SKIA_ENABLE_DDL
WEBKIT_SHOW_DAMAGE / WEBKIT_SHOW_FPS / WEBKIT_SHOW_COMPOSITING_DEBUG_VISUALS
```

其中 `WEBKIT_USE_SKIA_FOR_COMPOSITION=0` 能退回旧合成器（二进制里 `TextureMapperLayer` /
`PaintTextureMapperLayerTree` 符号都还在，上游 bug 327701 的验证者就是靠它把 CPU 占用打回
0%）。这条**理论上**能同时拿到「不闪 + 不掉帧」，但会绕开上游主路径（TextureMapper 迟早被
移除），所以只作为实验档，不作为默认。

> ⚠️ **待验证**：本仓库的默认档目前是 `norenderer`（= 历史验证过不闪的 DISABLE）。`shm` 档
> 的代码路径已经就位，但没有在真机上做肉眼确认。确认 `TAURI_WEBKIT_RENDERER=shm` 也不闪、
> 且帧率更好之后，把 `main.rs` 里那个 `_ =>` 分支换成 `shm` 的默认值即可（一个词的改动）。

## 开关该投递到哪儿（为什么不是改配置文件就完事）

环境变量必须在 **WebKit 初始化前进入应用进程**，所以只有「进程自己的启动路径」有效：

| 投递方式 | 覆盖哪些启动 | 覆盖不到 | 适用 |
| --- | --- | --- | --- |
| **代码里 `set_var`（本仓库做法）** | 全部：菜单/终端/自启动/自动更新后自重启/`tauri dev` | — | ✅ 默认做法 |
| `deb`/`rpm` 的 `desktopTemplate` 里 `Exec=env VAR=1` | 仅从菜单/任务栏启动 | `tauri dev`、终端直接跑二进制、开机自启、被别的程序 exec | 发行版集成方统一覆盖 |
| AppImage 的 `AppRun` / 包装脚本 | 同上 | 同上 | 同上，且要自己接管生成的 AppRun（Tauri 无钩子） |
| flatpak `--env=` / snap `environment:` | 该 flatpak/snap 的所有启动 | 容器外启动 | 分发渠道定制 |
| systemd user unit 的 `Environment=` | 仅经 systemd 启动 | 手动启动 | 守护进程类（小云的 daemon） |

结论：**代码里那份是唯一能全覆盖的，必须留**。配置文件方式适合"发行版/分发方想统一钉住行为"，
作为补充；它连 `pnpm tauri dev` 都管不到，所以别指望只改打包配置就能在开发时看到效果。

需要 `desktopTemplate` 时的写法（Tauri v2）：

```json
"bundle": { "linux": { "deb": { "desktopTemplate": "linux/app.desktop" },
                       "rpm": { "desktopTemplate": "linux/app.desktop" } } }
```

```
[Desktop Entry]
Type=Application
Name={{name}}
Comment={{comment}}
Exec=env WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 {{exec}}
Icon={{icon}}
Categories={{categories}}
StartupWMClass={{wm_class}}
Terminal=false
```

## 跨平台：macOS / Windows 完全不受影响

这些开关**只有 WebKitGTK 会读**，跟"要不要关硬件加速"无关：

| 平台 | 渲染引擎 | 读 `WEBKIT_*`？ | 后果 |
| --- | --- | --- | --- |
| Linux | WebKitGTK | 读 | 生效 |
| macOS | WKWebView（Apple 的 WebKit） | 不读（Apple 侧是 `WebKitPreferences*` 这类 defaults 名） | 无操作，**照常硬件合成、不掉帧** |
| Windows | WebView2（Chromium/Edge） | 不读 | 无操作，同上 |

而且 `DMABUF` 本身是 Linux DRM 的概念，macOS 的 Core Animation / Windows 的 DComp 里没有
这个东西，变量名在那两个平台上连语义都不存在。**在代码里或打包配置里设它，都不会让
mac/win 版本变成 CPU 渲染，也不会掉帧。**（`std::env::set_var` 是进程级的、会被子进程继承，
所以还是建议用 `#[cfg(target_os = "linux")]` 把它关在 Linux 里，纯属保持环境干净和可读。）

## 代价：动画可能掉到 30–40 帧

`norenderer` 档下 WebKit 回落到共享内存 + Cairo 路径：每帧把脏区域画进 CPU 内存，再整块
上传给合成器。成本与**像素量**成正比。

实测环境（KDE Plasma Wayland，2× 缩放，窗口 1220×800）：

| 项 | 数值 |
| --- | --- |
| 窗口 CSS 像素 | 1220 × 800 |
| 实际渲染像素（2×） | 2440 × 1600 ≈ 390 万 |
| 动画帧率（观感，`norenderer` 档） | 30–40 fps |

这是 2.54.x 上**当前渲染路径**的实测上限，不是代码问题（换 WebKit 版本要复测）。能做的事：

1. **把动画时长压短**（0.16–0.22 s）。160–220 ms 的过渡在 30 fps 下还有 5–7 帧，肉眼是顺的；
   反过来 400 ms 的慢动画在这个帧率下会明显顿。
2. **减少同时在动的像素**：一次只让一个容器动，不要给整页做位移/缩放。
3. **不要给静止元素挂常驻的 `will-change` / `contain: paint`**：会把图层长期提升，每帧都要
   重新上传整个图层（实测帧率更低、更容易闪）。只在动画期间加，结束就摘掉。
4. 真要更高帧率，先试 `shm` 档；再不行只能降低渲染倍率（系统缩放 2× → 1.5×）。

## 四条错误的路（都试过，都没用，别再走）

这些方案在「合成状态变化 → 显示陈旧表面」这个根因面前必然失败，但过程中确实能观察到一些
现象，容易被误读成「有效果」：

1. **换动画属性**。transform / opacity / width / position 全试过，一样闪。
   于是给静止态补 `transform: translateY(0)`、`translateZ(0)`、`isolation`、`contain: paint` ——
   异常帧数从 22 降到 7，但**没消失**。
2. **换成 Web Animations API（`element.animate`）** + `fill: "both"`。这确实修掉了**另一个**
   问题（见下），但闪烁照旧。
3. **换动画库**（motion-v / framer-motion 的 Vue 端口）。库只负责「怎么算值」，不控制合成器
   怎么把图层交出去，同样照闪。
4. **逐帧调 CSS 时序**（缩短时长、换 easing、拆多段 keyframes）。只是把闪烁挪到别的时刻。

顺带一个**真实存在、值得修**的问题（与闪烁无关，但会被误认为闪烁）：

> **CSS `transition` 在 WebKitGTK 上不是逐帧推进的。** `get_snapshot` 1100×720 @2× 抓帧实测：
> 一个 190 ms 的 CSS transition 只产出 **3 个不同帧**、最大帧间隔 210 ms，还拖着重绘
> 21k–43k px；同一动画改用 `element.animate()` 后是 **10 个不同帧**、最大间隔 26 ms。
>
> 所以入场/位移动画应走 Web Animations API（或基于它的库），不要用 CSS `transition`。
> 这条修完动画会流畅，但闪烁仍在 —— 直到处理掉 DMABUF/合成器。

## 如何量化验证（如果你要自己复现）

不要靠眼睛，抓帧算术：

```bash
# 录屏（Wayland + KWin 下可用 obs / wf-recorder；KWin 不支持 wlr-screencopy 时用 obs）
# 然后逐帧 diff，找「跳变帧」和「相同帧连跑」
ffmpeg -i capture.mp4 -vf scale=426:240 -f rawvideo -pix_fmt rgb24 -
```

脚本简化版（逐帧算平均绝对差、亮度、变化像素数，标出 `diff > 3×均值` 的跳变帧，并统计
「最长相同帧连跑时长」）：

```
changed frames / mean diff / max diff / 最长相同帧连跑（ms）
```

- 闪烁 → 1–2 帧的孤立跳变（旧内容），中间夹着几乎静止的帧。
- 掉帧 → 大量帧完全相同（连跑 30–60 ms），最后突然跳到终点。

辅助手段：在页面里用 `requestAnimationFrame` 逐帧采样 `getComputedStyle` 的 `opacity` /
`transform`、`anim.currentTime`、`node.isConnected`，通过 `invoke` 打到 Rust 侧日志
（WebKitGTK 的 `console.log` 不写进任何日志文件，`WEBKIT_INSPECTOR_SERVER` 也不是 CDP，
不能当 Chrome DevTools 用 —— 只能走 `invoke`）。云语保留了 `src/lib/probe.ts` +
`#[cfg(debug_assertions)] debug_probe` 命令：

```ts
import { probeAnimation } from "@/lib/probe";
probeAnimation("pop", el, anim); // 每帧记 ct / op / tr / conn / dt
```

它能直接区分「动画值没推进」和「值推进了但屏幕上没画出来」：前者是 JS/引擎问题，后者才是
合成器问题。当年就是靠它证明「每帧 dt=16ms、ct 从 12 平滑到 190、末帧 op=1、图层仍连接」，
从而把怀疑从动画代码转到渲染器上。

## 三个项目的现状

| 项目 | 库名 | 开关变量 | 备注 |
| --- | --- | --- | --- |
| 云语聚合网盘 `yunyu-drive` | `yunyu_drive_lib` | `YUNYU_WEBKIT_RENDERER` | deb 由 Tauri bundler 出 |
| 4YMail `4yMail` | `fourym_lib` | `FOURYMAIL_WEBKIT_RENDERER` | 1.2.0 起对外文档化了这个变量，勿改名 |
| 小云 PC Agent `pc-agent` | `app_lib` | `YUYU_WEBKIT_RENDERER` | deb 由 `scripts/build-deb.sh` 调 `cargo tauri build --bundles deb` |

三份 `main.rs` 里的 `configure_webkitgtk_renderer()` 逻辑一致，只有库名和变量前缀不同；
三份 `docs/tauri-webkitgtk-flicker.md` 是同一份文件（改一处要同步另外两处）。

## 环境信息（复现参考）

| 项 | 值 |
| --- | --- |
| 桌面 | KDE Plasma，Wayland |
| 缩放 | 2×（屏 2133×1200 @scale 2 → 4266×2400） |
| WebKitGTK | `webkit2gtk-4.1` / `javascriptcoregtk-4.1`：2.54.0 → 2.54.3 均复现 |
| GPU | AMD（Mesa 26.1.6，RADV，`radeonsi`/ACO） |
| Tauri | 2.x，窗口 1220×800，`decorations: false` |

同机 Chromium 的同一个页面完全正常 —— 这就是前面那条「第一条判据」。
（Debian 侧：trixie 的 webkit2gtk 由 security 通道跨 series 滚动，2.52.x → 2.54.3；
2.54.3 只修了 24 个 CVE 和输入法/视频相关项，**没有任何图形修复**，别指望升级解决闪烁。）

## 复测清单（每次 WebKit 升版都跑一遍）

1. `TAURI_WEBKIT_RENDERER=native` → 还闪吗？不闪就说明上游修好了，可以删掉默认值。
2. `TAURI_WEBKIT_RENDERER=shm` → 闪吗？帧率多少？不闪且更顺就把默认切到 `shm`。
3. `TAURI_WEBKIT_RENDERER=norenderer` → 闪吗？帧率多少？（当前默认，作为对照基线）
4. 关注 WebKit bug 327524 是否 `RESOLVED FIXED`。

## 一句话总结

**动画闪烁 = WebKitGTK 2.54 的合成器/DMABUF 回归（上游 bug 327524，2.54.3 仍未修），
不是你的环境、不是 Tauri、更不是 CSS。** 应用侧用 `WEBKIT_DMABUF_RENDERER_FORCE_SHM=1`
（首选，待验证）或 `WEBKIT_DISABLE_DMABUF_RENDERER=1`（兜底，代价 30–40 fps）关掉那条路径；
开关必须设在进程启动前，且只影响 Linux —— macOS/Windows 用别的引擎，既不读这些变量，
也不会因此变成 CPU 渲染。
