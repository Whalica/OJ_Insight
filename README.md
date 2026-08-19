# OJ Insight

**Unified Online Judge statistics & visualization.**

OJ Insight v0.2.0 是一个 Windows / macOS 本地优先桌面面板，把 Codeforces、AtCoder、洛谷、牛客、QOJ 与 LeetCode 的个人训练数据缓存到 SQLite，并用统一的 Career、时间范围统计、Activity、平台概览和难度分布展示。

## 功能

- 总览和每个 OJ 的独立页面。
- Career 生涯统计与当前时间范围统计严格分开。
- `< [ 2026 ▼ ] >` 年份控件；`Until now` 显示截至今天最近 365 天，Activity 最右列包含今天。
- Activity 四种口径：First AC、Unique AC、AC Submissions、Platform Activity。
- Platform Summary、Recent Accepted、Data Sources。
- Difficulty Profile 按平台自身体系分别绘制 histogram，不跨 OJ 强行统一难度。
- 增量同步、全量重建、清空单 OJ、清空所有同步数据。
- 缓存数据与最近一次同步错误分离；同步失败不会删除旧缓存。
- 同步进度显示完成站点数、新增记录与失败站点数。
- 指定年份区间或 Until now 的 Activity 导出；All OJs/单 OJ；PNG/SVG。
- About 页提供版本、GitHub Releases 更新检查、仓库和 Issue 入口。
- Windows Release 使用 GUI subsystem；macOS Release 使用原生 `.app`。启动、同步、检查更新、导出均不创建 console / shell 子进程。

## 便携目录（Windows）

Windows 版所有持久数据都保存在 `OJ Insight.exe` 同级目录：

```text
OJ Insight/
├─ OJ Insight.exe
├─ data/
│  └─ oj-insight.sqlite3
├─ exports/
├─ logs/
│  └─ oj-insight.log
└─ webview/
```

- `data/`：账号、QOJ Cookie、提交、统计、同步游标与状态。
- `exports/`：默认图片导出目录。
- `logs/`：同步诊断日志。`UOJSESSID` 与用户填写的 Secret 会被脱敏。
- `webview/`：WebView2 localStorage 与缓存。

复制整个目录即可备份或迁移。目录必须可写，不建议把便携版放在普通用户不可写的 `Program Files`。

## 数据目录（macOS）

macOS 应用包是只读的，因此持久数据保存在用户应用支持目录：

```text
~/Library/Application Support/com.ojinsight.app/
├─ data/oj-insight.sqlite3
├─ exports/
├─ logs/oj-insight.log
└─ webview/
```

复制整个 `com.ojinsight.app` 目录即可备份或迁移。构建说明见 [BUILD_MACOS.md](BUILD_MACOS.md)。

## 第一次使用

