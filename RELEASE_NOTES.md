# OJ Insight 0.2.0 — Career & Activity

Buildfix: 修复 LeetCode provider 中局部变量遮蔽 `calendar_node()` 函数导致的 Rust E0618 编译错误。

## Buildfix 2

- 修复 LeetCode 中国站把 `userCalendar` 错误放在根 Query 导致的 HTTP 400，改用 `matchedUser.userCalendar`。
- LeetCode CN 难度统计优先使用当前 `userProfileUserQuestionProgress`，并兼容 V2 operation。
- 修复切换到 AtCoder / 洛谷单 OJ 页面时 Difficulty Profile 读取空分组导致的前端黑屏。
- Difficulty Profile 恢复 Preview 的绿色紧凑 histogram 风格，并增加中位难度、峰值区间、最高难度和有难度题目摘要。
- 关键功能标题、统计项、同步状态和 About 操作改为中文，英文仅保留为辅助装饰。

## Buildfix 3

- 修正 LeetCode CN schema 判断：标准 `/graphql/` 不支持国际站的 `matchedUser/userCalendar`。
- 中国站日历改为在独立 `/graphql/noj-go/` schema 中尝试。
- 若中国站当前不再公开 Activity 日历，同步不再整体失败：解题总数与难度照常更新，并保留已有日期缓存。
- Data Sources 会以“同步成功 + Activity 暂不可用”明确展示降级状态。

## Buildfix 4

- 删除 LeetCode 中国站所有 `matchedUser/userCalendar` 请求，不再猜测中国站日历 schema。
- 中国站只调用自己的用户进度接口，同步解题总数和 Easy / Medium / Hard；Activity 暂不可用不会再导致同步失败。
- 保留已有中国站日期缓存，不用空结果覆盖历史数据。

## Providers

- LeetCode 国际站与中国站拆成独立 GraphQL provider，修复 `leetcode.cn HTTP 400` 路径，并在失败时显示 operation/HTTP/响应摘要。
- QOJ 支持完整 `UOJSESSID=value` 或只填 value 自动补齐。
- QOJ 解析改为根据 submission/problem 链接与整行语义识别当前表格，不再依赖固定列号。
- QOJ 分别报告未登录/过期、确实无提交、页面结构变化和网络错误。

## Analytics & UI

- 总览与单 OJ 都增加独立 Career 统计。
- 当前年份/Until now 统计与 Career 分离。
- 年份控件更新为 `< [ year ▼ ] >`；Until now 固定最近 365 天并以今天为 Activity 右边界。
- UI 统一使用 Activity。
- 新增/完善 Platform Summary、Recent Accepted、Data Sources 与逐站同步进度。
- Difficulty Profile 改成分平台 histogram，保留平台自身难度体系。
- 缓存记录、Last successful 与 Latest sync error 分开显示。

## Desktop & release

- 新增 About 页、GitHub Releases 更新检查、Repository 与 Report an Issue。
- Release 使用 Windows GUI subsystem；应用功能不启动 console 子进程。
- 新增根目录 `logs/`，同步日志自动脱敏 QOJ Cookie/Secret。
- Activity 导出支持年份区间/Until now、All OJs/单 OJ、PNG/SVG。
- 版本统一更新到 0.2.0。
- 保留 Rust MutexGuard 显式解引用编译修复和 Actions checkout/setup-node v5。
- tag workflow 可自动创建 GitHub Release 并上传 NSIS/MSI。

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
