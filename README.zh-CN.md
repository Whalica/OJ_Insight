<p align="center"><img src="src/assets/branding/oji-logo.png" alt="OJ Insight Logo" width="144" /></p>

# OJ Insight

[![Read in English](docs/assets/read-in-english.svg)](README.md)

**把跨 OJ 的训练进度、日常练习和比赛复盘放在一个本地应用里。**

OJ Insight 面向算法竞赛选手，汇集 Codeforces、AtCoder、洛谷、牛客、QOJ 和 LeetCode 的训练记录，并提供独立的 ICPC / CCPC Tracker。它保留各平台原有的 Rating 与难度体系，明确区分可核实的逐题记录、公开汇总和暂时无法取得的数据。

[下载最新版本](https://github.com/Whalica/OJ_Insight/releases/latest) · [用户手册](docs/manual/user-manual.pdf) · [更新记录](docs/UPDATEINFO.md) · [反馈问题](https://github.com/Whalica/OJ_Insight/issues)

当前版本：**v0.10.5** · 支持 Windows、macOS 和 Linux。

## 集中查看跨 OJ 的成长

总览集中展示解题数、AC 提交、活跃天数、连续训练、Rating 历史、最近 AC 与难度分布。活动砖可查看自然年或最近 365 天，并分别呈现首次 AC、当日去重 AC、AC 提交数及平台原始活动量；图表支持导出 PNG 或 SVG。

同一平台可记录多个账号。不同 OJ 的 Rating 和难度不会被强行换算成统一分数；只有日期汇总时，也不会凭空生成逐题提交。数据源暂时不可用时，应用显示状态并保留此前的本地缓存。

## 整理日常训练

- **题单**：整理跨 OJ 题目、角色、备注和标签，支持导入导出，也能转成独立的模拟赛。
- **推荐题单**：浏览 [OJ Insight Community](https://github.com/Whalica/OJ_Insight-Community) 审核过的题单及多层文件夹，对照本地通过记录预览后保存副本。个人提交、笔记和代码不会自动上传。
- **收藏夹**：分类整理题目、题单、博客等网页链接，支持搜索、置顶，以及编辑显示名称、简介和 Markdown 备注；原网页内容不会被复制进来。
- **做题小助手与解题手记**：练习计时、记录 Markdown 笔记和失误；结束后归档，未完成时保留草稿。手记支持搜索、筛选和 JSON / CSV 导出。桌面应用运行时，Competitive Companion 也可为当前题目准备草稿。
- **个性化组题**：从可靠的 Codeforces、AtCoder 和 QOJ 目录筛选候选题并排除本地已做题，导出包含个人画像、约束、候选池和大模型指令的 ZIP；生成的比赛 JSON 可导入为题单或比赛。

## 比赛训练与复盘

模拟赛保存独立的配置和历史场次。VP 支持赛前倒计时、暂停与继续、单题思路和整场笔记。提交仍在原 OJ 进行；OJ Insight 根据本地 AC 记录与可取得的 Codeforces / AtCoder 判题结果更新状态，不会把无法取得的错误提交推断为 WA。

**赛后分析**可以整理已结束的 VP 或 AtCoder 正式比赛，导出包含题目、提交时间线、Markdown 笔记、可取得的代码和大模型入口说明的复盘包。

**ICPC / CCPC Tracker** 汇集 ICPC、CCPC 和省赛题集，根据本地 QOJ 记录显示补题进度，公开榜单可用时也展示题目层级；支持按年份、阶段、赛站、系列、完成情况和层级筛选。**关注**可跟踪队友或朋友的公开账号，提示新的 AC。

## 支持的平台

| 平台 | 可用信息 |
|---|---|
| Codeforces | 逐题 AC、难度与 Rating |
| AtCoder | 逐题 AC、难度与 Algorithm Rating |
| 洛谷 | 可取得的提交或公开活动、题量与官方难度 |
| 牛客 | 普通题 AC 与 Tracker 完成记录 |
| QOJ | 逐题 AC 与 ICPC / CCPC 补题记录；完整提交历史需要登录 |
| LeetCode | 题量与难度；国际站比赛 Rating 在可取得时展示 |

各平台公开的数据范围和接口可能变化。OJ Insight 会标明缺失或受限信息，不把它误写成零或完整记录。

## 数据留在本地

账号、同步记录、训练历史与导出文件保存在应用自己的数据目录，不需要注册 OJ Insight 账号。常规个人信息导出不包含登录凭据；应用支持数据库备份和签名更新检查。

英文界面仍在**开发中**：导航和部分训练工具已有英文内容，其他页面仍为中文，语言切换入口暂时隐藏。题目原名、平台标签和个人笔记保留原文。

安装配置、账号凭据、备份和故障排查请参阅[用户手册](docs/manual/user-manual.pdf)。项目架构和源码构建另见[架构文档](docs/ARCHITECTURE.md)与[构建文档](docs/BUILDING.md)。
