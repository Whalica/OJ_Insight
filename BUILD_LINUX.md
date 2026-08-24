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
