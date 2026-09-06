# 旧版 iOS 连接兼容性

## 问题与依据

设备：iPod touch 4（iPod4,1 / N81AP / iOS 6.1.6），使用 Legacy iOS Kit 默认路径越狱。

最初的 Rust 会话出现 HandshakeFailure，未配对的 GetValue 请求只能读取部分字段。对比 [Legacy iOS Kit 的设备读取流程](https://github.com/LukeZGD/Legacy-iOS-Kit/blob/fdddc6143a3c789c1fce6b2874df4770e8f3c329/restore.sh) 与它使用的 libimobiledevice，确认以下差异：

- [lockdown.c](https://github.com/libimobiledevice/libimobiledevice/blob/master/src/lockdown.c) 在 iOS 7 之前先调用 ValidatePair，恢复已有主机身份的受信任状态。当前 Rust 依赖未执行这一步。
- [idevice.c](https://github.com/libimobiledevice/libimobiledevice/blob/master/src/idevice.c) 对 iOS 10 之前的设备使用 TLS 1.0，并使用配对根证书与根私钥认证。Legacy-iOS-Kit-rs 固定启用的 rustls 连接不支持这个旧协议。
- 默认越狱路径下，Cydia 可以注册在 SpringBoard 中，却不出现在 installation_proxy 的应用列表中；AFC2 也不一定安装。因此，单独依赖 AFC2 或安装服务不足以判断越狱。

## Studio 的处理

兼容逻辑现已移入 Legacy-iOS-Kit-rs 并推送至 `42b423f`。Studio 固定依赖该提交、启用 `legacy-tls`，通过库的 `NormalDevice::inspect()` 获取配对信息和越狱证据；应用内重复的 TLS 与检测实现已移除。仍不运行 restore.sh 或外部设备工具。

会话只读取已有配对记录。ValidatePair 请求仅包含公开证书、HostID 与 SystemBUID，不包含私钥或 EscrowBag。私钥只在本机 TLS 认证时使用。USB 会话按设备版本选定协议，不会在握手失败后盲目降级；服务器证书必须与已配对设备证书完全匹配。

使用 vendored OpenSSL 是为了支持已知旧设备所需的 TLS 1.0 / RSA / CBC 套件。对这些已识别的旧版设备允许初始兼容握手，并禁用后续重新协商。网络下载等连接继续使用各自的 TLS 配置。编译时需要 C 编译器、Make 和 Perl，运行时不需要安装额外命令或动态 OpenSSL 库。

越狱判定采用可观察证据：AFC2 根目录访问成功，或 SpringBoard 中存在 Cydia 注册信息且 SSH 服务响应正常。仅有 Cydia 图标、仅有 SSH 响应、或任何读取失败，都不会被误判为确认的越狱状态。

## 验证

```sh
pnpm check
pnpm test
pnpm test:native
```

硬件检查使用显式运行的本地诊断工具，不属于自动测试。`src-tauri/examples/` 已被 Git 忽略，不随仓库分发。真机已返回型号、系统、序列号（脱敏）、电量与数据分区容量，配对、越狱和 SSH 检测通过。未执行重新配对、越狱、设备重启、安装或 SSH 命令。

回归测试覆盖版本边界、公开配对字段、双向 TLS 身份验证、错误设备证书拒绝、过大消息拒绝，以及 Cydia 在文件夹中的注册信息识别。组件正文不显示工程阶段或后端实现标记，故障只提供用户可采取的操作提示。
