# 构建与发布

OJ Insight 是 Tauri 2 + React 桌面应用。最终用户不需要启动本地服务器或保留 Node.js 进程。

## 通用要求

- Node.js 22+
- Rust stable
- pnpm 11+

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

`.github/workflows/build.yml` 是唯一的三平台构建入口。工作流在 Pull Request、`v*` 标签和手动触发时运行。在 Actions 页面手动运行时，`release_tag` 留空表示普通测试构建；填写与源码一致的版本号（如 `v0.7.1`）会生成签名更新包和 `latest.json`，并创建等待人工确认的 Draft Release。

## Windows

额外安装 Microsoft Visual Studio Build Tools（Desktop development with C++）和 WebView2 Runtime，然后执行：

```powershell
pnpm tauri build
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

## MacOS

额外安装 Xcode Command Line Tools。正式发布使用 Universal target，同时包含 Intel `x86_64` 与 Apple Silicon `arm64`：

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
pnpm tauri build --target universal-apple-darwin
```

产物位于 `src-tauri/target/universal-apple-darwin/release/bundle/`。发布前检查主程序：

```bash
lipo -archs "src-tauri/target/universal-apple-darwin/release/bundle/macos/OJ Insight.app/Contents/MacOS/oj-insight"
```

输出必须同时包含 `x86_64` 与 `arm64`。v0.7.1 的最低目标系统为 MacOS 11。

MacOS 数据保存在：

```text
~/Library/Application Support/com.ojinsight.app/
```

### MacOS 常见提示

- “这台 Mac 不支持此应用程序”：通常是下载了错误 CPU 架构的包。优先下载文件名包含 `universal` 的 v0.7.1 或更高版本。
- “无法验证开发者”：这是签名或公证提示，不是架构不兼容。在 Finder 中右键应用并选择“打开”，或在“系统设置 → 隐私与安全性”中允许。
- “App 已损坏，无法打开”：开源未公证构建被 Gatekeeper 加上隔离标记时也会出现，并不代表程序文件实际损坏。确认安装包来自本项目 Release，保持 DMG 已挂载，然后对 DMG 中的实际 App 路径执行：

```bash
xattr -cr "/Volumes/OJ Insight/OJ Insight.app"
```

  DMG 卷名或 App 路径不同时应使用 Finder 中看到的实际路径。只对可信来源下载的软件执行此命令。
- CI 会对社区构建执行临时签名和完整性检查，但这不能替代 Apple Developer ID 签名与 notarization；配置正式证书后应以签名、公证彻底消除 Gatekeeper 提示。

## Linux

Ubuntu 24.04 构建依赖：

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential curl file libayatana-appindicator3-dev librsvg2-dev \
  libssl-dev libwebkit2gtk-4.1-dev libxdo-dev patchelf wget
pnpm tauri build
```

面向用户的产物为 `src-tauri/target/release/bundle/appimage/` 中的 AppImage。数据通常位于：

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
2. `pnpm check`、前端生产构建及 Rust 测试通过。
3. Windows、MacOS Universal 和 Linux 三个平台产物均已生成。
4. MacOS `lipo` 检查包含两个架构。
5. 三档字号、亮色/灰色/暗色主题、最小窗口、未评级难度和同步部分成功状态通过检查。
6. `OJ-Insight-All-Platforms.zip` 根目录只包含 `.exe`、`.dmg` 与 `.AppImage` 各一个。

## 签名更新

Tauri updater 使用独立的更新签名密钥。私钥不能提交到仓库；将私钥内容配置为 GitHub Actions Secret `TAURI_SIGNING_PRIVATE_KEY`，如密钥有密码，再配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。公钥已经写入 `src-tauri/tauri.conf.json`。

在 Actions 页面填写 `release_tag`，或推送 `v*` 标签后，CI 会生成 EXE、Universal DMG、AppImage 三种用户安装包，同时生成 updater 内部需要的签名文件、MacOS 更新归档和 `latest.json`，并自动创建 Draft Release。检查安装包后需在 Releases 页面手动发布。丢失私钥后，已经安装的客户端将无法验证后续更新，因此必须离线备份。
