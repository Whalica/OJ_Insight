# 项目结构

OJ Insight 是一个 Tauri 2 桌面应用。React 前端负责界面与交互，Rust 后端负责外部数据同步、SQLite 存储、领域逻辑和系统能力。

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
└─ training.css         题单与训练比赛页面
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

命令层接收参数、进行边界校验并调用领域代码。`lib.rs` 只负责应用组装和命令注册。

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
└─ training.rs          Problem Set、Match 与训练画像持久化
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

数据库持久化进入 `db/training.rs`，Tauri 接口进入 `commands/training.rs`。前端页面使用 `ProblemSetsPage`、`TrainingPage` 和对应 hooks。Problem Set、Training Template 与 Match 共用 canonical problem identity。

自动组题与已有题单直接训练是两条独立路径。自动组题从可靠的跨 OJ 题目目录出发，结合本地已做记录生成候选池，再导出 AI 组题包；已有题单不会作为自动候选池的来源。大模型只负责在候选池与约束内生成可导入的 Problem Set，用户查看题单后才启动 Match。

## 新增功能时

1. 先确定功能属于页面、应用 hook、领域逻辑、外部同步还是持久化。
2. 前后端交互结构需要在 TypeScript 和 Rust 两侧保持一致。
3. 领域逻辑通过 `commands` 暴露，再在 `services` 中集中添加 Tauri 调用。
4. 外部 OJ 抓取只放入 `sync`；数据库查询和写入放入 `db`。
5. 页面局部状态留在页面中，跨页面或应用级逻辑抽到 hooks。
6. 提交前运行 `pnpm check`、`pnpm build`、Rust 测试和必要的布局回归测试。
