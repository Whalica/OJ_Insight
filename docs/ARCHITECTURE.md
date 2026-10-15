# 架构与数据流

OJ Insight 是一个 Tauri 2 桌面应用。React 前端负责界面与交互，Rust 后端负责外部数据同步、SQLite 存储、领域逻辑和系统能力。

先阅读 [产品原则](PRODUCT_PRINCIPLES.md) 和 [贡献指南](../CONTRIBUTING.md)。本文件说明实现位置，不授权新增功能或修改产品行为。

## 根目录

```text
OJ_Insight/
├─ .github/             GitHub Actions
├─ config/              Vite、TypeScript、Playwright 配置
├─ docs/                架构、构建、更新记录和用户手册
├─ scripts/             版本与发布辅助脚本
├─ src/                 React 前端
├─ src-tauri/           Rust / Tauri 后端
├─ tests/               Playwright 布局回归测试
├─ index.html           Vite 页面入口
├─ package.json
├─ pnpm-lock.yaml
└─ pnpm-workspace.yaml
```

`package.json`、两个 pnpm YAML 和 `index.html` 保留在根目录。工具配置集中在 `config/`，不改变 Vite root。

## 前端

```text
src/
├─ components/          可复用展示组件
├─ hooks/               应用状态、同步和布局逻辑
├─ lib/                 日期、导航、平台配置和纯领域辅助函数
├─ pages/               侧栏对应的页面组件
├─ services/            Tauri 命令、更新和导出等外部交互
├─ styles/              按功能拆分的样式
├─ App.tsx              页面编排、导航、全局提示和页面级组装
├─ main.tsx             React 入口
├─ styles.css           全局样式导入入口
└─ types.ts             前后端交互使用的 TypeScript 数据形状
```

依赖方向保持为：

```text
pages / components
        ↓
hooks / lib / services
        ↓
Tauri commands
```

- `lib` 不依赖 React 页面或组件。
- Tauri `invoke` 调用集中在 `services`。
- 账号、统计快照、同步、关注和更新逻辑放在对应 hooks 中。
- `App.tsx` 主要负责 layout、navigation、global toast 和 page assembly。
- 页面标识统一由 `lib/navigation.ts` 导出，避免导航组件重复维护页面类型。

样式入口按以下顺序加载：

```text
styles/
├─ tokens.css           主题色、OJ 色、奖牌色和共享变量
├─ base.css             基础元素和通用组件
├─ following.css        关注功能
├─ dashboard.css        数据面板、Rating 和知识画像
├─ settings.css         设置页
├─ xcpc.css             ICPC / CCPC Tracker
├─ layout.css           共享布局、响应式、导出和比赛复盘
├─ training.css         题单与训练比赛页面
├─ favorites.css        链接收藏
├─ study-assistant.css  独立做题小助手
├─ solve-journal.css    解题手记
└─ kickers.css          小标题字体与主题颜色
```

## Rust 后端

```text
src-tauri/src/
├─ app/                 应用状态与资源初始化
├─ commands/            Tauri 命令入口和参数校验
├─ db/                  SQLite 连接、迁移和各领域持久化
├─ infrastructure/      路径、日志等基础能力
├─ sync/                外部 OJ 数据抓取、标准化和同步编排
├─ xcpc/                ICPC / CCPC 目录、匹配、标签和 Rating
├─ training/            题单、候选池、比赛与训练导出
├─ fetch_queue.rs       外部请求并发控制
├─ models.rs            当前共享领域模型
├─ operation.rs         长操作并发保护
├─ lib.rs               插件、窗口、状态和命令注册
└─ main.rs              桌面程序入口
```

主要调用方向为：

```text
Tauri invoke
    ↓
commands
    ↓
domain / service
    ↓
db
    ↓
SQLite
```

新增逻辑优先让命令层接收参数、进行边界校验并调用领域代码。当前社区目录、手记等部分实现仍集中于命令文件，修改时应识别这一现状，不把目录示意当成已完成的全面分层。`lib.rs` 只负责应用组装和命令注册。

