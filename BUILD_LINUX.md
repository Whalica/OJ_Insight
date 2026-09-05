# Linux 构建说明

OJ Insight 使用 Tauri 2 构建 Linux 桌面应用，GitHub Actions 会分别生成 AppImage、DEB 与 RPM。

## GitHub Actions 构建

1. 打开仓库 `Actions`。
2. 选择 `Build desktop apps`。
3. 点击 `Run workflow`。
4. 全部 job 完成后下载 `OJ-Insight-All-Platforms` artifact，解压其中的 `OJ-Insight-Linux/` 即可取得 AppImage、DEB 与 RPM；同一总包还包含 Windows 与 macOS 目录。

工作流也会在推送 `v*` tag 或发起 Pull Request 时运行。

## Ubuntu / Debian 本机构建

先安装 Node.js 22+、Rust stable，以及 Tauri 所需系统依赖：

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential curl file libayatana-appindicator3-dev librsvg2-dev \
  libssl-dev libwebkit2gtk-4.1-dev libxdo-dev patchelf wget
```

然后执行：

```bash
npm install
npm run tauri build
```

产物通常位于：

```text
src-tauri/target/release/bundle/appimage/
src-tauri/target/release/bundle/deb/
src-tauri/target/release/bundle/rpm/
```

## Linux 数据目录

Linux 安装目录通常不可写，因此应用使用当前用户的数据目录，通常为：

```text
~/.local/share/com.ojinsight.app/
```

其中包含 `data/`、`exports/`、`logs/` 与 `webview/`。准确路径以应用「关于」页面显示为准。

## Arch Linux / niri / Wayland 的 EGL_BAD_ALLOC

OJ Insight 使用 Tauri + GTK/WebKitGTK，不是 Electron。Electron 的 Ozone 和 --disable-gpu 参数不控制此应用的图形初始化。

v0.5 在创建 WebView 前对 AppImage + Wayland 尝试以下兼容策略：

- 使用 GDK_BACKEND=wayland,x11，避免打包启动钩子强制 X11；需要固定后端时使用下面的应用专用变量。
- 默认设置 WEBKIT_DISABLE_DMABUF_RENDERER=1（未被用户显式设置时）。
- 检测 niri（包括 NIRI_SOCKET）时，默认尝试 WEBKIT_DISABLE_COMPOSITING_MODE=1。
- 显式软件渲染选项优先于启用 GPU。这些选项是规避路径，不保证能够解决所有 EGL、驱动或 AppImage 图形库冲突。

可分别测试：

```bash
# 默认兼容路径
./'OJ Insight_0.5.0_amd64.AppImage'

# 显式请求软件合成
OJ_INSIGHT_SOFTWARE_RENDERING=1 ./'OJ Insight_0.5.0_amd64.AppImage'

# 分别验证原生 Wayland 和 XWayland
OJ_INSIGHT_GDK_BACKEND=wayland ./'OJ Insight_0.5.0_amd64.AppImage'
OJ_INSIGHT_GDK_BACKEND=x11 ./'OJ Insight_0.5.0_amd64.AppImage'

# 对照测试：允许 niri 的硬件合成，并明确恢复 DMA-BUF
OJ_INSIGHT_ENABLE_GPU=1 WEBKIT_DISABLE_DMABUF_RENDERER=0 WEBKIT_DISABLE_COMPOSITING_MODE=0 ./'OJ Insight_0.5.0_amd64.AppImage'
```

若仍退出，请提供显卡型号、驱动/Mesa 版本、webkit2gtk-4.1/gtk3 版本、niri 版本，以及上述四种路径的终端错误。不要上传 Cookie 或完整账号数据库。还应对比使用系统 WebKitGTK 构建的原生二进制与 AppImage：前者正常而后者失败才进一步检查打包库兼容性。不要直接删除打包库或强制替换系统库。

目前没有在该报告的 Arch+niri+显卡环境上实测，不能将此 issue 标记为已验证解决。

参考上游：[AppImage GTK 后端覆盖问题](https://github.com/tauri-apps/tauri/issues/15781)、[WebKitGTK DMA-BUF/EGL 相关报告](https://github.com/tauri-apps/wry/issues/1366)、[合成兼容性报告](https://github.com/tauri-apps/tauri/issues/9394)。

## 发布前验收

- 原有 CI 编译：Windows、macOS、Ubuntu；另执行 Rust 回归测试。
- Linux 启动：X11、GNOME/KDE Wayland、Arch+niri；记录 GPU/驱动和实际显示后端。
- AppImage、DEB/RPM 与源码构建对照；启动、保存账号、同步、导出和重启。
- 三种主题及三档字号（含系统切换时应用已打开的情况）。