1. Windows：将程序放到可写目录，例如 `D:\Tools\OJ Insight\`；macOS：打开 DMG，把 `OJ Insight.app` 拖入 `Applications`（或其他可写目录）。
2. 打开「设置」，填写需要使用的平台账号并保存。
3. 打开「数据源」，对新账号执行「重建」。
4. 以后使用「增量」或「同步全部」。
5. 数据同步后可离线查看；远端临时失败只更新 Latest sync 错误，不清除 Cached data 和 Last successful。

## 账号填写

| 平台 | 填写内容 |
|---|---|
| Codeforces | Handle |
| AtCoder | 用户名 |
| 洛谷 | 用户名或数字 UID |
| 牛客 | 个人主页 URL 中的数字 User ID |
| QOJ | 用户名；另填 `UOJSESSID` |
| LeetCode 国际站 | `/u/` 后的用户名 |
| LeetCode 中国站 | `cn:用户名` |

### QOJ

QOJ 当前要求登录后才能查看完整提交列表：

1. 在浏览器登录 `qoj.ac`。
2. 打开开发者工具的 Cookies。
3. 找到 `UOJSESSID`。
4. Secret 可填完整形式：

```text
UOJSESSID=xxxxxxxx
```

也可以只粘贴 value：

```text
xxxxxxxx
```

应用会自动补成 `UOJSESSID=value`。provider 会区分：

- 未登录或 Cookie 过期：`auth_required`；
- 已登录但筛选结果确实没有 AC：成功且记录为 0；
- 页面已返回但表格结构无法识别：结构变化错误；
- 网络/HTTP 错误：保留具体上游错误。

Cookie 等价于登录凭据。不要上传 `data/`，也不要把数据库或日志发给不信任的人。

### LeetCode

国际站直接填用户名。中国站必须加前缀：

```text
cn:admiring-sutherlanduel
```

v0.2.0 不再只替换域名：

- `leetcode.com` 使用 `matchedUser(username)` 获取公开日历与统计；
- `leetcode.cn` 使用当前公开的 `userProfileUserQuestionProgress(userSlug)` 获取解题总数与 Easy / Medium / Hard；
- 中国站的标准 `/graphql/` schema 不提供国际站的 `matchedUser/userCalendar`。为避免接口 400 让整站同步失败，应用不会再向中国站发送国际站日历查询；解题总数与难度正常同步，已有日期缓存会保留，数据源状态会明确提示 Activity 暂不可用。

GraphQL 错误会显示 operation、HTTP 状态与有限响应摘要，方便判断接口变化。

## Career 与时间范围定义

Career 永远基于本地已知的全部历史，不随年份/Until now 切换。

- `Solved`：各平台内至少 AC 一次的不同题数之和；不尝试把不同 OJ 的题目跨站去重。
- `AC Submissions`：provider 能获取到的逐题 Accepted submission 数。
- `Active Days`：Activity 大于 0 的不同日期数。
- `Longest Streak`：历史最长连续活跃天数。
- `Current Streak`：截至今天的连续活跃天数。
- `Peak Day`：所选 Activity 口径下计数最高的一天。

当前范围统计只计算选择的自然年或最近一年窗口。洛谷、LeetCode 等无法提供完整逐题数据的平台不会被伪造成逐题 AC 数据；不可用的统计会显示中文警告或“暂无”。

## Activity 四种口径

- `First AC`：一道题在生涯中第一次 AC 的日期计 1。
- `Unique AC`：同一道题同一天无论 AC 几次只计 1。
- `AC Submissions`：每条 Accepted submission 都计数。
- `Activity`：平台公开的原始日期活动量，主要用于只能获取 calendar/dailyCounts 的数据源。

Until now 固定为截至今天最近 365 天；自然年模式展示 1 月 1 日至 12 月 31 日。点击格子可查看当日逐题记录或平台公开活动说明。

## Difficulty Profile

难度是有序变量，因此使用 histogram，不使用饼图。每个平台保留自身体系：

- Codeforces / AtCoder：各自 rating/difficulty 区间；
- 洛谷：官方难度标签（可获取时）；
- LeetCode：Easy / Medium / Hard；
- 牛客 / QOJ：只有可靠难度数据时才展示。

总览通过 tab 切平台，不把不同体系映射到一个虚假的统一分数。

## 同步与数据管理

- 「增量」：从已有 cursor 附近继续拉取并去重。
- 「重建」：重新拉取该平台的完整可用数据并替换对应缓存。
- 「清空」：删除单 OJ 的提交、Activity、难度与同步状态，保留账号。
- 「清空所有」：对六站执行清空，仍保留账号。

同步全部按已配置平台逐站执行，UI 显示 `x / n`、新增记录和失败数量。任一站失败不会中断其他站，也不会删除该站上次成功缓存。

## 导出

「导出」支持：

- 年份区间或 Until now；
- All OJs 合并或单 OJ；
- PNG 或 SVG。

保存对话框默认打开对应平台数据目录中的 `exports/`。导出过程完全在应用/WebView 内完成，不启动 PowerShell、cmd 或其他 console / shell 子进程。

## About 与更新检查

About 显示当前版本 `0.2.0`。Check for Updates 请求：

```text
https://api.github.com/repos/Whalica/OJ_Insight/releases/latest
```

这里只检查并跳转到 GitHub Release，不自动下载安装。Repository、Report an Issue 和 Release 均从 About 打开。

## 日志与故障排查

诊断日志位于对应平台数据目录中的 `logs/oj-insight.log`：

- Windows：`OJ Insight.exe` 同级的 `logs/oj-insight.log`。
- macOS：`~/Library/Application Support/com.ojinsight.app/logs/oj-insight.log`。

日志记录同步开始、完成、insert/update 数与错误分类，不记录明文 QOJ Secret。若 QOJ 报「结构变化」，可在确认日志已脱敏后附上相关错误行提交 Issue；不要附带数据库。

## 源码开发

要求：Node.js 22+、Rust stable。

- Windows：Visual Studio C++ Build Tools、WebView2 Runtime。
- macOS：Xcode Command Line Tools（WKWebView 由系统提供）。

```bash
npm install
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

开发模式数据位置：

- Windows：当前可执行文件旁，通常是 `src-tauri/target/debug/{data,exports,logs,webview}`。
- macOS：`~/Library/Application Support/com.ojinsight.app/`。

## GitHub Actions 与发布

- `.github/workflows/windows-build.yml`：手动 `workflow_dispatch` 构建 Windows NSIS/MSI 并上传 artifact。
- `.github/workflows/macos-build.yml`：手动 `workflow_dispatch` 或 Pull Request 触发 macOS `app` + `dmg` 构建，上传 DMG 与 `.app.zip` artifact。
- `.github/workflows/release.yml`：推送 `v*` tag 时自动完成 Windows/macOS 构建，并创建/更新 GitHub Release，自动上传：
  - Windows：`*.exe`、`*.msi`
  - macOS：`*.dmg`、`OJ Insight.app.zip`

发布前确保下列版本一致：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

然后推送 tag：

```bash
git tag v0.2.0
git push origin v0.2.0
```

Release workflow 会自动创建 GitHub Release 并挂载安装包，无需再从 Actions Artifacts 手动下载上传。

Windows Release 构建仍使用 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`；macOS Release 直接使用 `.app` 应用包。

## 数据源边界

OJ Insight 尊重上游公开数据能力，不虚构统一精度：

- CF / AtCoder / 牛客 / 已登录 QOJ：可保存逐题 AC 历史。
- 洛谷：公开 `dailyCounts` 更稳定，部分数据只能用于 Activity。
- LeetCode 国际站：`submissionCalendar` 表示提交活动，并不提供完整历史逐题首次 AC。
- LeetCode 中国站：公开 profile 可同步解题总数与难度；Activity 日历由独立 schema 尝试获取，不可用时安全降级并保留旧缓存。

上游网站可能随时修改接口或限制访问。错误应表现为 Latest sync 失败，旧缓存仍可查看。
