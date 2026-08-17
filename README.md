# OJ Insight

**Unified Competitive Programming Analytics**

OJ Insight 是一个本地优先的桌面数据面板，用于把多个 Online Judge 的个人训练数据整合到同一个 SQLite 数据库中，并统一展示砖墙、解题量、活跃天数、连续训练、难度分布和逐日记录。

当前版本：`0.1.1`

> 这一版采用**便携式数据布局**：OJ Insight 产生的持久化数据全部保存在程序根目录下，不写 `%APPDATA%` / `%LOCALAPPDATA%`。

---

## 1. 已接入平台

- **Codeforces**：官方 `user.status` API，保存 AC 提交及题目 Rating。
- **AtCoder**：AtCoder Problems submission API，保存 AC 提交并读取 problem difficulty。
- **洛谷**：读取个人页 `dailyCounts` 作为日期活动砖墙，并尝试从 practice 数据读取总通过数/难度；不伪造逐日题目明细。
- **牛客**：读取竞赛站公开练习提交页的 AC 记录；账号填写**数字 User ID**。
- **QOJ**：完整提交列表目前需要登录态，因此支持用户名 + 可选 `UOJSESSID=...` Cookie。登录态失效时会显示 `auth_required`，不会错误显示成 0 条记录。
- **LeetCode**：支持国际站与中国站。国际站直接填用户名，中国站填写 `cn:用户名`；通过 GraphQL 获取活动日历与难度统计。

---

## 2. 当前功能

### 数据整合

- 六个 OJ 统一账号配置。
- SQLite 本地持久化。
- 增量同步。
- 单个平台全量重建。
- 一键同步全部已配置平台。
- 离线查看已经同步到本地的数据。
- 数据源状态、最后成功同步时间和错误信息。

### 统计展示

- 所有 OJ 合并总览。
- 单独查看某个 OJ。
- 年度 GitHub 风格砖墙。
- 四种砖墙统计口径：
  - 首次 AC
  - 当日去重 AC
  - AC 提交
  - 平台活动
- Solved 数。
- AC 提交数。
- 活跃天数。
- 最长连续天数。
- 当前连续天数。
- 峰值日。
- Codeforces / AtCoder 难度区间统计。
- LeetCode Easy / Medium / Hard 统计。
- 洛谷在数据可用时展示官方难度信息。
- 点击日期查看单日详情。

### 数据管理

- 清空指定 OJ 的本地记录。
- 清空全部 OJ 的本地记录。
- 账号配置与同步数据统一存储在根目录数据库中。
- 整个应用目录可直接复制备份或迁移。

### 导出

- 指定起止年份。
- 所有 OJ 合并导出。
- 单个 OJ 导出。
- PNG 导出。
- SVG 无损导出。
- 默认保存位置为程序根目录下的 `exports/`。

---

# 3. 数据保存位置

OJ Insight **不会把自己的数据库写入系统 AppData**。

假设你的应用目录是：

```text
D:\Tools\OJ Insight\
```

首次运行后目录结构会变成：

```text
OJ Insight\
├─ OJ Insight.exe
├─ data\
│  └─ oj-insight.sqlite3
├─ exports\
│  └─ ...
└─ webview\
   └─ ...
```

其中：

- `data/oj-insight.sqlite3`
  - OJ 账号配置
  - QOJ Cookie（如果填写）
  - 已同步提交记录
  - 日期活动数据
  - 难度统计
  - 同步游标
  - 数据源状态
- `exports/`
  - 默认的 PNG / SVG 导出目录
- `webview/`
  - Windows WebView2 为 OJ Insight 产生的 localStorage、缓存等运行数据
  - 该目录也被固定在程序根目录，不使用系统 AppData 作为 OJ Insight 的 WebView 数据目录

因此迁移电脑时，不需要寻找任何隐藏目录：

```text
直接复制整个 OJ Insight 文件夹即可。
```

## 重要：目录必须可写

因为数据库就放在程序旁边，**不要把便携版放在普通用户无写权限的目录中**，例如某些机器上的：

```text
C:\Program Files\...
```

推荐：

```text
D:\Apps\OJ Insight\
D:\Tools\OJ Insight\
C:\Users\你的用户名\Desktop\OJ Insight\
```

如果目录不可写，OJ Insight 无法创建 `data/` 或数据库，并会在启动阶段报错。

---

# 4. 快速开始

下面按第一次使用的完整流程说明。

## 第一步：准备应用目录

将 OJ Insight 放入一个固定且可写的目录，例如：

```text
D:\Tools\OJ Insight\
```

不要只把 `OJ Insight.exe` 放在临时下载目录中使用几天后再删除，因为数据库与它位于同一根目录。

