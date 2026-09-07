# 原生设备准备

当前执行链仅支持 iPod touch 4（iPod4,1 / n81）、iOS 6.1.6（10B500）、A4 / iBoot-574.4。没有扩大设备就绪矩阵，也不会把越狱未知自动转换成未越狱。

## 用户入口与授权

用户可以在接入条件页查看方案，也可以进入前置环境页。方案生成只读。正常模式下，未越狱设备的配对失败、电量未知或低于 50%、型号或固件不符都会得到明确错误。已越狱且 SSH 可用的设备进入独立的 AppSync 准备方案。DFU 入口通过 CPID `0x8930`、BDID `0x08`、ECID 和 iBoot-574.4 校验硬件；系统、电量、配对不能读取，界面明确提示，保留电量确认并在 ramdisk 中校验实际系统。

原生方案先下载和构建，再请求用户进入 DFU。`entryMode: dfu` 的方案省去进入 DFU 的步骤和按键可用性确认。概览列出步骤、系统影响、风险和取消边界。越狱方案的风险与免责各等待 5 秒后点击一次确认，切出软件继续计时，不要求逐项勾选或滚到底部。授权版本为 `2026-09-07`，包含 root ramdisk 会话、分区挂载、Aquila 6 / Cydia / OpenSSH 安装；原来的“仅用于演示”条款已移除。浏览器模式另行显示模拟提示。

Rust 校验所有风险和前提、方案 ID、条款版本、阅读时长、阅读时间顺序、未来时间和 30 分钟有效期。方案在开始时被消费，不能复用；失败后的重试重新生成方案并收集确认。可观察到的正常模式会话更换会使旧目标校验失败。授权详情和步骤结果写入操作日志，导出保留参数。

## 执行与身份

1. 获取并校验所有固件与载荷，在电脑上构建临时 ramdisk。
2. 按入口模式重新验证设备。正常模式检查配对、电量与固件，再等待手动 DFU；DFU 模式重新检查原 ECID 的芯片、主板和 bootrom，直接继续。
3. 只对预检时的 ECID 发送 A4 limera1n 载荷，并确认 pwned DFU 标记；已经通过硬件校验且带 pwned 标记的 DFU 会复用现有状态。
4. 通过同一 ECID 启动构建的 ramdisk。临时镜像带有随机操作标记。
5. 在该 ECID 对应的 USB 端口匹配 usbmux 的序列号与设备 ID，连接临时 ramdisk 的 root SSH。会话只接受本次镜像的随机标记；不会回退为“第一台设备”。此处使用临时主机密钥策略和 ramdisk 公开默认密码，不连接正常 iOS 的特权 shell。
6. 只读挂载系统分区（不请求 fsck 或可写挂载），核对磁盘中的 6.1.6 / 10B500，再检查 `/mnt1/bin/bash`，避免重复安装。
7. 核验通过后将系统分区重新挂载为可写，再挂载数据分区，安装经过校验的 fstab、Aquila 6、Cydia、SSH deb、OpenSSH、OpenSSL、LukeZGD 和 nopatcyh 资源。严格检查每个命令的退出状态；tar 失败不会因清理命令成功而被忽略。
8. 检查 Aquila、Cydia 和 sshd 文件，调用 ramdisk 原有的 `reboot_bak` 刷新文件系统并重启，等待原 ECID 对应的设备返回（正常入口同时绑定原 UDID，DFU 入口按 ECID 解析返回的 UDID），再建立既有配对会话，再确认越狱与 SSH。重启连接丢失本身不算成功。

下载、构建和等待 DFU 可取消。limera1n 及其后续步骤不可取消；只读核验通过后，从重新挂载为可写并安装时标记系统修改边界。单个 native 进程只允许一个准备操作；关键步骤失败后不会自动重试或回滚。超时后可能存在部分写入，因此结果要求先检查设备状态。正常关闭窗口和退出会被运行中的准备操作阻止，并显示原因；不能防止强制结束进程、断电或系统崩溃。

## 资源与实现依据

