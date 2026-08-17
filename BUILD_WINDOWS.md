# Windows 构建说明

OJ Insight 是 Tauri 2 桌面应用，不再启动 localhost 服务，也不需要最终用户在日常使用时保留 Node 进程。

当前版本使用**便携式数据布局**：数据库位于 `OJ Insight.exe` 同级的 `data/` 中，因此运行目录必须可写。

## 最省事：GitHub Actions 构建

1. 把整个 `oj-insight` 目录上传到一个 GitHub 仓库。
2. 打开仓库 `Actions`。
3. 选择 `Build Windows`。
4. 点击 `Run workflow`。
5. 完成后下载 `OJ-Insight-Windows` artifact；推送 `v*` tag 时 workflow 还会自动创建 Release。
6. 解压后可取得 Windows 构建产物 / 安装包。

## 本机开发构建

需要先安装：

- Node.js 22+
- Rust stable (`rustup`)
- Microsoft Visual Studio Build Tools，勾选 Desktop development with C++
- WebView2 Runtime

执行：

```powershell
npm install
npm run tauri build
```

正式构建主程序通常位于：

```text
src-tauri\target\release\
```

安装包位于：

```text
src-tauri\target\release\bundle\
```

## 便携版数据目录

假设把主程序放到：

```text
D:\Tools\OJ Insight\OJ Insight.exe
```

首次启动后会自动创建：

```text
D:\Tools\OJ Insight\data\oj-insight.sqlite3
D:\Tools\OJ Insight\exports\
D:\Tools\OJ Insight\logs\oj-insight.log
```

复制整个 `D:\Tools\OJ Insight\` 即可一起迁移程序和数据。

## Release 无黑框

`src-tauri/src/main.rs` 在非 debug 构建启用 Windows GUI subsystem。当前同步、更新检查和导出均由应用进程内部完成，不调用 PowerShell/cmd。Debug 版本可保留 console 以便开发排错。
