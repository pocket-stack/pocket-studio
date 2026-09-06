# Pocket Studio

Pocket Studio 是 Pocket 生态的桌面连接桥梁。项目目标是通过 USB 或网络发现设备、识别设备类型、在用户明确授权后完成准备流程，并为设备提供兼容软件的发现与安装能力。

当前版本已开始接入真实设备：**Tauri 桌面窗口使用 Legacy-iOS-Kit-rs 读取 USB 设备，浏览器保留前端交互演示**。本阶段完成设备发现、设备信息读取、只读接入条件检测、连接诊断与日志。真实越狱、前置环境安装、应用安装和卸载尚未接入，后端会明确拒绝这些操作。

适配器固定使用 [Legacy-iOS-Kit-rs `42b423f`](https://github.com/HalfSweet/Legacy-iOS-Kit-rs/tree/42b423fbfdcda66f1fb7cca2b605654dd905087e) 的 services、transport、assets 和 core crate；Rust 最低版本为 1.88。原生模拟驱动已移除，模拟数据仅存在于浏览器网关。

## 运行方式

- `pnpm dev`：纯浏览器模式，使用 `src/shared/gateway/simulatedGateway.ts` 模拟 native 层。
- `pnpm tauri dev`：启动真实 USB 只读接入，不会自动配对、越狱或安装。

**浏览器模式**底部状态栏的「演示控制」可以模拟接入/拔出设备、进入 DFU、标记越狱状态，以及为某个步骤注入失败，用于走通所有分支。

## 真实设备接入

- 自动扫描与手动重新检测使用同一个 Rust 适配器；当前每 3 秒刷新一次，读取有超时限制。
- 多台设备可以通过顶部设备菜单选择。扫描结果带递增版本，旧响应不会恢复已经断开的设备。
- 正常模式通过系统 usbmux 复用已有配对；DFU、Recovery 等模式只枚举 USB 信息，不切换模式。
- 受保护字段无法读取时显示「未读取」。未发现 AFC2 不等于未越狱；通过根目录访问，或通过 SpringBoard 中的 Cydia 注册信息与 SSH 响应共同确认越狱。SSH 只读取服务标识，不登录或执行命令。
- 完整 UDID、ECID、序列号保留在适配器内部；界面使用临时会话 ID 和脱敏标识。
- Windows 需要 Apple 设备驱动与 Apple Mobile Device Service；Linux 需要 usbmuxd 与适当的 USB 权限；macOS 使用系统服务。应用不会自动重启服务、切换驱动或提升权限。

macOS / iPod touch 4 真机验证已读取 `iPod4,1`、`N81AP`、`A4`、`6.1.6`、`10B500`、脱敏序列号、电量与存储信息，配对、越狱及 SSH 检测均通过。设备使用 Legacy iOS Kit 默认路径越狱：AFC2 和安装服务列表未提供足够证据，SpringBoard 能确认 Cydia 注册信息。

设备会话按 libimobiledevice 的规则兼容旧协议：iOS 7 之前先验证已有配对，iOS 10 之前使用 TLS 1.0；设备证书必须匹配既有配对记录。兼容实现已移入 Legacy-iOS-Kit-rs；Studio 启用库的 `legacy-tls` 功能并调用 `NormalDevice::inspect()`，应用内不再保留独立 TLS 实现。库使用静态链接的 OpenSSL，仅作用于 USB 设备会话。构建需 C 编译器、Make 和 Perl；程序运行时无需另外安装 OpenSSL。分析与测试说明见 [旧版 iOS 连接兼容性](docs/legacy-ios-compatibility.md)。

可以使用与桌面应用相同的适配器执行一次只读硬件检查（不属于常规自动测试）：

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example detect_devices
```

## 浏览器演示与交互

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
│   ├── infrastructure/    # Legacy iOS Kit 只读适配器与三平台策略
│   └── commands/          # Tauri 命令边界
├── docs/architecture.md   # 模块边界与设备操作约束
├── docs/device-preparation-flow.md  # 准备向导状态机与同意规则
├── AGENTS.md              # 工程规范
└── CLAUDE.md -> AGENTS.md
```

## 开发状态

下一阶段接入需要明确授权的准备和应用管理工作流。浏览器演示的成功状态不会用于判断真实设备。不要在设备发现阶段执行任何会改变设备状态的操作。