### 数据库

```text
db/
├─ mod.rs               公共数据库接口和数据库测试
├─ connection.rs        打开连接和初始化流程
├─ schema.rs            初始数据库结构
├─ migrations.rs        兼容迁移
├─ accounts.rs          OJ 账号
├─ submissions.rs       提交、同步状态和数据清理
├─ ratings.rs           Rating、难度和知识画像
├─ analytics.rs         快照、日期和详情统计
├─ relationships.rs     关注账号、事件和通知
├─ training.rs          Problem Set、Match 与训练画像持久化
└─ favorites.rs         链接收藏与分类持久化
```

现有数据库实现采用渐进式拆分：各职责已分文件，入口继续导出原有 `db::*` 调用，避免在一次重构中改变所有调用方。

### 外部数据同步

```text
sync/
├─ service.rs           多账号同步流程与数据库写入编排
├─ relationships.rs     关注账号同步
├─ metadata_cache.rs    公共题目元数据缓存
├─ codeforces.rs
├─ atcoder.rs
├─ luogu.rs
├─ nowcoder.rs
├─ qoj.rs
└─ leetcode.rs
```

`sync/` 只负责“外部 OJ 数据进入 OJ Insight”。训练题单、训练模板和 Match 不放入 `sync/`；同步完成后，训练领域可根据数据库中的提交记录更新 Match AC 状态。

### ICPC / CCPC

```text
xcpc/
├─ mod.rs               公共接口和模块测试
├─ model.rs             内部数据结构
├─ catalog.rs           目录加载和总体编排
├─ matcher.rs           比赛名称标准化与匹配
├─ rating.rs            公开榜单数据合并与题目层级
├─ tags.rs              题目标签
├─ cache.rs             本地目录缓存
└─ sources/
   ├─ qoj.rs
   ├─ rankland.rs
   └─ xcpcio.rs
```

外部数据源抓取位于 `sources/`，比赛匹配和标准化位于 `matcher.rs`，两类职责保持分离。

比赛复盘由 `commands/contest_review.rs` 独立处理，不写入训练统计表。各 OJ 的比赛与提交数据先标准化，再生成固定结构的 Markdown 文档和本地 ZIP；账号凭据只用于请求，不进入导出内容。

## Training System 边界

Training System 使用独立领域目录，不并入 `sync/` 或 `xcpc/`：

```text
training/
├─ mod.rs
├─ model.rs
├─ problem_set.rs
├─ template.rs
├─ candidate.rs
├─ pack.rs
└─ match.rs
```

数据库持久化进入 `db/training.rs`，Tauri 接口进入 `commands/training.rs`。前端分为 `ProblemSetsPage`、`ContestsPage`、`VpPage`、`TrainingPage` 和 `TrainingReviewPage`。Problem Set 是可复用集合，Contest 是比赛配置快照，Match 是一次 VP 实例；三者共用 canonical problem identity。旧 `training_matches` 原位扩展，保留既有记录。题单与手动比赛的链接解析不依赖外部题目 API，标题与标签可以后续补充；比赛说明和笔记以 Markdown 文本保存，前端负责预览。

自动组题从可靠目录与本地已做记录生成候选池，不使用已有题单作初筛来源。AI 组题包自带指令和 JSON schema，生成结果可导入为题单或比赛。比赛加入参赛区后形成 Match；`countdown_seconds` 保存赛前预设，只有点击“开始 VP”才写入倒计时截止时间，到期再转为进行中。赛时 verdict 独立保存在 `vp_submissions`，本地同步表仍只承载 AC。复盘时按题绑定的代码以 BLOB 存在 `vp_code_files`，导出的 ZIP 不含账号凭据。

## 新增功能时

