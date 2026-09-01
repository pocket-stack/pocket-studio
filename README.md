# Pocket Studio

Pocket Studio 是 Pocket 生态的桌面连接桥梁。项目目标是通过 USB 或网络发现设备、识别设备类型、在用户明确授权后完成准备流程，并为设备提供兼容软件的发现与安装能力。

当前版本只包含可编译的应用框架，不包含前端页面、设备识别实现、root/越狱工具或应用市场数据。

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
├── src/                   # Vue 启动入口与共享基础设施
│   ├── shared/i18n/       # 本地化配置
│   ├── shared/theme/      # light/dark/system 主题状态
│   └── styles/            # Tailwind 与设计 token
├── src-tauri/             # Tauri 应用与未来 native 业务
├── docs/architecture.md   # 模块边界与设备操作约束
├── AGENTS.md              # 工程规范
└── CLAUDE.md -> AGENTS.md
```

## 开发状态

下一阶段应先确定设备协议、受支持的具体型号以及应用包格式，再实现 native 模块。不要在设备发现阶段执行任何会改变设备状态的操作。