## 第二步：启动 OJ Insight

双击：

```text
OJ Insight.exe
```

首次启动时会自动创建：

```text
data\
exports\
webview\
```

如果这三个目录成功出现，说明便携数据目录工作正常。

## 第三步：打开「设置」

左侧导航选择：

```text
设置
```

这里可以填写六个平台账号。

不需要使用的平台可以留空。

填写完成后点击：

```text
保存
```

账号会写入：

```text
data\oj-insight.sqlite3
```

---

# 5. 各 OJ 账号怎么填

## Codeforces

填写 Handle，例如：

```text
Whalica
```

不需要 Cookie 或 API Key。

## AtCoder

填写 AtCoder 用户名，例如：

```text
Whalica
```

大小写应与实际用户名保持一致。

## 洛谷

按照应用输入框提示填写洛谷账号标识。

洛谷当前主要使用个人页公开的日期热度数据。由于公开接口粒度限制，一部分日期只能知道“当天有多少活动”，不能恢复当天具体 AC 题目。

这属于数据源能力限制，不是本地数据库丢数据。

## 牛客

牛客填写**数字 User ID**，不是昵称。

例如个人主页地址类似：

```text
https://www.nowcoder.com/users/123456789
```

则填写：

```text
123456789
```

## QOJ

QOJ 至少填写用户名。

如果只填写用户名而 QOJ 不允许匿名读取完整提交记录，状态页会显示需要登录。

如果需要完整同步，可以额外填写浏览器中的：

```text
UOJSESSID=xxxxxxxxxxxxxxxx
```

### 如何获取 QOJ Cookie

1. 在浏览器中正常登录 QOJ。
2. 打开浏览器开发者工具。
3. 找到 QOJ 域名对应的 Cookie。
4. 找到 `UOJSESSID`。
5. 在 OJ Insight 的 QOJ 凭据栏填写完整形式：

```text
UOJSESSID=你的值
```

### 安全说明

当前版本会把 QOJ Cookie 保存在本地 SQLite：

```text
data\oj-insight.sqlite3
```

因此：

- 不要把自己的 `data/` 目录公开上传。
- 不要把数据库发给不信任的人。
- 如果 Cookie 泄露，请在 QOJ 退出登录或重新登录，使旧登录态失效。

后续版本计划改为系统 Keyring / Windows Credential Manager。

## LeetCode 国际站

直接填写用户名：

```text
username
```

## LeetCode 中国站

在用户名之前加：

```text
cn:
```

例如：

```text
cn:username
```

这样 OJ Insight 会按中国站账号处理。

---

# 6. 第一次同步

完成账号设置后，进入：

```text
数据源
```

或者在对应平台页面执行同步。

## 推荐第一次使用：全量同步

对于第一次添加的平台，执行：

```text
全量重建 / 全量同步
```

它会尽可能读取该平台已有历史数据，然后写入本地数据库。

第一次通常比以后慢，因为需要拉取完整历史。

## 之后使用：增量同步

以后正常使用时选择：

```text
同步
```

OJ Insight 会根据已经保存的同步游标，只获取后续新增数据。

这样可以：

- 减少请求数量；
- 提高启动后的同步速度；
- 降低洛谷、牛客、QOJ 等平台触发访问限制的概率。

## 同步全部

如果配置了多个 OJ，可使用：

```text
同步全部
```

只会处理已经填写账号的平台。

---

# 7. 如何判断同步是否正常

进入：

```text
数据源
```

每个平台会展示自己的同步状态。

常见状态含义：

### 正常

```text
ok
```

表示最近一次同步成功。

### 正在同步

```text
syncing
```

等待当前请求完成即可。

### 需要登录

```text
auth_required
```

常见于 QOJ，表示当前公开访问不足以读取完整数据，需要有效 Cookie。

### 请求失败 / 数据源变化

错误区域会保留具体信息，例如：

- HTTP 403
- HTTP 429
- 用户不存在
- 页面格式变化
- 网络连接失败

即使某个平台暂时同步失败，**以前已经进入 SQLite 的数据仍然可以离线查看**。

---

# 8. 总览页面怎么用

选择左侧：

```text
总览
```

这里聚合所有已经同步的平台。

可以查看：

- Solved
- AC 提交
- 活跃天
- 最长连续
- 当前连续
- 峰值日
- 总砖墙

总览不是简单把所有平台数字机械相加。对于不同口径，OJ Insight 会根据平台实际具备的数据能力进行统计。

---

# 9. 砖墙四种口径

不同 OJ 提供的数据不完全一致，因此 OJ Insight 把砖墙口径明确区分。

## 首次 AC

