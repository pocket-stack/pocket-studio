# 原生设备准备

当前执行链仅支持 iPod touch 4（iPod4,1 / n81）、iOS 6.1.6（10B500）、A4 / iBoot-574.4。没有扩大设备就绪矩阵，也不会把越狱未知自动转换成未越狱。

## 用户入口与授权

用户可以在接入条件页查看方案，也可以进入前置环境页。方案生成只读。正常模式下，已越狱、配对失败、电量未知或低于 50%、型号或固件不符都会得到明确错误。DFU 入口通过 CPID `0x8930`、BDID `0x08`、ECID 和 iBoot-574.4 校验硬件；系统、电量、配对不能读取，界面明确提示，保留电量确认并在 ramdisk 中校验实际系统。

原生方案先下载和构建，再请求用户进入 DFU。`entryMode: dfu` 的方案省去进入 DFU 的步骤和按键可用性确认。概览列出步骤、系统影响、风险和取消边界。风险与免责各等待 5 秒后点击一次确认，切出软件继续计时，不要求逐项勾选或滚到底部。授权版本为 `2026-09-07`，包含 root ramdisk 会话、分区挂载、Aquila 6 / Cydia / OpenSSH 安装；原来的“仅用于演示”条款已移除。浏览器模式另行显示模拟提示。

Rust 校验所有风险和前提、方案 ID、条款版本、阅读时长、阅读时间顺序、未来时间和 30 分钟有效期。方案在开始时被消费，不能复用；失败后的重试重新生成方案并收集确认。可观察到的正常模式会话更换会使旧目标校验失败。授权详情和步骤结果写入操作日志，导出保留参数。

## 执行与身份

1. 获取并校验所有固件与载荷，在电脑上构建临时 ramdisk。
2. 按入口模式重新验证设备。正常模式检查配对、电量与固件，再等待手动 DFU；DFU 模式重新检查原 ECID 的芯片、主板和 bootrom，直接继续。
3. 只对预检时的 ECID 发送 A4 limera1n 载荷，并确认 pwned DFU 标记；已经通过硬件校验且带 pwned 标记的 DFU 会复用现有状态。
4. 通过同一 ECID 启动构建的 ramdisk。临时镜像带有随机操作标记。
5. 在该 ECID 对应的 USB 端口匹配 usbmux 的序列号与设备 ID，连接临时 ramdisk 的 root SSH。会话只接受本次镜像的随机标记；不会回退为“第一台设备”。此处使用临时主机密钥策略和 ramdisk 公开默认密码，不连接正常 iOS 的特权 shell。
6. 只读挂载系统分区（不请求 fsck 或可写挂载），核对磁盘中的 6.1.6 / 10B500，再检查 `/mnt1/bin/bash`，避免重复安装。
7. 核验通过后将系统分区重新挂载为可写，再挂载数据分区，安装经过校验的 fstab、Aquila 6、Cydia、SSH deb、OpenSSH、OpenSSL、LukeZGD 和 nopatcyh 资源。严格检查每个命令的退出状态；tar 失败不会因清理命令成功而被忽略。
8. 检查 Cydia 和 sshd 文件，sync、重启，等待原 ECID 对应的设备返回（正常入口同时绑定原 UDID，DFU 入口按 ECID 解析返回的 UDID），再建立既有配对会话，再确认越狱与 SSH。重启连接丢失本身不算成功。

下载、构建和等待 DFU 可取消。limera1n 及其后续步骤不可取消；只读核验通过后，从重新挂载为可写并安装时标记系统修改边界。单个 native 进程只允许一个准备操作；关键步骤失败后不会自动重试或回滚。超时后可能存在部分写入，因此结果要求先检查设备状态。正常关闭窗口和退出会被运行中的准备操作阻止，并显示原因；不能防止强制结束进程、断电或系统崩溃。

## 资源与实现依据