1. 先确定功能属于页面、应用 hook、领域逻辑、外部同步还是持久化。
2. 前后端交互结构需要在 TypeScript 和 Rust 两侧保持一致。
3. 领域逻辑通过 `commands` 暴露，再在 `services` 中集中添加 Tauri 调用。
4. 个人 OJ 记录同步放入 `sync`；XCPC 公共目录和榜单适配位于 `xcpc/sources`。社区目录和训练候选池按各自领域处理，不混入个人同步。新增持久化优先放入 `db`。
5. 页面局部状态留在页面中，跨页面或应用级逻辑抽到 hooks。
6. 按 [构建文档](BUILDING.md#按改动选择验证) 选择必要验证；改动数据口径或产品行为时同步相关说明。

## 数据整合主链路

```text
用户发起同步 → useSync / services/api → commands/sync
    → sync/service → 对应 OJ 适配器 → 标准化记录
    → db 持久化与统计 → commands/analytics → useSnapshot → 图表与详情
```

`app/state.rs` 维护数据库连接、HTTP 客户端、操作门和数据路径；`fetch_queue.rs` 控制请求并发。`sync/metadata_cache.rs` 处理公共元数据缓存。不要在 React 页面中另外建立一套真实账号同步流程。

统计口径定义可从 `src/lib/platforms.ts` 的 `METRICS`、`src/lib/date.ts`、`db/analytics.rs` 和 `db/submissions.rs` 追踪。改动时核对账号维度、题目唯一标识、去重粒度、时区、日期范围、缓存和缺失状态。输入、落库、查询及图表必须保持一致。

前后端结构主要见 `src/types.ts`、`src-tauri/src/models.rs`，部分领域在各自模块内定义类型。新增或修改字段需检查实际命令返回值、序列化命名、前端消费点、测试模拟值和旧数据兼容，不能只改一侧声明。

## 辅助能力的实际入口

| 能力 | 前端入口 | 后端及存储 |
| --- | --- | --- |
| 本地与推荐题单 | `ProblemSetsHubPage`、`ProblemSetsPage`、`CommunityPage` | `commands/training.rs`、`commands/community.rs`、`training/`、`db/training.rs` |
| 模拟赛、参赛区、赛后分析 | `ContestsHubPage` 分发到对应页面 | `commands/training.rs`、`commands/contest_review.rs`、`training/` |
| 收藏夹 | `FavoritesPage` | `commands/favorites.rs`、`db/favorites.rs` |
| 小助手与手记 | `StudyAssistant`、`SolveJournalPage`、`services/solveJournal.ts` | `commands/assistant.rs`、`commands/solve_journal.rs`；`solve_records` 表由数据库初始化创建 |
| Competitive Companion | 手记中的接入状态与草稿入口 | `commands/companion.rs` 的本地监听，草稿写入使用手记能力 |
| 关注 | `RelationshipsPage`、`useFollowing` | `commands/relationships.rs`、`sync/relationships.rs`、`db/relationships.rs` |
| 更新 | `useUpdater`、`services/updater.ts` | `commands/update.rs` 与 Tauri updater |

小助手包含独立窗口；普通浏览器预览与桌面 IPC 的能力不同，修改时同时检查服务层和测试 mock，不能用 mock 成功推断桌面保存已成功。

`ProblemSetsHubPage` 承担本地/推荐切换；`ContestsHubPage` 按侧栏选择分发页面，不重复显示模拟赛/参赛区/赛后分析的内部导航。当前结构与入口调整的产品边界见 [界面规范](UI_AND_COPY.md)。

## 兼容与验证边界

- 数据库新增或变更字段需检查 `schema.rs`、`migrations.rs`、`connection.rs` 和使用该字段的领域初始化，验证已有数据可继续读取。
- 测试使用临时数据库和模拟账号；不同 OJ 的空数据、部分成功与失败状态分别覆盖。
- 布局测试使用 Tauri mock，能发现布局和交互问题，但不能代替桌面窗口、真实文件系统、数据库或网络适配验证。
- 架构改动与产品改动可以相互关联，但应分别说明原因。不要借重构删除反馈或改变统计含义。
