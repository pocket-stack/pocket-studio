# 3DS 首期接入

本次工作跨 Studio、Store 与 PocketJS，**修改和提交仅保留在本地**。不推送、不创建 PR、不部署云资源。Store 直接调整 JSON v1，不保留旧格式兼容层。

## 实现与边界

- Store 保留原有表并增加 `native_identities`，通过追加 migration 保护身份归属、CIA 版本顺序及并发导入；已签名快照不重写。
- 应用以包名、SemVer、revision 标识。IPA、CIA、3DSX 与 `.pocket` 属于同一 release；SHA-256 blob 可复用。CIA 的 Title ID 来自登记，Title Version 受 16 位范围及发布顺序约束。
- Studio 的发现、已安装观察和安装驱动按平台分派。iOS 发现故障不丢弃 3DS 结果；多个设备可从现有设备菜单选择。
- 3DS 首次安装选择启动器形态或独立形态，独立形态默认 CIA、可选 3DSX。只发布了一种形态的应用直接安装；有多种形态时弹出“选择安装形态”，这是安装前唯一的对话框，方案本身不再单独确认。后续操作绑定 `installationId`；同一应用的不同容器使用不同版本、数据与回退记录。
- `.pocket` 支持首次安装到启动器，以及更新同应用的已有独立宿主。独立宿主缺失时不能用 `.pocket` 完成首次独立安装。ABI 或 Runtime 不满足时，需要更新原生宿主或准备启动器。
- 设备端安装登记与运行验证分开。上传使用 SHA-256；旧 FNV footer 仅用于包识别。应用运行失败时保留失败证据并尝试旧包；新应用无可恢复包时返回启动器。
- 安装计划固定设备绑定、安装实例、生成序号、安装格式与 blob 摘要。提交前可取消；持久化提交意图后不自动重放写入。断线后按原操作 ID 查询，再读取实际安装证据。
- CIA 由启动器通过原生 AM 服务管理，并检查 Title 与内嵌构建记录；3DSX 限定到 `3ds/<小写包名>/boot.3dsx`，FAT 大小写别名在 Store 登记阶段冲突。独立 Runtime 的管理通道只接受自身 `.pocket`。
- 系统 Title、未知原生归属、启动器自身不进入普通应用删除路径。3DS 卸载默认保留应用数据；“卸载并清除数据”是“已安装”页行内确认里的另一项选择，生成 `deleteData: true` 的方案。iOS 的现有卸载仍包含系统数据容器删除及相应确认。前端把 `deleteData` 同意值直接取自方案，不再由对话框勾选。

## 操作模型

3DS 上有两种使用 Pocket 的方式，Studio 对两者都只做“写卡 + 连接”，不碎不刷机：

|              | 装 Pocket 启动器                                                                | 不装启动器，只用自带 Runtime 的应用                                                                  |
| ------------ | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 第一步       | 通过“安装或升级启动器”把启动器 CIA/3DSX 复制到卡，FBI 或 Homebrew Launcher 安装 | 在应用详情或形态选择里“复制到 SD 卡…”，把应用自身的 CIA/3DSX 复制到卡，FBI 或 Homebrew Launcher 安装 |
| 连接         | 启动器运行时验证连接                                                            | 该应用运行时验证连接（独立 Runtime 的管理通道只接受自身）                                            |
| 之后能做什么 | 安装、更新、卸载所有应用，两种形态共存                                          | 更新这个应用的 `.pocket`；换另一个应用要再复制一次                                                   |
| 配对密钥     | `pocketjs/runtime/dev.key`，两种方式共用                                        | 同左                                                                                                 |

“验证连接”只有在主机上正在运行 PocketJS Launcher 或某个自带 Pocket Runtime 的应用时才可能成功。主机没有应答报 `hostUnreachable`；主机应答但拒绝密钥（卡上的 `dev.key` 与主机使用的不一致，或应答的不是原来配对的主机）报 `pairingRejected`；其余身份问题报 `pairingUnavailable`。

