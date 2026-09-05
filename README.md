# Pocket Studio

Pocket Studio 是 Pocket 生态的桌面连接桥梁。项目目标是通过 USB 或网络发现设备、识别设备类型、在用户明确授权后完成准备流程，并为设备提供兼容软件的发现与安装能力。

当前版本是一个**交互演示**：完整的前端界面与状态流转已经实现（设备识别、接入条件检测、越狱准备向导、DFU 引导动画、操作日志、应用商店），native 层只提供模拟驱动，不会对任何真实设备执行操作。目标设备为 iPod touch (4th generation)，流程参照 [Legacy-iOS-Kit-rs](https://github.com/HalfSweet/Legacy-iOS-Kit-rs)。

## 演示方式

- `pnpm dev`：纯浏览器模式，使用 `src/shared/gateway/simulatedGateway.ts` 模拟 native 层。
- `pnpm tauri dev`：Tauri 模式，使用 Rust 侧 `infrastructure/demo` 驱动，事件流与浏览器模式一致。

侧边栏底部的「演示控制」可以模拟接入/拔出设备、进入 DFU、标记越狱状态，以及为某个步骤注入失败，用于走通所有分支。

## 技术栈

- Tauri 2 + Rust
- Vue 3 + TypeScript + Vite
- Tailwind CSS 4
- Vue I18n
- `tracing`、`thiserror`、`anyhow`
- pnpm + lefthook

## 开发环境

先按照 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) 安装当前系统所需依赖，然后执行：

```sh
pnpm install
pnpm tauri dev
```

仅启动 webview 开发服务器：

```sh
pnpm dev
```

## 质量检查

```sh
pnpm check
pnpm test:native
pnpm build
```

lefthook 会在提交前执行前端检查、`cargo fmt --check` 和 Clippy。提交信息遵循 Conventional Commits。

## 目录

```text
.
├── src/
│   ├── app/               # 壳层组件（演示控制、通知）
│   ├── features/          # device / preparation / store / logs / settings
│   ├── shared/gateway/    # 类型化 native 网关（Tauri 实现 + 浏览器模拟）
│   ├── shared/composables # 设备会话、操作跟踪、日志、通知
│   ├── shared/ui/         # 通用组件（进度、步骤、强制阅读、设备示意图）
│   ├── shared/i18n/       # 按命名空间拆分的 zh-CN / en 文案
│   ├── shared/theme/      # light/dark/system 主题状态
│   └── styles/            # Tailwind 与设计 token
├── src-tauri/src/
│   ├── domain/            # 设备、就绪规则、准备方案与同意校验、目录、日志模型
│   ├── application/       # ports 与 Studio 用例
│   ├── infrastructure/    # demo 驱动（模拟事件流）
│   └── commands/          # Tauri 命令边界
├── docs/architecture.md   # 模块边界与设备操作约束
├── docs/device-preparation-flow.md  # 准备向导状态机与同意规则
├── AGENTS.md              # 工程规范
└── CLAUDE.md -> AGENTS.md
```

## 开发状态

下一阶段用基于 Legacy-iOS-Kit-rs 的真实适配器替换 `infrastructure/demo`，保持 port、领域规则与事件流不变。不要在设备发现阶段执行任何会改变设备状态的操作。
