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

`.github/workflows/build.yml` 是唯一的三平台构建入口。Pull Request 只运行前端检查和 Windows 上的完整 Rust 单元测试；独立的 `.github/workflows/layout.yml` 使用与项目 Playwright 版本一致的官方预装镜像，在同一个 job 中顺序运行 Chromium 全套布局测试、WebKit 的 relationships/responsive 用例和 XCPC 用例（XCPC 允许两次重试）。两个工作流分开保留，避免更改已有检查项名称，合并文件也不会缩短三平台打包时间。`v*` 标签和手动触发会构建三平台安装包。在 Actions 页面手动运行时，`release_tag` 留空表示普通测试构建；填写与源码一致的版本号（如 `v0.10.6`）会生成签名更新包和 `latest.json`，并创建等待人工确认的 Draft Release。

三端打包开始前会先执行前端类型与版本检查，及完整 Rust 单元测试。Pull Request 有新提交时，尚未完成的旧检查会自动取消，避免重复占用资源。需要在合并前检查三平台安装包时，可手动触发普通测试构建。

## 日常开发

先阅读 [贡献指南](../CONTRIBUTING.md)。本地仓库可放在任意目录；以下命令均在仓库根目录执行。

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

`pnpm tauri dev` 启动前端和桌面应用，需先满足对应系统依赖。只查看前端可运行 `pnpm dev`，但普通浏览器没有真实 Tauri IPC；涉及本地文件、数据库、独立窗口和更新时应在桌面验证，或明确使用测试 mock。

Windows 若出现 Git dubious ownership，应核对仓库属于你并仅信任该目录；不要设置全局通配符。PowerShell 示例：

```powershell
$repo = 'C:\OJ_Insight'
$safe = $repo.Replace('\', '/')
git -c "safe.directory=$safe" -C $repo status
```

## 按改动选择验证

| 改动 | 必要验证 |
| --- | --- |
| 纯文档 | 内容与当前实现一致、相对链接有效、`git diff --check`；无需为文档重新打包三端 |
| 前端逻辑或类型 | `pnpm check`；影响生产构建时运行 `pnpm build`；补充或运行受影响的交互测试 |
| 布局、导航和反馈 | 受影响的 Playwright 用例及截图核对；覆盖主题、字号、窗口和键盘交互，关注正反馈是否仍触发 |
| Rust、同步、统计与持久化 | `cargo test --locked --manifest-path src-tauri/Cargo.toml`，检查旧数据兼容、失败和缓存路径 |
| 桌面窗口、文件或更新 | 除相关测试外，在受影响平台验证实际桌面行为，不能只依赖浏览器 mock |
| 发布 | 执行下文完整发布检查，生成并验证三平台产物 |

针对性布局测试示例：

```bash
pnpm exec playwright test --config config/playwright.config.ts tests/layout/navigation.spec.ts --project=chromium
```

Rust 本地编译若提示缺少 Tauri 前端资源，先运行 `pnpm build` 生成 `dist/`。CI 的 Rust 测试使用最小前端占位文件，不代表完成了前端生产构建。

如果本机缺少工具链或无法运行浏览器，应记录实际限制，并在 PR 中明确未完成的验证。不要把列出测试用例、类型检查或 mock 结果写成真实桌面流程通过。CI 仍按工作流运行其配置的检查；本表不改变 CI 门禁。

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

输出必须同时包含 `x86_64` 与 `arm64`。当前 `tauri.conf.json` 中的最低目标系统为 macOS 11。

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

## 工具配置

前端工具配置集中在 `config/`：

- `config/vite.config.ts`
- `config/tsconfig.json`
- `config/playwright.config.ts`

通过 `package.json` 中的脚本执行开发、检查、构建和布局测试；脚本已传入相应配置路径。

## 发布检查

布局回归可在本地运行：

```bash
pnpm install --frozen-lockfile
pnpm exec playwright install chromium webkit
pnpm test:layout
```

Linux 首次安装浏览器时可使用 `pnpm exec playwright install --with-deps chromium webkit` 安装系统依赖。测试启动本地 Vite 服务，并模拟 Tauri 数据和第三方 Tracker 页面；不会同步账号或请求真实 Tracker。覆盖 1080p / 2K / 4K、150% / 200% DPI、窗口缩放、侧栏折叠、半年活动砖、大字号及方格点击。Chromium / WebKit 测试用于检查渲染兼容性，桌面平台打包仍由三平台构建验证。

缓存命中、过期检查、损坏恢复、断网回退、强制刷新和榜单请求并发上限包含在 `cargo test --locked --manifest-path src-tauri/Cargo.toml` 中。需要单独访问真实公共源时，可运行：

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml live_public_metadata_cache_reuses_downloads -- --ignored --nocapture
```

1. `package.json`、`src/lib/version.ts`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 中 OJI 自身的条目与 `src-tauri/tauri.conf.json` 版本一致；同步更新中英文 README、发布文档示例、更新记录及测试中的当前版本。历史版本和第三方依赖版本不批量替换。`pnpm check:version` 当前只核对四个主要配置文件，锁文件与文档需另外核对。
2. `pnpm check`、前端生产构建及 Rust 测试通过。
3. Windows、MacOS Universal 和 Linux 三个平台产物均已生成。
4. MacOS `lipo` 检查包含两个架构。
5. 三档字号、亮色/灰色/暗色主题、最小窗口、未评级难度和同步部分成功状态通过检查。
6. `OJ-Insight-All-Platforms.zip` 根目录只包含 `.exe`、`.dmg` 与 `.AppImage` 各一个。

## 签名更新

Tauri updater 使用独立的更新签名密钥。私钥不能提交到仓库；将私钥内容配置为 GitHub Actions Secret `TAURI_SIGNING_PRIVATE_KEY`，如密钥有密码，再配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。公钥已经写入 `src-tauri/tauri.conf.json`。

在 Actions 页面填写 `release_tag`，或推送 `v*` 标签后，CI 会生成 EXE、Universal DMG、AppImage 三种用户安装包，同时生成 updater 内部需要的签名文件、MacOS 更新归档和 `latest.json`，并自动创建 Draft Release。检查安装包后需在 Releases 页面手动发布。丢失私钥后，已经安装的客户端将无法验证后续更新，因此必须离线备份。