`SetupRequest.appId` 指定要复制的应用时，准备服务从已签名目录中选择该应用带 Bundled 交付的 3DS 产物，CIA 写入 `cias/<appId>-<sha256 前 16 位>.cia`，3DSX 写入登记的入口路径；不再要求该应用提供 Runtime。启动器仍按 `runtime_requirement` 选择并写入 `cias/pocket-launcher-<sha256>.cia`。

## 首次准备与启动器升级

主机 IP 与 ftpd 端口、用户名、密码保存在偏好设置的“Nintendo 3DS”一栏，仅存本机。启动时和每次刷新设备时，Studio 按该地址静默单播发现请求（同时仍在局域网广播），已配对的主机无需任何操作即可重新出现；地址留空则只靠广播。找不到主机，或首次用读卡器写卡，才走手动连接。

手动连接是设备通用入口：设备菜单中的“手动连接设备…”先列出可手动配对的设备类型（目前只有 New 系列 3DS，USB 设备自动识别），选择后决定通过“无线（ftpd）”还是“SD 卡读卡器”写入配对密钥。无线方式直接使用设置中的 ftpd 账号并给出“在设置中修改”的入口；读卡器方式需要选择 SD 根目录。连接流程只写配对密钥，不涉及启动器。先在主机上按照 [CFW 检查说明](https://3ds.hacks.guide/checking-for-cfw.html) 检查环境，必要时访问 [现行指南](https://3ds.hacks.guide/)。Studio 不执行破解或固件写入。

启动器是另一条流程：设备“前置环境”页、就绪面板以及商店里需要启动器的应用都通过“安装或升级启动器”打开它，选择 CIA 或 3DSX 后复用同样的无线/读卡器目标。文件列表和目标位置在确认前展示。SD 根目录需要已有 `Nintendo 3DS` 目录和 `boot.firm`，且两者不能是符号链接；这只是目标目录检查，不代表已经通过 CFW 或型号验收。

**Studio 只写文件，安装和启动都在主机上完成**（依据 PocketJS 仓库 `hosts/3ds/README.md` 与 `docs/3DS-LAUNCHER.md`）。写入成功后的结果页按传输方式和文件形式列出主机侧步骤：

1. 用 ftpd 的先在主机上退出 ftpd——它与 Pocket 不能同时运行，退出后 Wi-Fi 需要几秒重新连接；用读卡器的安全弹出后插回主机开机。
2. CIA：打开 FBI → SD → `cias`，选择刚复制的文件安装，再从 HOME 菜单启动。3DSX：文件已在 `3ds/<小写包名>/boot.3dsx`，从 Homebrew Launcher 启动即可。仅配对时直接启动已安装的 PocketJS Launcher 或自带 Runtime 的应用。
3. Pocket 读到 `dev.key` 才会在 TCP/UDP 8131 监听。按 L+R+SELECT 打开 Runtime 菜单：`DEV LINK` 为 `DISCOVERABLE` 或 `TCP ONLY` 并显示 `IP:端口` 表示已配对并监听；`NOT PAIRED` 表示没找到密钥。
4. 回到 Studio 点击“验证连接”。之后每次启动都按设置里的地址静默找到它。

启动器升级同样经 FBI（CIA）或替换 SD 文件（3DSX）后重启；普通管理通道不能替换或删除启动器自身。

密钥位于 `pocketjs/runtime/dev.key`，由 Rust 创建或读取，写入后回读比较；不会传给 webview、加入 Store 或写入日志。只有密钥（以及所选启动器文件）写入并回读成功之后，Studio 才登记配对；写入失败不留下配对记录，重试沿用卡上的密钥。ftpd 与现有 Runtime 开发传输使用可信局域网，**不提供传输加密**。使用 ftpd 后先退出 ftpd，再启动 Pocket。新设备的物理身份只有在用户点击“验证连接”后才会绑定，发现不会创建绑定。

**已知限制：** UDP 发现应答未经认证。局域网内任何主机若得知设备广播的 key id，都可以抢先应答，使 Studio 把明文 hello 中的配对 token 发到它那里。要消除这一点需要 PocketJS 线协议增加挑战应答（设备先证明持有 token，Studio 再发送），属于跨仓库改动，此处仅记录。

CIA 启动器文件复制到 `cias/`，用户通过 FBI 安装。3DSX 写入登记的 SD 入口。重启启动器后验证版本与连接。升级启动器使用相同准备流程；CIA 升级仍经 FBI，3DSX 经 SD 替换。未配置或尚未发布启动器的 Store 不影响手动连接：连接流程本身就只写配对密钥。

## 跨仓库协议

Store 中 `native_identity_json` 为分类 JSON 或 SQL NULL；`requirements_json` 保存平台、架构、型号、OS、host ABI、`runtime_deliveries`、安装器、按平台分类的前置条件及 Runtime requirement/provides。JSON 在 SQLite/D1 使用 TEXT，时间为毫秒 INTEGER。

设备、配对密钥、安装实例、运行状态与操作日志保存在 Studio 或设备端，不写入发布数据库。`contracts/store-v1` 是 Store 的签名协议副本，前端由其生成类型。PocketJS 的管理接口定义在 `contracts/spec/3ds-management.ts`。

设备提交后若只有意图、没有可证明的登记或实际原生结果，状态保留“待验证”，禁止用原操作重放写入。损坏或缺失的设备记录不等同于成功卸载。运行成功与文件安装成功分别展示。

## 浏览器演示

浏览器演示只有一台模拟设备。“连接 3DS”走与原生层相同的 `setup.plan/execute/connect`，连接成功后 New Nintendo 3DS LL（型号码 `RED`）替换 iPod；演示面板的“模拟接入 New 3DS LL”直接得到同一台主机。六个 3DS 演示应用带 `details.candidates`，按后端的安装实例模型走 `store.plan/start`；`pocket-runtime-ctr` 是启动器本身，商店里始终显示“需要前置环境”，只能通过准备对话框写入。图标来自 `public/vectors/packages`，目录没有 icon 媒体时按包名回退到这些矢量图。

## 本地验收

```sh
pnpm check
pnpm test
pnpm test:native
pnpm test:ui
pnpm test:ui:3ds
```

Store：`pnpm check`、`pnpm test`。PocketJS：`bun test tests/3ds-*.test.ts`、`bun test --timeout 60000 tests/launcher-sim.test.ts`、`bun tests/contract.ts`，并执行启动器原生构建及 `tests/e2e/launcher-azahar.ts`。

已加入的自动检查覆盖签名与迁移、原生身份/构建记录、包摘要、ABI、安装实例选择、旧状态拒绝、SD 目标变化、密钥回读、TCP 分片与错误配对、操作不重放、回退存储，以及中英文、明暗主题、960×618 下的手动连接（设备类型列表、无线摘要、读卡器路径）、启动器安装、形态选择、单形态直接安装、两种形态共存、行内卸载与清除数据。iPod 页面仍通过最小、默认和大窗口回归。

**尚未完成的设备验收必须单独记录，不得由模拟结果代替。** 首批目标是 New 系列，New 3DS LL 为真机验收机；其他 New 型号标记未验收，Old 系列不声明支持。

真机检查由用户逐项发起：启动器首次安装；两个 `.pocket` 应用安装、更新、切换；首次启动失败与旧包回退；独立 CIA 安装、更新、卸载；同应用两种形态数据隔离；3DSX 回读；启动器升级；断线、IP 变化及重启后的原操作查询。测试开始前确认目标 SD 卡及主机身份。

## 本次运行记录

2026-09-08：Studio 原生测试 92 项通过（2 项既有 iPod 真机检查跳过），前端测试 48 项通过。原有页面通过三种窗口尺寸；3DS 准备、两种形态共存和分别卸载通过中英文、明暗主题组合检查。

Azahar 2126.0 使用隔离的 XDG 数据目录。已通过管理通道安装两个动态应用、中文标题/无封面、Guest 切换、失败更新回退和首次启动失败返回启动器。Studio Rust `verify_three_ds` 读取了该实例的设备信息及安装记录。CIA、3DSX 经过实际构建、Store 导入，以及模拟器中的原生安装、读取和卸载。独立宿主的 `.pocket` 更新也通过了原生版本保留与其他实例不变的验证；这些结果不声明任何真机型号通过验收。

`src-tauri/examples/verify_three_ds.rs` 只读取目标 Runtime：参数为 IP:port 和密钥文件路径，不接受命令行明文密钥。可通过 `STUDIO_3DS_VERIFIER` 将编译后的示例程序接入 PocketJS 的启动器模拟器测试。

### 2026-09-10 逻辑修复

- 配对记录改为在密钥与启动器文件写入并回读成功后才登记；`connect` 指定地址失败时恢复原地址。
- 连接的宿主是否有权管理目标应用改为按设备上报的 `host_app_id` 与 `launcher` 判断：启动器不能安装或卸载自身，目录里 `runtime` 类别的应用只能通过准备流程处理，独立 Runtime 只能更新自己的 `.pocket`；该规则在方案生成和提交前各检查一次。
- 独立形态的 `.pocket` 更新按独立宿主自身的 ABI 与 Runtime 版本评估，不再同时受启动器的 ABI 约束；更新时显式指定 `format: "pocket"` 只更新客体，指定原生格式则更新宿主，其他格式拒绝。
- 已安装 revision 未知且版本相同的更新要求显式重装；超过 24 MiB 的 `.pocket` 归为不支持而非需要准备。
- 卸载校验先检查记录是否可读，损坏记录报“无法核实”，仍在装态报失败；设备上报的操作状态仍是删除成功的前提。
- ftpd 回读失败后删除暂存文件；上传阶段本地读取失败也向设备发送 abort。
- SD 根目录标记不再接受符号链接；密钥文件只接受十六进制字符；启动器候选接受全部 New 系列型号；迁移失败时按操作 ID 记录错误日志。

### 已有 Studio 缓存的启动迁移

旧 iOS 安装任务使用平铺的 `bundleId/previous/appsync/jailbreak`。启动时，Studio 在 SQLite 事务中迁移这些私有任务记录及其内嵌产物条件，将原文保存在 `package_job_migration_backups`；操作 ID、设备绑定、已提交状态和历史结果保持不变。任何一条记录无法转换时，事务回滚。中断的任务只恢复为待验证状态，不重放设备写入。公开的 JSON v1 协议及已签名目录快照不走此转换。

可失败的服务初始化在进入 Tauri 事件循环前完成，初始化失败会返回正常错误，避免在 macOS 原生事件回调中触发 panic/abort。

### ftpd 传输与首次准备

FTP 协议使用 [suppaftp](https://docs.rs/suppaftp/11.0.0/suppaftp/) 的 Tokio 客户端，负责登录、被动连接、响应解析、传输和重命名。Studio 保留超时和下载大小限制；数据连接固定使用控制连接的 IP；文件先上传到临时路径，回读一致后重命名，再回读最终文件。第三方协议日志被应用的日志过滤器排除，避免输出凭据和设备路径。

ftpd 的 `CWD` 成功响应为 `200`，部分 FTP 服务使用 `250`，由库处理。创建目录兼容 ftpd 的 `250` 与库接受的 `200/257`，目录已存在时通过 CWD 校验。首次准备读取尚不存在的密钥或启动器时，ftpd 在父目录缺失时返回 `553`，文件缺失时返回 `450`；仅明确的文件不存在响应允许继续生成方案，权限拒绝或非法文件名仍然失败。FTP 连接、登录和远端 SD 目录不可访问分别报告错误，不提示用户选择电脑上的 SD 卡路径。

更换客户端后，已连接实际 ftpd，通过 `SetupService::plan` 完成包含密钥、启动器路径检查和 Store 产物选择的只读准备流程，成功生成 3DSX 方案。上传、临时文件回读、重命名和最终回读由本地 FTP 模拟服务器验证；本次未向真机写入文件，不计为真机安装验收。
