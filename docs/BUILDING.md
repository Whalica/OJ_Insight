# 构建与发布

OJ Insight 是 Tauri 2 + React 桌面应用。最终用户不需要启动本地服务器或保留 Node.js 进程。

## 通用要求

- Node.js 22+
- Rust stable
- 当前仓库的前端依赖

```bash
npm install
npm run check
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

`.github/workflows/build.yml` 是唯一的三平台构建入口。工作流在 Pull Request、`v*` 标签和手动触发时运行，并为发布文件生成 SHA-256 校验清单。

## Windows

额外安装 Microsoft Visual Studio Build Tools（Desktop development with C++）和 WebView2 Runtime，然后执行：

```powershell
npm run tauri build
```

安装包位于 `src-tauri/target/release/bundle/`。Windows 版使用便携数据目录，程序所在文件夹必须可写：

```text
OJ Insight/
├─ OJ Insight.exe
├─ data/oj-insight.sqlite3
├─ exports/
├─ logs/oj-insight.log
└─ webview/
```

Release 构建使用 Windows GUI subsystem，不会额外弹出控制台窗口。

## macOS

额外安装 Xcode Command Line Tools。正式发布使用 Universal target，同时包含 Intel `x86_64` 与 Apple Silicon `arm64`：

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run tauri build -- --target universal-apple-darwin
```

产物位于 `src-tauri/target/universal-apple-darwin/release/bundle/`。发布前检查主程序：

```bash
lipo -archs "src-tauri/target/universal-apple-darwin/release/bundle/macos/OJ Insight.app/Contents/MacOS/oj-insight"
```

输出必须同时包含 `x86_64` 与 `arm64`。v0.5.1 的最低目标系统为 macOS 11。

macOS 数据保存在：

```text
~/Library/Application Support/com.ojinsight.app/
```

### macOS 常见提示

- “这台 Mac 不支持此应用程序”：通常是下载了错误 CPU 架构的包。优先下载文件名包含 `universal` 的 v0.5.1 或更高版本。
- “无法验证开发者”：这是签名或公证提示，不是架构不兼容。在 Finder 中右键应用并选择“打开”，或在“系统设置 → 隐私与安全性”中允许。
- 未签名测试包仍可能被 Gatekeeper 阻止。稳定发布应完成 Developer ID 签名与 Apple notarization。

## Linux

Ubuntu 24.04 构建依赖：

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential curl file libayatana-appindicator3-dev librsvg2-dev \
  libssl-dev libwebkit2gtk-4.1-dev libxdo-dev patchelf wget
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/{appimage,deb,rpm}/`。数据通常位于：

```text
~/.local/share/com.ojinsight.app/
```

### Arch / niri / Wayland

OJ Insight 使用 GTK/WebKitGTK，不使用 Electron 参数。遇到 `EGL_BAD_ALLOC` 时可分别测试：

```bash
OJ_INSIGHT_SOFTWARE_RENDERING=1 ./OJ\ Insight_*.AppImage
OJ_INSIGHT_GDK_BACKEND=wayland ./OJ\ Insight_*.AppImage
OJ_INSIGHT_GDK_BACKEND=x11 ./OJ\ Insight_*.AppImage
```

仍然失败时，请记录显卡、Mesa/驱动、WebKitGTK、GTK 与 niri 版本，并对比系统原生构建和 AppImage；不要上传 Cookie 或完整数据库。

## 发布检查

1. `package.json`、`src/lib/version.ts`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 版本一致。
2. `npm run check`、前端生产构建及 Rust 测试通过。
3. Windows、macOS Universal 和 Linux 三个平台产物均已生成。
4. macOS `lipo` 检查包含两个架构。
5. 三档字号、亮暗主题、最小窗口和同步部分成功状态通过检查。
6. 发布文件附带 SHA-256 校验清单。
