# OJ Insight 0.5.0 — Rating、个性化与多账号一致性（待发布）

## 功能与界面

- 保留既有侧栏、今日进度、生涯统计、活动砖、难度足迹、难度分布、平台列表、最近 AC、导出与数据源页面。
- 在生涯统计下增加 Rating；总览按 OJ 切换，平台页展示本平台，多个有 Rating 的 ID 可单独查看。
- 展示当前、历史最高、最近一场变化、Rated 场次、最近 Rated 比赛与历史曲线；不再使用“近 90 天涨幅”和“峰值日期”卡片。曲线范围单独切换。
- 接入 Codeforces、AtCoder Algorithm、LeetCode 国际站历史。LeetCode 中国站及其他 OJ 暂未接入，不以 0 冒充缺失数据。LeetCode 只有一场记录时不推测初始涨幅。
- 设置分为账号设置和个性化。主题（亮/暗/跟随系统）、字号（标准/放大/特大）、间距、活动砖配色、减少动效、统计时区、启动页及恢复默认值。
- 统计时间范围和统计口径仍保留在总览/各 OJ 页面。未增加赛后复盘模块。

## 数据一致性

- 删除、清空或改名 ID 时，在同一事务中清除原 ID 的提交、活动、难度、Rating、统计及同步游标，保留其他 ID 和其他平台。
- 保存全部平台的账号改为一次事务；任何平台失败时不产生半保存状态。保存前明确提示将清理哪些 ID。
- 启动时清理旧版遗留的无主账号缓存；旧单账号缓存只在首次迁移时导入，避免重启后回流。
- 提交记录主键包含账号，避免不同账号/站点相同 submission ID 相互覆盖；列表键也同步修复。
- 同步与账号保存/清空互斥；入库前校验账号归属，避免迟到的同步结果复活已删除记录。
- 清空平台/全部数据采用事务；中断的同步状态在重启后恢复为待同步。
- 保存/清空后刷新统计、同步状态与详情；切换筛选或关闭抽屉后，旧请求不再覆盖新内容。
- Rating 独立请求；失败或返回不完整数据时保留缓存，并显示未更新状态。多账号部分失败不再被最后一个成功账号掩盖。

## Linux 与验证

- 针对 AppImage + Wayland 增加 WebKitGTK 启动兼容设置；niri 默认尝试关闭合成，并提供图形后端和软件渲染覆盖选项。不是 Electron 参数。
- 这属于兼容性缓解，尚未在报告问题的 Arch/niri/显卡组合上复现并验证，不能据此宣称 EGL_BAD_ALLOC 已彻底修复。排查步骤见 BUILD_LINUX.md。
- 增加数据库与同步互斥的 Rust 回归用例；保留 GitHub 当前的三平台工作流与 npm 构建方式。独立 Node/SQL 测试仅用于开发验证，不随此源码包附带。
- 本地已通过 TypeScript 检查、Vite 生产构建、7 项 Node/SQL 测试。当前环境无 Rust 工具链，因此 Rust 测试、三平台安装包及原生 WebView 交互仍待验证。

升级前请退出程序并备份整个应用数据目录。被删除 ID 的记录会从应用数据库中移除，重新添加需重新同步；旧版已错误归属到保留账号的数据无法安全猜测归属，请对受影响平台重建。历史日志和已导出的图片是独立文件，本次不会删除。

# OJ Insight 0.4.0 — Time Zone & Reliable Sources

## 统计与界面

- 新增统计时区选择，今日进度、问候语、活动砖、难度足迹、连续打卡与零点换日统一按所选时区计算。
- “至今”明确改为“至今（近一年）”；生涯累计移动到今日进度与时间范围之间。
- 难度统计改为生涯去重题目数：同一账号同一题只计一次；未完成的 0 项不显示，柱间距随实际分级数量变化。
- Codeforces 难度图隐藏系统白色滚动条；AtCoder 低分 difficulty 使用官方显示换算且不再出现负分。
- 最近 AC 不受当前年份筛选影响；Tracker 仅有日期时明确显示“来源日期”。

## 数据源

- Luogu 从练习题单补齐已通过题目的最新八级难度，用于提交与难度足迹；提交接口不可用时仍安全保留 `dailyCounts`。
- NowCoder 保留普通 OJ 提交，并支持用可选 Cookie 读取 Tracker 完成日；优先匹配真实提交，无法匹配时保存不伪造时间的日期记录。
- LeetCode 国际站日历保留原始 epoch 后按统计时区换算；中国站新增独立活动日历、最近 AC 与可选 Cookie 回退，接口失败不清空旧缓存。

## 发布

- 版本统一更新到 0.4.0。
- 统一 workflow 一次并行构建 Windows、macOS、Linux，最终额外产出 `OJ-Insight-All-Platforms.zip` 与 SHA-256 校验清单。

# OJ Insight 0.3.0 — Multi-account & Difficulty Footprint

## UI 与统计

- 首页固定展示六个 OJ 的今日进度，问候语根据本地时段切换。
- 每个 OJ 页面同时提供活动砖、难度足迹和独立难度分布。
- 活动砖继续支持点击查看当天全部提交；最近 AC 增加显眼的题目跳转按钮。
- Codeforces 按官方颜色逐个展示 800～3500 rating；Luogu 使用最新八级难度名称与颜色。
- 时间范围和统计口径会记住上一次选择，并按 UTC+8 在零点立即补出今天的格子。

## 账号与同步

- 每个平台支持多个用户 ID，同时提供聚合与单账号筛选。
- 修复同步有新增记录但界面显示新增 0 的问题。
- 全部同步时只展示一条随机竞赛 Tips。
- AtCoder 使用真实提交时间并扩大重叠窗口；Luogu 优先读取提交记录；NowCoder 同时统计普通题目与每日一题 Tracker。
- LeetCode 国际站同步最近 AC，页面补充活动和难度数据；中国站继续按公开接口安全降级。

## 桌面与发布

- 版本统一更新到 0.3.0。
- `.github/workflows/build.yml` 并行构建 Windows、macOS 与 Linux，并分别上传三个 artifact。
- Linux 数据改为写入用户应用数据目录，避免安装目录不可写。
- Issue 反馈入口直接打开项目 GitHub Issues。

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