- [Legacy-iOS-Kit-rs 42b423f](https://github.com/HalfSweet/Legacy-iOS-Kit-rs/tree/42b423fbfdcda66f1fb7cca2b605654dd905087e)：协议、镜像处理、HFS、limera1n 传输及 ramdisk 启动。
- [Legacy iOS Kit 1ff4be0 的 device_ramdisk](https://github.com/LukeZGD/Legacy-iOS-Kit/blob/1ff4be07ea2946ccaeff2db60c4426488b8f6e32/restore.sh)：n81 / 10B500 组件、boot args、SSH ramdisk 组合与越狱资源次序。
- [固定固件密钥元数据](https://github.com/LukeZGD/Legacy-iOS-Kit-Keys/blob/af6bf5934dc61ed557a967a3f42ab7fb8ed8c45e/iPod4%2C1/10B500/index.html)。元数据也经过 SHA-256 校验，不输出其内容。
- [ipwndfu 0e28932 的 A4 载荷参数](https://github.com/axi0mX/ipwndfu/blob/0e28932ec6a2a570b10fd77e50bda4216418cd98/limera1n.py)：下载同版本 shellcode 后验证占位符，再应用 `constants_574_4`。堆喷由 Rust 库完成。

`preparation-assets.json` 固定来源 URL、大小及 SHA-256。Apple 完整 IPSW 为 888,894,104 字节，来源是 Apple HTTPS；在原始下载的 SHA-1 与公开固件记录一致后计算并固定 SHA-256，各组件另有摘要。其余安装包沿用库的固定资源目录。二进制、固件、密钥和 SSH 资源均在运行时下载，不随仓库分发；来源项目的许可仍适用。

固定版本的远程 ZIP 读取器在真实 Apple IPSW 验证中返回 `invalid remote ZIP archive`，因此这里使用完整固件下载与摘要校验后再读取组件。没有静默回退为未验证下载。首次需要约 889 MB 固件空间，之后复用校验缓存；临时构建目录由具体操作拥有并自动删除。

SSH tar 含 `./` 根目录条目和 `etc/` 等目录，而 stock ramdisk 的 `/etc`、`/var`、`/tmp` 是符号链接。适配器跳过根目录元数据，在检查 stock 链接目标后将条目映射到 `/private` 下的真实目录，保留 stock 链接，拒绝绝对路径与父目录路径。此兼容问题有独立回归测试。

库的 firmware crate 固定 `serde_json = 1.0.140`；锁文件将 Tauri 的兼容传递依赖 `serde_with` 固定到 3.14.1，以解决 3.17.0 要求更高 serde_json 的冲突。

## 平台与验证范围

macOS、Linux、Windows 都使用已有系统 usbmux 与共享 Rust USB / SSH 实现。平台诊断沿用 HostEnvironment；缺少驱动、USB 权限或系统服务时失败，不自动安装驱动、重启服务或提升主机权限。

已完成 macOS 正常模式及 DFU 真机只读检测与方案预检，以及真实固件下载校验、iBSS / iBEC 补丁、32 MB SSH ramdisk 构建和 8 个安装资源包校验。自动测试覆盖授权时序与重放、并发互斥、取消边界、失败终止、事件字段、资源损坏及 HFS 目录链接兼容。

尚未对连接设备执行 limera1n、ramdisk 启动、越狱写入或重启后的验证。三平台的真实写入均不应被视为硬件验证通过。后续验收需要用户在桌面应用亲自确认备份与风险、启动操作并按引导进入 DFU。

只验证电脑上的资源步骤：

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example prepare_resources -- /path/to/cache
```

## DFU 入口的只读验收

DFU 主板映射依据 [libirecovery 的设备表](https://github.com/libimobiledevice/libirecovery/blob/master/src/libirecovery.c)：n81ap 为 CPID `0x8930` / BDID `0x08`。同一 A4 芯片的其他主板不能选择 iPod 固件。DFU 会话键包含内部 ECID，避免在相同 USB 端口更换设备后复用旧方案；完整标识不对外输出。重启后多台正常模式设备按原 ECID 精确匹配，重复身份会被拒绝。

连接已经在 DFU 的设备，运行以下检查只生成方案，不下载、不引导、不写入：

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example detect_devices -- --plan-preparation
```
