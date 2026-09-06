# Pocket Studio

Pocket Studio 是 Pocket 生态的桌面连接桥梁。项目目标是通过 USB 或网络发现设备、识别设备类型、在用户明确授权后完成准备流程，并为设备提供兼容软件的发现与安装能力。

当前版本是一个**交互演示**：完整的前端界面与状态流转已经实现（设备识别、接入条件检测、越狱准备向导、DFU 引导动画、操作日志、应用商店），浏览器和 Tauri 窗口统一使用前端模拟网关，native 层保留未来接入边界，不会对任何真实设备执行操作。目标设备为 iPod touch (4th generation)，流程参照 [Legacy-iOS-Kit-rs](https://github.com/HalfSweet/Legacy-iOS-Kit-rs)。

## 演示方式

- `pnpm dev`：纯浏览器模式，使用 `src/shared/gateway/simulatedGateway.ts` 模拟 native 层。
- `pnpm tauri dev`：在 Tauri 桌面窗口展示同一套前端模拟，不调用设备命令。

底部状态栏的「演示控制」可以模拟接入/拔出设备、进入 DFU、标记越狱状态，以及为某个步骤注入失败，用于走通所有分支。

## 设计与交互

界面仅参考当前工作目录 `.design/Pocket Studio 设计规划` 中的完整预览和 5a–5k 线稿：顶部 LCD、设备侧栏、底部日志、七列应用商店、应用详情截图区与右栏。设备信息与按键说明按 iPod touch 4 调整。依据后续界面反馈，四个页面入口已合并为顶部左侧图标，原独立导航行被移除；同一行提供前进 / 后退按钮，支持设备子页与应用详情历史，准备期间禁用导航。

- 启动自动接入模拟设备，检测不会触发越狱。可在设备选择器弹出，再模拟连接。
- 前置环境页先确认备份、供电与按键条件，随后进入独立的风险、免责弹窗。风险阅读 15 秒、免责阅读 20 秒，均须前台停留并滚到底部；每项风险单独确认。
- DFU、执行步骤、进度与近期日志放在前置环境页。正常模式按键演示为 Power + Home 10 秒，再单独 Home 8 秒；恢复模式第一段为 8 秒。可重播或模拟 DFU 检测。
- 商店支持搜索、分类、详情、依赖提示、串行安装队列、取消与失败重试。已安装页支持查看与模拟卸载；被其他应用依赖的软件不能直接卸载。
- 最近 2,000 条日志保存在本机 webview 存储中。确认记录包含方案、条款版本、阅读时长和时间。导出提供可复制文本；浏览器预览也会下载 `.log`。
- 模拟设备与应用状态在刷新后重置，历史日志保留。应用目录、截图、签名和存储分布都是演示数据。

## 样式约定

组件布局、交互状态和响应式样式全部使用 Vue 模板中的 Tailwind CSS 4 工具类。`src/styles/main.css` 只保留 Tailwind 入口、主题变量、动画与基础重置，不再包含组件选择器。运行时进度、存储占比和图标尺寸通过 CSS 自定义属性传给静态工具类；不要拼接动态 Tailwind 类名。通知过渡使用 Vue TransitionGroup 的工具类属性，减少动态效果的系统偏好通过 `motion-safe` / `motion-reduce` 响应。

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
pnpm test
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