一题只在第一次 AC 的日期计入一次。

适合看真正的“新增解题量”。

## 当日去重 AC

同一道题同一天无论 AC 多少次，只计一次。

适合观察每天实际覆盖了多少题。

## AC 提交

每个 Accepted submission 都计数。

重复提交也会增加数量。

适合观察提交活动强度，但不适合作为纯 solved 数。

## 平台活动

用于只能可靠提供日期活动计数的平台，例如某些 LeetCode / 洛谷数据。

这个数字不一定等价于“当天首次 AC 题数”。

---

# 10. 单独查看某个 OJ

左侧直接点击：

```text
Codeforces
AtCoder
洛谷
牛客
QOJ
LeetCode
```

页面会切换到该平台独立统计。

此时：

- 砖墙只包含当前平台；
- Solved / AC / 活跃天等都只统计当前平台；
- 难度统计采用该平台能够获得的难度体系；
- 日期详情只显示该平台记录。

这适合判断某段时间自己主要在哪个平台训练。

---

# 11. 查看某一天的记录

在砖墙中点击某个日期。

如果平台保存了逐题提交历史，会打开单日详情，展示例如：

```text
时间
题目 ID
题目名称
语言
难度
题目链接
```

如果数据源本身只提供日期计数，例如某些洛谷 / LeetCode 数据，则只会显示该日活动计数，不会虚构不存在的逐题历史。

---

# 12. 导出指定年份总图

进入：

```text
导出
```

## 选择年份范围

例如：

```text
2024 — 2026
```

## 选择平台

可以选择：

```text
所有 OJ 合并
```

也可以单独选择某个已配置平台。

## 选择格式

### PNG

适合：

- 发群聊；
- 发博客；
- 插入文档；
- 直接作为图片保存。

### SVG

适合：

- 无损缩放；
- GitHub README；
- 后续编辑；
- 高清打印或排版。

## 默认保存目录

保存窗口默认定位到：

```text
OJ Insight\exports\
```

例如：

```text
exports\OJ-Insight-2024-2026.png
```

你仍然可以在保存窗口手动选择其他路径。

---

# 13. 清空某个 OJ 的数据

如果某个平台：

- 填错账号；
- 历史同步结果异常；
- 想重新拉取；

可以选择：

```text
清空该 OJ 数据
```

然后重新执行全量同步。

清空的是本地统计记录，不会修改在线 OJ 上的任何数据。

---

# 14. 清空所有 OJ 数据

数据管理中可以执行：

```text
清空所有 OJ 记录
```

这适合完全重建本地统计库。

执行前建议先备份：

```text
data\oj-insight.sqlite3
```

注意：当前版本的“清空记录”与“删除整个数据库文件”不是完全相同概念。若希望彻底恢复成全新状态，可关闭 OJ Insight 后手动备份并删除：

```text
data\oj-insight.sqlite3
```

再次启动会自动创建新的数据库。

---

# 15. 备份数据

OJ Insight 的备份非常直接。

## 方法一：备份数据库

关闭应用后复制：

```text
data\oj-insight.sqlite3
```

即可保存所有本地数据。

## 方法二：备份整个应用目录

推荐长期使用这种方式：

```text
OJ Insight\
```

整个文件夹复制到其他磁盘。

这样同时保留：

- 程序；
- 数据库；
- 导出图片；
- WebView2 本地缓存 / localStorage；
- 当前目录结构。

---

# 16. 换电脑 / 移动目录

因为 OJ Insight 采用便携式数据库布局，所以迁移步骤只有：

1. 关闭 OJ Insight。
2. 复制整个 `OJ Insight` 文件夹。
3. 粘贴到新电脑的可写目录。
4. 启动 `OJ Insight.exe`。

不需要重新导入数据库。

例如：

```text
旧电脑：D:\Tools\OJ Insight\

复制整个目录
        ↓

新电脑：E:\Apps\OJ Insight\
```

应用会直接使用新目录中的：

```text
data\oj-insight.sqlite3
```

---

# 17. 更新应用时如何保留数据

如果之后拿到新版 OJ Insight：

## 推荐方法

1. 关闭旧版。
2. 备份 `data/`。
3. 用新版程序文件替换旧版程序文件。
4. **不要删除 `data/`。**
5. 启动新版。

理想目录：

```text
OJ Insight\
├─ OJ Insight.exe        ← 更新这个及相关程序文件
├─ data\                 ← 保留
│  └─ oj-insight.sqlite3
├─ exports\              ← 保留
└─ webview\              ← 保留
```

如果未来数据库结构发生升级，版本说明会注明是否需要迁移。

---

# 18. 常见问题