- 本地 `../../../GitHub/Legacy-iOS-Kit-rs`（相对 `src-tauri`）：USB 传输、A4 利用、镜像处理、HFS 及 ramdisk 启动。当前直接修改并构建此库，不要求先推送远程仓库。
- [Legacy iOS Kit 1ff4be0 的 device_ramdisk](https://github.com/LukeZGD/Legacy-iOS-Kit/blob/1ff4be07ea2946ccaeff2db60c4426488b8f6e32/restore.sh)：n81 / 10B500 组件、boot args、SSH ramdisk 组合与越狱资源次序。
- [固定固件密钥元数据](https://github.com/LukeZGD/Legacy-iOS-Kit-Keys/blob/af6bf5934dc61ed557a967a3f42ab7fb8ed8c45e/iPod4%2C1/10B500/index.html)。元数据也经过 SHA-256 校验，不输出其内容。
- [ipwndfu 0e28932 的 A4 载荷参数](https://github.com/axi0mX/ipwndfu/blob/0e28932ec6a2a570b10fd77e50bda4216418cd98/limera1n.py)：下载同版本 shellcode 后验证占位符，再应用 `constants_574_4`。库的 `A4Limera1n` 在其前方添加 16 个 64 字节堆头，A4 入口地址为 `0x84000401`。

`preparation-assets.json` 固定来源 URL、大小及 SHA-256。Apple 完整 IPSW 为 888,894,104 字节，来源是 Apple HTTPS；在原始下载的 SHA-1 与公开固件记录一致后计算并固定 SHA-256，各组件另有摘要。其余安装包沿用库的固定资源目录。二进制、固件、密钥和 SSH 资源均在运行时下载，不随仓库分发；来源项目的许可仍适用。

此前远程 ZIP 读取器把 HEAD 响应的空正文长度当成固件长度，导致 `invalid remote ZIP archive`；本地库已修正为读取有效的 `Content-Length` 头，并添加本地 HTTP 回归测试。Studio 仍使用完整固件下载与摘要校验后再读取组件，以保留当前整包认证边界。没有静默回退为未验证下载。首次需要约 889 MB 固件空间，之后复用校验缓存；临时构建目录由具体操作拥有并自动删除。

SSH tar 含 `./` 根目录条目和 `etc/` 等目录，而 stock ramdisk 的 `/etc`、`/var`、`/tmp` 是符号链接。库的 `HfsImage::untar` 跳过根目录元数据，仅在镜像内解析目录链接，保留 stock 链接。归档成员仍拒绝绝对路径与父目录路径；链接解析限制在镜像根目录内，并拒绝循环链接。Studio 直接调用库接口，此兼容问题的回归测试也已移入 image crate。

库的 firmware crate 固定 `serde_json = 1.0.140`；锁文件将 Tauri 的兼容传递依赖 `serde_with` 固定到 3.14.1，以解决 3.17.0 要求更高 serde_json 的冲突。

## 平台与验证范围

macOS、Linux、Windows 都使用已有系统 usbmux 与共享 Rust USB / SSH 实现。平台诊断沿用 HostEnvironment；缺少驱动、USB 权限或系统服务时失败，不自动安装驱动、重启服务或提升主机权限。

已完成 macOS 正常模式及 DFU 真机只读检测与方案预检，以及真实固件下载校验、iBSS / iBEC 补丁、32 MB SSH ramdisk 构建和 8 个安装资源包校验。自动测试覆盖授权时序与重放、并发互斥、取消边界、失败终止、事件字段、资源损坏及 HFS 目录链接兼容。

旧版 limera1n 执行曾在 USB 控制传输阶段失败，后续只读 GETSTATE 也观察到 IOKit `0xe0004051` 事务超时。此前紧凑 A4 流程已观察到真机 PWND 标记，随后 ramdisk 上传停在 DFU 状态 8；经过后续引导及 HFS 修复，macOS 真机的临时引导、USB SSH 和只读系统核验已经通过；后续 macOS 真机的越狱写入和正常重启检测也已通过。Linux/Windows 的真实写入仍未经硬件验收。产品中的新准备操作仍需用户在桌面应用确认备份与风险、主动启动并按引导进入 DFU。

以下诊断工具位于 Git 忽略的 `src-tauri/examples/`，不随仓库分发；仅在本机保留对应文件时可运行。只验证电脑上的资源步骤：

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example prepare_resources -- /path/to/cache
```

## DFU 入口的只读验收

DFU 主板映射依据 [libirecovery 的设备表](https://github.com/libimobiledevice/libirecovery/blob/master/src/libirecovery.c)：n81ap 为 CPID `0x8930` / BDID `0x08`。同一 A4 芯片的其他主板不能选择 iPod 固件。DFU 会话键包含内部 ECID，避免在相同 USB 端口更换设备后复用旧方案；完整标识不对外输出。重启后多台正常模式设备按原 ECID 精确匹配，重复身份会被拒绝。

本机保留 `detect_devices.rs` 时，连接已经在 DFU 的设备，运行以下检查只生成方案，不下载、不引导、不写入：

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example detect_devices -- --plan-preparation
```

## macOS A4 利用流程对齐

以本地 Legacy-iOS-Kit 的 `restore.sh`（A4 / macOS 分支调用 `ipwnder -pv`）和其 `ipwnder_lite` 原始实现为对照，本地库在 `crates/exploits/src/a4_limera1n.rs` 协调以下序列：USB 重置/重新枚举并绑定原 ECID → 一次发送紧凑堆头与载荷 → 1 字节预备读取 → 短时传输与触发 → 再次重新枚举 → 结束传输与三次状态请求 → 最后重新枚举并验证 PWND 标记。Studio 直接调用 `A4Limera1n`，原有临时实现及直接 nusb 依赖已移除；库的 facade 同样对 A4 选择这一入口。整个流程不调用本地主机可执行文件。

旧路径采用另一种大缓冲区堆喷布局，缺少前置重连，且触发和结束传输之间未重新枚举。紧凑布局与当前固定的 ipwndfu 载荷配套。原版的参考实现见 [ipwnder_lite limera1n](https://github.com/dora2ios/ipwnder_lite/blob/cf6d1e6e60727e79c281cd559ebcafd43149e4c1/src/exploit/limera1n.c)。

预备读取或载荷发送失败会立即停止；协议故意触发的短时传输和结束阶段允许 stall/timeout，但这些结果不能当作成功，最终必须观察到原设备的 PWND 标记。错误携带固定 `diagnostic.stage` / `diagnostic.reason`，同时进入界面、事件和可导出日志，不含设备标识或载荷。nusb 0.2.7 将 macOS USB 超时和取消合并，因此显示“超时或被取消”，不伪装成更精确的原因。

回归测试独立列出 USB 请求顺序、长度和超时边界，覆盖载荷布局、预备读取失败、载荷发送失败、最终无 PWND 证据以及诊断信息不能在应用层丢失。

另外，原先的 `IbootClient::reset()` 和 `upload_image()` 在调用 nusb 重置时仍持有已 claim 的接口。真机重现会得到“cannot perform this operation while interfaces are claimed”，导致上传后的 DFU 状态 8 无法继续。库现统一先释放接口，再通过保留的设备句柄执行复位；上传路径复用这一实现并保留错误。ramdisk 引导在重枚举前等待 1 秒，并继续按原 ECID 重连；没有更改系统服务或驱动。

库向嵌入方报告组件上传、引导命令和脱敏错误分类。Studio 将其映射为中英文的具体阶段（例如上传 iBSS、运行 iBEC、激活 ramdisk、启动内核），保留在失败事件和导出日志中。安装归档的设备端命令也已同步到库：仅在 tar 成功后清理临时包，避免清理成功掩盖安装失败。

2026-09-07 的同链路真机诊断中，修正前 GETSTATE 约 2.2 秒超时，释放接口并重新枚举后约 0.9 毫秒返回状态 5。该检查只进行了 USB 重枚举和状态读取，没有发送利用载荷或写入设备文件系统。此结果验证了控制通信恢复，尚不代表完整利用、ramdisk 启动或安装成功。

## IMG3 引导镜像的原版对照

2026-09-07 的早期真机重试通过 iBSS 上传和 USB 复位后，设备没有重新枚举，未进入 SSH 或安装步骤。使用本地 Legacy-iOS-Kit 的 `xpwntool` 与 `iBoot32Patcher` 仅在电脑上对照相同输入，定位到两个库问题：

- `decrypt_img3_payload` 保留了 KBAG，加密标记与明文 DATA 同时存在。现与 `xpwntool -decrypt` 一样移除全部 KBAG，并更新容器长度和 SHSH 偏移，保留其他元素及填充。
- `debug-enabled` 的 Thumb 补丁字节序错误，现写入两个正确的 `MOVS R0, #1` 指令。

修正后，iBSS 与 iBEC 的解密 IMG3、原始载荷、修补载荷和最终 IMG3 均与原版逐字节一致。原版可执行文件仅用于离线对照，应用仍只运行 Rust 实现。

同次诊断验证了 DFU 状态 8 经 ABORT 返回状态 2。库在显式的新镜像上传前执行这一清理并重新读取状态；只在确实返回空闲后上传新内容，不执行旧缓冲区中的镜像。状态清理与镜像封装均有回归测试。后续已完成引导与 SSH 验收，结果见下节。

## ramdisk 启动与文件系统校验进展

修复 HFS 之前的真机尝试已通过 A4 PWND 验证、iBSS 和 iBEC，进入 Recovery。A4 的 `getenv ramdisk-delay` 返回 STALL，而 `ramdisk` 激活正常；库现仅对原版注明需要该查询的 S5L8900 发送它，其他必要命令的错误继续保留。从当前 Recovery 继续后，ramdisk、设备树和内核已上传，全部引导命令完成，但 100 秒内没有 USB SSH，未挂载或修改设备磁盘。设备之后返回正常 iOS 6.1.6 / 10B500。

对完整生成镜像执行电脑上的只读 `fsck_hfs`，发现并在本地库修正了三个 HFS 问题：新建目录记录缺少末尾字段（应为 88 字节）、覆盖文件后遗留孤立的扩展属性、32,000,000 字节卷的备份头未放在实际卷尾前 1024 字节。修正后的完整 ramdisk 已通过目录树、目录层级、扩展属性、空间位图和卷头检查，`fsck_hfs -fn` 返回 0。检查只附加电脑上的临时镜像为只读且不挂载文件系统，随后解除附加；应用执行链不依赖此工具。

iBSS、iBEC、设备树和内核与本地原版工具的对应输出一致。完成 HFS 修复后，从新的 DFU 状态使用 Studio 同一原生准备适配器进行真机验收：A4 PWND、iBSS / iBEC、ramdisk 引导、USB SSH、随机会话标记、只读挂载和磁盘中的 6.1.6 / 10B500 核验全部通过。`retry_ramdisk --boot-and-check-readonly` 输出 `PASS MountFilesystem` 并正常退出。该验收没有执行 InstallUntether 或重启后的越狱验证；设备留在临时 ramdisk，系统分区只读。

## 安装与重启验收

用户明确要求继续后，在现有 ramdisk 上通过 restored 的 HardwareInfo 查询核对 A4、n81 和 ECID，确认 RAM 根文件系统、只读系统分区、6.1.6 / 10B500 以及尚未越狱。随后实际写入 fstab、Aquila 6、Cydia bootstrap、SSH deb、OpenSSH、OpenSSL、LukeZGD 和 nopatcyh 共 8 个已校验资源包，每包的上传与解包均成功，安装文件核验通过。

本轮修正了应用执行链对精简 ramdisk 工具的两个错误假设：

- ramdisk 没有 `umount`，改用其支持的 `/sbin/mount -u -w /mnt1` 原地切换为可写，并在真机确认挂载状态。
- ramdisk 没有独立 `sync` 工具，文件核验不再调用它；重启使用原版 `reboot_bak`，该二进制包含自身的文件系统刷新逻辑。安装核验同时检查 Aquila 执行文件与 launchd 配置。

设备成功回到主屏幕。正常系统的 Studio 原生发现与条件检测确认 iPod4,1 / 6.1.6 / 10B500，配对、越狱和 SSH 均通过，报告状态为 `ready`。独立诊断脚本曾在等待重启结果时超时，随后通过正常模式的原生发现完成验收。脚本现与产品中的 DFU 入口一致，按 restored 读取的 ECID 识别重启后的设备，避免依赖跨模式可能变化的 USB 标识。

本地 `continue_installation` 示例仍由 Git 忽略；它使用开发依赖中的 restore crate，只调用 QueryType / QueryValue 做只读身份查询，不调用 StartRestore。`--inspect` 仅做预检；安装与重启的诊断选项必须由用户明确启动。产品的授权和会话标记校验保持不变。

## 重启后验证超时与设备发现

用户在完整桌面流程中遇到：设备已回到主屏幕且 USB 已枚举，Studio 却在最终验证超时后报告零设备。随后独立检测和应用均确认越狱、SSH 与接入条件通过。稳定状态下的只读并发探测未重现当时的启动瞬态，因此没有将某一条请求武断认定为唯一原因；代码审查确认了 15 秒验证预算短于库的 20 秒检查预算，以及发现总超时会丢弃已枚举设备的问题。

现为重启/最终验证保留普通模式读取通道，后台此时仍更新连接列表，检测事实保持未知。单设备读取超时不再清空 USB 中实际存在的设备；未配对字段查询仅在完整检测失败后使用。并行设备检查有独立时限，手动刷新复用正在进行的后台扫描。

验证超时显示“安装结果待确认”，可以点击“重新检测设备”查看接入条件，不再引导直接重复安装。自动测试覆盖读取所有权、超时保留连接、刷新合并、完成写入后的未确认错误和只读按钮行为。已有越狱设备可显式运行以下硬件测试：

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib read_only_post_reboot_verification_with_background_discovery -- --ignored --nocapture
```

该测试直接执行原生最终验证步骤，确认后台发现保留连接且验证完成后恢复完整检测；不会重启或安装软件。2026-09-07 在当前 iPod touch 4 上通过，约 0.75 秒完成。

## AppSync 应用安装环境

AppSync Unified 允许安装开发用未签名或伪签名 IPA。商店目录签名证明目录来自可信发布者，IPA 的系统签名检查由设备完成，两者独立。系统报告签名拒绝时，商店显示具体诊断并提供前置环境入口。

- 首次环境准备：原有越狱、重启和 SSH 验证后继续连接正常系统、安装 AppSync 依赖、重载安装服务并验证系统包记录。
- 已越狱设备：直接生成 USB SSH 方案，不包含 DFU、漏洞利用、untether 或重启设备步骤。仍需概览、风险及系统修改确认；不要求倒计时或物理按键。缺少 SSH 时说明如何在设备中补齐。
- 两种方案均仅支持现有的 iPod4,1 / iOS 6.1.6 / 10B500。root SSH 密码在概览输入，仅驻留内存，单独传入执行命令，不写入授权记录、日志或配置。默认值为 alpine，可修改。正常 USB 设备由 UDID / ECID 绑定；独立方案先无认证读取 SSH 主机公钥，实际认证固定该指纹，连接后再次读取固件与型号。
- AppSync 状态独立为已安装、未安装、未知。常规探测尝试只读系统包状态；显式准备后可以复用已认证 SSH 会话只读检查，断开时清除会话，不保存密码。读取失败不会被解释成缺少 AppSync。AppSync 未知仍允许用户发起普通 IPA 安装。

固定资源清单位于 `src-tauri/src/infrastructure/legacy_ios/appsync-assets.json`：AppSync Unified 116.0 来自作者 GitHub Release；Cydia Substrate 0.9.6301 和 Substrate Safe Mode 0.9.6001 来自 apt.saurik.com。固定大小与 SHA-256，下载后及上传后分别验证。包元数据和原始资源摘要已在开发机核对。

通过系统 dpkg 预演依赖检查后执行安装，保留已正确安装的同版本或较新版本，拒绝降级较新的损坏包；不使用强制依赖覆盖或自动卸载冲突包。安装后核对三项系统包均已配置且版本满足要求，再清理本次 UUID 暂存目录。失败可能留下已完成的系统包变更和本次暂存日志；重新检查后显式生成新方案，不自动重放安装。安装服务重载不等于插件加载已验证，仍需要真实 IPA 安装验收。

下载及连接阶段可取消；开始传输系统包后不可取消，并复用设备写互斥和关闭保护。测试覆盖独立方案不执行越狱、空密码拒绝、敏感信息不进入事件或日志、安装阶段取消边界、依赖失败阻止写入、保留较新版本、损坏状态和精确错误文案。脚本契约测试仅执行本机 shell 中的 dpkg 模拟函数，不调用设备或本机包管理器。

本次 AppSync 真机写入、插件实际加载和后续 IPA 安装尚待用户在 Studio 中显式启动验证；既有越狱真机验收不作为 AppSync 的验收结果。

### 重复准备与激活待确认

真机诊断确认三个包均为 `install ok installed`，但 `launchctl` 返回 `launch_msg(): Socket is not connected`。设备重启后再次准备也会遇到同一个服务控制错误；它不证明 AppSync 包安装失败，也不证明商店 IPA 安装失败。

现在先只读核对所有系统包的版本和配置状态：已满足要求时不上传、不重装、不重复重载服务。实际发生包变更才尝试激活；在发出“已安装，请重启设备”提示之前，必须先核对三个包记录。激活失败使用独立错误分类，结果页突出手动关机再开机的操作及“已重启，重新检测”按钮；按钮只触发检测，不自动重启或再次写入。成功文案仅承诺已核对安装记录，插件加载及真实 IPA 安装仍需单独验证。

已授权的 SSH 会话在连接验证后即可用于后续只读状态检测，即使激活失败也保留到设备断开。本次没有通过修改 launchd 套接字、系统服务权限或强制注入来掩盖激活失败。
