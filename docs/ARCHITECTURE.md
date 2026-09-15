# 项目结构

OJ Insight 是一个 Tauri 2 桌面应用。前端负责界面与交互，Rust 后端负责数据同步、SQLite 存储和系统能力。

## 前端

```text
src/
├─ components/  可复用的展示组件
├─ hooks/       可复用的 React 状态与布局逻辑
├─ lib/         日期、平台配置等纯工具和领域辅助函数
├─ pages/       对应侧栏入口的页面组件
├─ services/    Tauri 命令、更新、导出等外部交互
├─ App.tsx      页面编排和应用级状态
├─ main.tsx     React 入口
└─ types.ts     前后端共享数据形状的 TypeScript 定义
```

依赖方向保持为：页面和组件可以使用 `hooks`、`lib`、`services`；`lib` 不应依赖 React 页面或组件；Tauri `invoke` 调用集中放在 `services`。

## Rust 后端

```text
src-tauri/src/
├─ app/             应用状态与资源初始化
├─ commands/        Tauri 命令入口和参数校验
├─ infrastructure/  路径、日志等基础能力
├─ sync/            各平台数据获取、标准化与同步流程编排
├─ db.rs            SQLite 读写与统计查询
├─ models.rs        后端领域模型
├─ operation.rs     并发操作保护
├─ xcpc.rs          XCPC 目录与 Rating 数据
└─ lib.rs           插件、窗口、状态和命令注册
```

命令层只负责接收参数、校验并调用领域代码。平台抓取和同步流程编排放入 `sync`，数据库查询集中在 `db.rs`，应用组装只保留在 `lib.rs`。

## 新增功能时

1. 在 `types.ts` 与 `models.rs` 中确认数据结构一致。
2. 在对应 Rust 领域模块实现逻辑，再通过 `commands` 暴露命令。
3. 在 `services/api.ts` 中集中添加 Tauri 命令调用；文件对话框、更新器等外部交互也放在 `services`。
4. 页面状态留在 `pages` 或 `App.tsx`；可复用的 React 逻辑放入 `hooks`。
5. 提交前运行 `pnpm check`、`pnpm build`、`cargo fmt --check` 和 Rust 测试。
