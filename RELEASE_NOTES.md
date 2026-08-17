# OJ Insight 0.1.1 — Portable Data Layout

本次更新重点是把 OJ Insight 改成真正的**便携式数据布局**。

## 本次变更

- SQLite 不再写入 Tauri 的系统 AppData 目录。
- 所有持久化应用数据统一放在 `OJ Insight.exe` 所在目录下。
- 首次运行自动创建：
  - `data/`
  - `exports/`
  - `webview/`
- Windows WebView2 的 localStorage / cache 目录也固定到根目录 `webview/`，不再使用默认 AppData 数据目录。
- 主数据库固定为：

```text
data/oj-insight.sqlite3
```

- 账号配置、同步游标、提交缓存、统计数据与 QOJ Cookie 均跟随该数据库移动。
- 导出窗口默认定位到根目录下的 `exports/`。
- 新增 `get_storage_info` Tauri command，前端可以查询当前根目录、数据库路径和默认导出目录。
- 版本号更新到 `0.1.1`。
- README 重写为完整使用手册，覆盖首次使用、六个平台账号配置、同步、统计、导出、清空、备份、迁移、更新与常见问题。

## 目录示例

```text
OJ Insight/
├─ OJ Insight.exe
├─ data/
│  └─ oj-insight.sqlite3
├─ exports/
└─ webview/
```

复制整个目录即可连同数据一起迁移。

## 注意

程序所在目录必须允许当前用户写入。不要把便携版放进受系统保护且不可写的目录。

# OJ Insight 0.1.0 — Desktop Foundation

第一版桌面化里程碑。

## 已完成

- Tauri 2 + React + TypeScript 桌面 UI。
- Rust 后端 + SQLite 本地数据层。
- Codeforces / AtCoder / 洛谷 / 牛客 / QOJ / LeetCode 六个平台适配器。
- LeetCode 国际站与中国站（`cn:用户名`）切换。
- 总览与单 OJ 独立数据页面。
- 年度砖墙以及首次 AC / 当日去重 AC / AC 提交 / 平台活动四种口径。
- 活跃日、最长连续、当前连续、峰值日、解题量与难度统计。
- 单日详情。
- 增量同步与全量重建。
- 清空单个平台 / 清空全部本地记录（保留账号配置）。
- 数据源状态与错误信息。
- 2010 至今任意整年范围的总图 / 单 OJ 图导出。
- PNG / SVG 导出。
- GitHub Actions Windows 构建工作流。
