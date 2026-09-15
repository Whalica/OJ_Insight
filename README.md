# OJ Insight

**把分散在多个 Online Judge 的训练记录，整理成一份可信、清晰、可长期追踪的个人档案。**

OJ Insight 是一个面向算法竞赛选手的跨平台训练数据面板。目前支持 Codeforces、AtCoder、洛谷、牛客、QOJ 和 LeetCode，并提供独立的 XCPC Tracker。所有数据保存在本地，应用会明确区分逐题记录、公开汇总和缺失数据，不用不可比的口径拼出“看起来完整”的统计。

[下载最新版本](https://github.com/Whalica/OJ_Insight/releases/latest) · [查看更新记录](docs/UPDATEINFO.md) · [反馈问题](https://github.com/Whalica/OJ_Insight/issues) · [源码构建](docs/BUILDING.md)

当前版本：**v0.8.1**，支持 Windows、MacOS 和 Linux。

## 为什么使用 OJ Insight

### 一个面板查看六个 OJ

不再分别打开多个个人主页。OJ Insight 将不同平台的账号、训练活动、解题数、难度分布和 Rating 变化集中展示，并支持同一平台配置多个账号。

### 统计全面，但不牺牲准确性

不同 OJ 开放的数据并不相同。OJ Insight 会区分：

- 可验证的逐题 AC 记录；
- 平台只提供的日期活动或题量汇总；
- 暂时不可获取的数据。

缺少逐题历史时不会伪造提交，Rating 和难度也不会跨平台强行换算。同步失败只更新错误状态，已经缓存的数据仍可继续查看。

### 从日常训练延伸到 XCPC 补题

XCPC Tracker 汇总 ICPC、CCPC 和省赛题集，可按年份、阶段、赛站、系列和完成进度筛选。题目完成状态来自本地 QOJ 记录，公开榜单可用时还会显示金、银、铜、铁难度层级。

### 本地保存，方便迁移

账号设置、同步结果、训练记录和导出文件都保存在应用自己的数据目录中。无需注册 OJ Insight 账号，复制数据目录即可备份或迁移。

## 你可以看到什么

- **生涯与区间统计**：Solved、AC Submissions、Active Days、最长连续训练、当前连续训练和单日峰值。
- **活动砖**：按自然年或最近 365 天查看 First AC、Unique AC、AC Submissions 和平台原始 Activity。
- **难度足迹**：保留各 OJ 自己的难度体系，点击柱形或日期可查看对应题目。
- **Rating 总览**：查看当前 Rating、历史最高、最近变化和比赛曲线，并可直接打开对应比赛。
- **近期记录**：集中浏览最近 AC，点击即可跳转题面。
- **XCPC Tracker**：按比赛追踪补题进度，并结合公开榜单观察题目层级。
- **图片导出**：按年份、统计口径和平台导出 PNG 或 SVG 活动图。

## 平台支持

| 平台 | 账号填写 | 主要可用数据 | 额外说明 |
|---|---|---|---|
| Codeforces | Handle | 逐题 AC、难度、Rating | 无需 Cookie |
| AtCoder | 用户名 | 逐题 AC、难度、Algorithm Rating | 公共题目元数据会在本地缓存 |
| 洛谷 | 用户名或数字 UID | 提交或公开活动、题量、官方难度 | 接口受限时安全降级为汇总数据 |
| 牛客 | 数字 User ID | 普通题 AC、Tracker 完成记录 | Tracker 数据可选填 Cookie |
| QOJ | 用户名 | 逐题 AC、XCPC 补题进度 | 完整提交列表需要 `UOJSESSID` |
| LeetCode | 用户名或 `cn:用户名` | 活动、题量、难度；国际站 Rating | 中国站部分接口可选填 Cookie |

上游网站可能调整接口或限制访问，因此同一平台在不同时间可取得的数据粒度可能不同。应用会在数据源状态和统计页面明确显示当前边界。

## 快速开始

1. 从 [Releases](https://github.com/Whalica/OJ_Insight/releases/latest) 下载对应平台的安装包。
2. 打开「设置」，填写需要同步的 OJ 账号并保存。
3. 打开「数据源」，对新账号执行一次「重建」。
4. 此后使用「增量」或「同步全部」更新数据。

应用启动时会先显示本地缓存，再在后台同步已配置的平台并检查更新。这两项行为都可以在设置中关闭。

### 下载哪个文件

- **Windows**：下载名称包含 `Windows` 的 EXE 安装包。
- **MacOS**：下载名称包含 `universal-MacOS` 的 DMG，同时支持 Intel 与 Apple Silicon。
- **Linux**：下载名称包含 `Linux` 的 AppImage。

## 账号与凭据

### QOJ

QOJ 需要登录后才能查看完整提交列表：

1. 在浏览器登录 [QOJ](https://qoj.ac)。
2. 在开发者工具的 Cookies 中找到 `UOJSESSID`。
3. 在 OJ Insight 的 QOJ Secret 中填入完整 Cookie 或仅填写 value。

```text
UOJSESSID=xxxxxxxx
```

Cookie 过期会显示需要重新登录；已登录但确实没有 AC 会正常记录为 0；网页结构变化或网络错误则会保留具体错误信息和旧缓存。

### LeetCode 中国站

中国站用户名需要使用 `cn:` 前缀：

```text
cn:用户名
```

公开接口不可用时，可以填写对应站点 Cookie 后重试。国际站直接填写 `/u/` 后的用户名。

### 凭据安全

Cookie 和 Session 等价于登录凭据，请勿把数据库、完整日志或含凭据的个人信息导出交给不信任的人。

个人信息 JSON 默认不包含 Cookie / Session；只有主动选择完整导出并确认警告后才会包含。运行日志会对用户填写的 Secret 和 `UOJSESSID` 脱敏。

## 统计口径

### Career 与当前范围

Career 始终基于本地已知的全部历史，不随年份或「至今」切换。当前范围只统计选中的自然年或截至今天最近 365 天。

- **Solved**：各平台内至少 AC 一次的不同题数之和，不跨 OJ 去重。
- **AC Submissions**：数据源能够取得的 Accepted submission 数量。
- **Active Days**：Activity 大于 0 的不同日期数。
- **Longest Streak**：历史最长连续活跃天数。
- **Current Streak**：截至今天的连续活跃天数。
- **Peak Day**：所选口径下记录最多的一天。

### Activity 四种口径

- **First AC**：一道题在生涯中第一次 AC 的日期计 1。
- **Unique AC**：同一道题同一天无论 AC 几次只计 1。
- **AC Submissions**：每条 Accepted submission 都计数。
- **Platform Activity**：平台公开的原始日期活动量，主要用于只能获得日历汇总的数据源。

带准确时间的记录会按所选时区重新计算日期。上游只提供 `YYYY-MM-DD` 的记录会保留来源日期，不会假造提交时刻。

### 难度

难度是有序变量，因此使用直方图展示，并保留平台自身体系：

- Codeforces：官方 Rating 分段；
- AtCoder：AtCoder Difficulty；
- 洛谷：官方难度；
- LeetCode：Easy、Medium、Hard；
- 牛客和 QOJ：仅在存在可靠难度来源时展示。

总览通过平台标签切换难度分布，不把不同 OJ 的体系换算成一个虚假的统一分数。已识别但没有可靠难度的题目会明确归入「未评级」。

## 同步与数据管理

- **增量**：从已有同步游标附近继续拉取并去重。
- **重建**：重新获取该平台当前能够取得的完整数据，并替换对应缓存。
- **清空单站**：删除该 OJ 的同步数据，保留账号设置。
- **清空所有**：清空六个 OJ 的同步数据，仍保留账号设置。

同步全部会逐站执行。一个平台失败不会中断其他平台，也不会删除该平台上次成功的数据。

删除或改名账号时，应用会清理对应账号的本地记录，不影响同一平台的其他账号。重新添加已经删除的账号后需要重新同步。

## 数据位置与备份

| 系统 | 默认位置 |
|---|---|
| Windows | `OJ Insight.exe` 所在目录 |
| MacOS | `~/Library/Application Support/com.ojinsight.app/` |
| Linux | 通常为 `~/.local/share/com.ojinsight.app/` |

实际路径可以在应用「关于」页面查看。目录结构如下：

```text
OJ Insight/
├─ data/oj-insight.sqlite3
├─ exports/
├─ logs/oj-insight.log
└─ webview/
```

迁移前请先退出应用，再复制整个目录。Windows 版需要放在普通用户可写的位置，不建议放进 `Program Files`。

## 更新与故障排查

应用可以自动检查并安装带签名的新版本，也可以从 [Releases](https://github.com/Whalica/OJ_Insight/releases/latest) 手动下载。

同步出现问题时，请先查看「数据源」页面显示的错误和 `logs/oj-insight.log`。反馈 Issue 时可以附上已经确认脱敏的相关日志行，但不要上传 Cookie、Session 或完整数据库。

MacOS 社区构建如果出现 Gatekeeper 提示，以及 Linux Wayland / niri 环境下的启动问题，请参阅[构建与故障排查说明](docs/BUILDING.md)。

## 开发与贡献

OJ Insight 使用 Tauri 2、React、TypeScript、Rust 和 SQLite。源码开发需要 Node.js 22+、pnpm 11+ 与 Rust stable。

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

项目的模块职责和依赖方向见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)，完整的系统依赖、三平台构建、发布流程和回归检查见 [docs/BUILDING.md](docs/BUILDING.md)。欢迎通过 [Issue](https://github.com/Whalica/OJ_Insight/issues) 报告数据源变化、统计问题或体验建议。
