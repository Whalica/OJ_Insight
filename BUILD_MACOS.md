# macOS 构建说明

OJ Insight 是 Tauri 2 桌面应用。自本版本起支持 macOS（Apple Silicon 与 Intel），界面使用系统自带的 WKWebView，无需额外运行时。当前 CI 在 `macos-latest` 上构建，产物架构跟随 runner；Intel Mac 可按下方说明本机构建。

## 最省事：GitHub Actions 构建

1. 打开仓库 `Actions`。
2. 选择 `Build macOS`。
3. 点击 `Run workflow`。
4. 完成后下载 `OJ-Insight-macOS` artifact；推送 `v*` tag 时会触发 `Release` workflow 自动创建 GitHub Release 并上传 DMG 与 `.app.zip`。
5. 解压后可取得当前 runner 架构的 `OJ Insight_<version>_<arch>.dmg`（如 `aarch64` 或 `x64`）与打包好的 `OJ Insight.app.zip`。

## 本机开发构建

需要先安装：

- Node.js 22+
- Rust stable（`rustup` 或 `brew install rust`）
- Xcode Command Line Tools（`xcode-select --install`）

执行：

```bash
npm install
npm run tauri build
```

产物位于：

```text
src-tauri/target/release/bundle/macos/OJ Insight.app
src-tauri/target/release/bundle/dmg/OJ Insight_<version>_<arch>.dmg
```

开发模式（热更新）：

```bash
npm install
npm run tauri dev
```

## macOS 数据目录

与 Windows 的便携式布局不同，macOS 应用包（`.app`）是只读的，且首次运行隔离下载的应用时会被 Gatekeeper 转移到随机临时路径。因此 macOS 版把所有持久数据放在用户应用支持目录：

```text
~/Library/Application Support/com.ojinsight.app/
├─ data/oj-insight.sqlite3
├─ exports/
├─ logs/oj-insight.log
└─ webview/
```

「设置 → 数据」页面展示的 Storage 路径即该目录。备份或迁移时复制整个 `com.ojinsight.app` 目录即可。

## 首次打开与 Gatekeeper

- 通过 DMG 安装后直接运行即可。
- 如果从浏览器下载的 `.app` 被 Gatekeeper 拦截（“无法验证开发者”），在 Finder 中右键 → 打开，或在「系统设置 → 隐私与安全性」中允许。
- 未签名 CI 产物在部分机器上可能需要执行 `xattr -dr com.apple.quarantine "/Applications/OJ Insight.app"` 后再运行。