## Q1：启动后没有 `data/` 文件夹

检查：

1. 是否启动的是正确的 `OJ Insight.exe`；
2. 当前目录是否有写权限；
3. 是否被安全软件阻止创建文件。

建议把程序移动到：

```text
D:\Tools\OJ Insight\
```

再试。

## Q2：同步失败会不会把以前的数据删掉？

不会。

普通增量同步失败时，已经保存在 SQLite 中的数据仍然保留。

## Q3：为什么不同 OJ 的砖墙数字和官网看起来不完全一样？

因为不同网站公开的数据口径不同。

OJ Insight 不会把：

- 提交活动；
- Accepted submission；
- 首次 AC；
- 当日去重题目；

强行当成同一种数据。

请根据当前页面选择的砖墙口径判断。

## Q4：洛谷为什么某一天看不到具体题目？

洛谷目前主数据源的公开个人热度数据只稳定提供日期计数，不能可靠恢复逐题历史。

## Q5：LeetCode 为什么 activity 和 solved 不完全对应？

LeetCode calendar 表示提交活动，不等于完整的“每天首次通过题目列表”。

## Q6：QOJ 一直提示需要登录

更新设置中的 `UOJSESSID`。

登录 Cookie 可能因为退出登录、过期或服务器刷新会话而失效。

## Q7：我想彻底重置 OJ Insight

关闭应用，然后删除：

```text
data\oj-insight.sqlite3
```

再次启动即可生成新数据库。

## Q8：我能直接把整个文件夹放 U 盘吗？

可以，只要当前运行目录允许写入。

这也是采用根目录便携数据布局的主要目的之一。

---

# 19. 从源码开发运行

如果只是使用已经构建好的 Windows 应用，不需要安装 Node.js 或 Rust。

只有开发 / 构建源码时才需要以下环境：

- Node.js 22+
- Rust stable
- Windows：Visual Studio C++ Build Tools
- WebView2 Runtime

安装依赖：

```bash
npm install
```

开发运行：

```bash
npm run tauri dev
```

### 开发模式数据位置

便携数据目录始终相对于**当前运行的可执行文件**。

因此在 `tauri dev` 下，数据库通常会位于类似：

```text
src-tauri\target\debug\data\oj-insight.sqlite3
```

这与正式版“数据跟随 exe”的规则一致。

---

# 20. 构建 Windows 应用

```bash
npm install
npm run tauri build
```

构建产物通常位于：

```text
src-tauri\target\release\
```

安装包位于：

```text
src-tauri\target\release\bundle\
```

仓库内也提供：

```text
.github\workflows\windows-build.yml
```

如果本机不想安装完整 Rust / Visual Studio 构建环境，可以把源码推送到 GitHub，然后运行 **Build Windows** workflow。

### 便携使用建议

如果希望完全保持“程序 + 数据都在一个文件夹”的效果，最直观的方式是使用构建后的主程序，并将其放入自己有写权限的独立目录。

---

# 21. 项目结构

```text
OJ Insight
├─ src/                       React + TypeScript UI
│  ├─ components/
│  └─ lib/
├─ src-tauri/
│  ├─ src/db.rs               SQLite / 统计查询
│  ├─ src/sync/               六个平台同步适配器
│  └─ src/lib.rs              Tauri commands / 便携数据路径
├─ data/                      运行后创建，本地数据库
├─ exports/                   运行后创建，默认导出位置
├─ webview/                   运行后创建，WebView2 本地数据
└─ .github/workflows/         Windows 自动构建
```

---

# 22. 数据口径说明

不同 OJ 能公开获取的数据粒度不同，因此 OJ Insight **不强行伪造统一数据**：

- CF / AtCoder / 牛客 / 已登录 QOJ：存在逐题 AC 历史，可计算首次 AC、当日去重 AC、AC 提交。
- 洛谷：公开个人页目前更稳定地提供日期热度计数，不稳定或受限的 `/record/list` 不作为主数据源。
- LeetCode：`submissionCalendar` 属于提交活动数据，不等价于完整逐题首次 AC 历史。

在某个平台不支持当前砖墙口径时，界面应显示数据能力警告，而不是制造一个看似统一但实际错误的数字。

---

# 23. 后续计划

优先级较高的后续功能：

1. 月 / 周视图。
2. 自定义日期区间统计，而不只按整年。
3. Rating 时间轴（CF / AtCoder / 牛客等可获取平台）。
4. 比赛 / 补题 / 普通练习分类。
5. 完整题目数据库与搜索。
6. 年度报告导出。
7. 系统 Keyring 保存 QOJ Cookie。
8. 自动备份与数据库迁移。
9. 系统托盘 / 可选自动同步。
