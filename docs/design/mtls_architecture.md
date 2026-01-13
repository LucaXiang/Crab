# mTLS 架构与证书管理 (mTLS Architecture)

## 1. 核心设计原则 (Core Principles)

*   **零信任 (Zero Trust)**: 不信任局域网，不信任 IP 地址，只信任**加密签名**。
*   **离线优先 (Offline First)**: Edge Server 必须在断网情况下能完成 99% 的业务（包括 POS 通信）。
*   **双重身份 (Dual Identity)**: Edge Server 对内是**服务端** (Server)，对外是**客户端** (Client)。
*   **极简运维 (Maintenance Free)**: 证书轮替、续期必须全自动，无需人工干预。

## 2. 证书层级 (PKI Hierarchy)

为了满足“云端管控”与“本地自治”的双重需求，我们采用**两级 CA 架构**，并严格隔离不同作用域。

### 2.1 实体定义

| 层级 (Level) | 实体名称 (Entity) | 类型 | 签发者 (Issuer) | 作用域 (Scope) | 谁持有私钥? | 谁信任它? |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **L1** | **Root CA** | 根证书 | Self-Signed | 全局信任锚点 | Auth Server (离线/HSM) | **仅在初始化时用于验证 CA 签名** (Bootstrapping Trust) |
| **L2** | **Uplink CA** | 中间 CA | Root CA | **云端专用通道** | Auth Server | **Cloud Core Server** |
| **L2** | **Tenant CA** | 中间 CA | Root CA | **租户局域网** | Auth Server | **Client, Edge Server (LAN)** |
| **L2** | **Ops CA** | 中间 CA | Root CA | **运维管理** | Auth Server | **Edge Server (LAN)** |
| **L3** | **Edge LAN Cert** | 局域网证书 | Tenant CA | Edge Server (Server端) | Edge Server | Client (POS) |
| **L3** | **Edge Uplink Cert** | 上行证书 | Uplink CA | Edge Server (Client端) | Edge Server | Cloud Core Server |
| **L3** | **Client Cert** | 客户端证书 | Tenant CA | POS / App | Client Device | Edge Server |
| **L3** | **Ops Cert** | 运维证书 | Ops CA | 工程师笔记本 | Ops Engineer | Edge Server |

### 2.2 为什么需要隔离 Tenant CA 和 Uplink CA？
*   **Tenant CA**: 仅用于签发该租户下的设备证书。即使 Tenant CA 私钥泄露，黑客也只能伪装成该餐厅的设备，无法伪装成云端核心组件，也无法连接 Cloud Core Server (因为 Cloud Core 只认 Uplink CA)。
*   **Uplink CA**: 专门用于 Edge Server 连接云端的鉴权。它的有效期可以设置得更短，且支持快速吊销。

## 3. 证书生命周期 (Lifecycle)

### 3.1 根证书与中间 CA (Bootstrap)
*   **生成时机**: 系统部署时生成 `Root CA`。租户注册时生成 `Tenant CA`。
*   **存储**: `Root CA` 私钥离线冷存储。`Tenant CA` / `Uplink CA` 私钥存放在 Auth Server 的 KMS (Key Management System) 中。

### 3.2 Edge Server 证书 (Dual Cert Strategy)
Edge Server 会同时持有两张证书：
1.  **LAN Cert**: 用于 `0.0.0.0:9625` (HTTPS) 和 `:9626` (TCP)。
    *   **CN**: `edge-server` (固定)
    *   **Validity**: 1 年 (长效，减少断网过期的风险)
2.  **Uplink Cert**: 用于连接 Cloud Core。
    *   **CN**: `UUID` (租户ID或设备ID)
    *   **Validity**: 7-30 天 (短效，强制联网刷新，便于封号)
    *   **Auto-Rotation**: Edge Server 只要联网，每天自动检查是否需要续期 Uplink Cert。

### 3.3 Edge Server 启动流程
Edge Server 的网络服务（HTTPS/TLS）并非随程序启动而立即开启，而是严格依赖于**租户登录状态**和**证书可用性**。

#### 状态 1: 未登录 / 初始化 (Unauthenticated)
*   **触发条件**: 首次安装或登出后。本地没有有效的 `tenant_ca.crt` 和 `edge_server.key`。
*   **行为**: 
    *   **不监听** 9625/9626 端口。
    *   程序处于“静默待机”状态，仅运行 UI 交互或后台配置逻辑。
    *   等待用户通过 UI 输入账号密码，连接 Cloud Auth Server。

#### 状态 2: 登录中 / 证书下发 (Provisioning)
*   **Root 锚点**: 程序二进制中**硬编码**（或安装包预置）了 `Root CA` 的公钥。这是信任的源头。
    > **为什么不直接信任 HTTPS 下载的内容？** 
    > 依赖公网 HTTPS (Web PKI) 意味着我们信任全球几百个公网 CA。如果攻击者劫持 DNS 并申请到合法的公网证书（如 Let's Encrypt），就能下发伪造的 Root CA。硬编码确保了信任锚点不依赖任何外部网络环境，是防御供应链攻击和中间人攻击的最后底牌。
*   **行为**: 
    *   通过外网连接 Auth Server。
    *   下载全套证书 (`tenant_ca`, `lan_cert`, `uplink_cert`)。
    *   **CA 验真 (Verify CA)**: 采用 **双重验证 (Double Verification)** 策略：
        1.  **内验证 (Hardcoded)**: 使用预置的 `Root CA` 验证下载的 `Tenant CA` 和 `Uplink CA` 的签名是否合法。这是主要防御手段。
        2.  **外验证 (HTTPS Side-Channel)**: 通过 HTTPS 接口 (`GET /api/pki/fingerprint`) 获取云端声明的最新 Root CA 指纹。
            *   **目的**: 交叉比对。如果“硬编码指纹”与“HTTPS 告知的指纹”不一致，通常意味着**客户端版本过旧**（Root CA 已轮替），系统应提示用户升级 App，而不是强行连接。
    *   持久化证书到磁盘。

#### 状态 3: 运行中 (Running)
*   **行为**:
    *   加载 `LAN Cert`。
    *   **启动监听**: 开启 9625 (HTTPS) 和 9626 (TLS) 端口。
    *   加载 `Uplink Cert`。
    *   **建立上行连接**: 使用 mTLS 主动连接 Cloud Core。
    *   开始接受局域网内的 mTLS 连接。

### 3.4 Client (客户端) 启动流程
客户端（如 POS 机、手机 App）的证书获取完全依赖云端，不通过 Edge Server：
1.  **云端登录**: 客户端通过 4G/5G/外网 WiFi 登录 Auth Server。
2.  **证书下发**: 验证通过后，Auth Server 下发 `client.crt`, `client.key` 以及 `tenant_ca.crt`。
    *   **注意**: Client **不需要** `Uplink Cert`，因为它不直接与 Core Server 同步业务数据（通过 Edge 转发）。
3.  **切换局域网**: 客户端拿到证书后，连接餐厅局域网，使用 mTLS 连接 Edge Server。

## 4. 运行时通信与隔离 (Runtime Isolation)

这是本设计的核心：**如何防止 Tenant A 的 Client 连上 Tenant B 的 Edge Server？**

### 4.1 信任链配置 (Trust Store)
尽管所有证书最终都由 Root CA 签名，但在 Edge Server 和 Client 的 TLS 配置中：

*   **Edge Server (LAN Interface)**: 
    *   信任列表: `[Tenant CA, Ops CA]`。
    *   **效果**: 既允许该租户的设备连接，也允许持有 Ops CA 证书的运维人员连接。
*   **Edge Server (Cloud Interface)**:
    *   信任列表: `[Root CA]` (用于验证 Cloud Server 的证书)。
    *   **Client Identity**: 使用 `Uplink Cert`。
*   **Cloud Core Server**:
    *   信任列表: `[Uplink CA]`。
    *   **效果**: 只接受持有有效 Uplink Cert 的 Edge Server 连接。**拒绝** Tenant CA 签发的证书（防止 POS 机直连云端核心接口）。
*   **Client (普通客户端)**: 
    *   信任列表: `[Tenant CA]`。
    *   **效果**: 只连接自己租户的 Edge Server，防止连接到隔壁餐厅的服务器。
*   **Ops Client (运维客户端)**:
    *   信任列表: `[Tenant CA (Target), Ops CA (Optional)]`。
    *   **效果**: 运维人员通常需要显式信任目标餐厅的 CA，或者在调试模式下暂时信任所有由 Root CA 签发的证书（视安全策略而定）。

### 4.2 握手过程 (Handshake)
当 Client 连接 Edge Server 时：
1.  **Client Hello**: 发起连接。
2.  **Server Hello**: Edge Server 发送自己的 `edge_server.crt`。
3.  **Client 验证**: Client 检查 Server 证书是否由自己信任的 `tenant_ca.crt` 签发。
    *   *Tenant A Client* 信任 *Tenant A CA*。
    *   如果遇到 *Tenant B Edge* (由 *Tenant B CA* 签发)，验证失败，连接断开。
4.  **Client Certificate Request**: Edge Server 要求客户端提供证书，并发送它接受的 CA 列表 (`Tenant CA`, `Ops CA`)。
5.  **Client Certificate**: Client 发送 `client.crt` 或 `ops_client.crt`。
6.  **Server 验证**: 
    *   如果证书由 `Tenant CA` 签发 -> **普通访问权限**。
    *   如果证书由 `Ops CA` 签发 -> **超级管理员/调试权限**。

### 4.3 Hostname 验证与 IP 直连策略 (Hostname Verification)
在局域网环境中，标准 DNS 通常不可用。虽然曾尝试使用 **mDNS** (Multicast DNS) 进行服务发现和域名解析，但实测发现其**延迟极高（可达 5s+）**，严重影响用户体验（如 POS 机开机后的首次连接）。因此，系统决定放弃域名解析，强制使用 **IP 直连**。

由于 IP 通常是动态分配的（DHCP），这导致无法将 IP 绑定到证书中：

*   **问题**: 标准 TLS 要求连接的 IP/域名必须与证书中的 CN/SAN 匹配。如果证书签发给 `edge-server` 但客户端通过 `192.168.1.100` 连接，会报 `HostnameMismatch` 错误。
*   **策略**: **禁用 Hostname 验证，严格保留 CA 签名验证**。
    *   客户端（基于 `rustls`）需实现自定义的 `ServerCertVerifier`。
    *   **验证逻辑**:
        1.  **签名校验 (必须)**: 确保证书链能追溯到受信任的 `Tenant CA` 或 `Ops CA`。这是防止中间人攻击(MITM)的根本保障。
        2.  **名称校验 (跳过)**: 显式忽略“连接地址”与“证书身份”的匹配检查。
*   **安全性论证**: 由于 `Tenant CA` 是私有的，只有合法的 Edge Server 才能持有由其签名的有效证书。只要签名验证通过，即可确认服务端身份，无需依赖不稳定的 Hostname。

### 4.4 离线与断网策略
*   **启动时**: 优先尝试联网校对 Root CA 指纹。
*   **断网时**: 
    *   跳过指纹校对，直接加载本地缓存的 `root_ca.crt` (仅作备用) 和 `tenant_ca.crt`。
    *   加载本地的私钥和证书。
    *   只要本地文件未损坏，mTLS 服务即可正常启动，完全不依赖云端。

## 5. 协议升级与端口规划 (Protocol Upgrade)

系统涉及两类通信协议的升级：HTTP 升级至 HTTPS (mTLS)，以及原始 TCP 升级至 TLS over TCP (mTLS)。为了平衡“首次配对”的便利性和“日常通信”的安全性，建议采用双端口或分阶段策略。

### 5.1 HTTP 服务 (Axum)
由于客户端证书的申请与分发统一通过云端 **Auth Server** 完成，Edge Server 无需处理设备的入网配对逻辑。

*   **端口**: `9625` (HTTPS only)
*   **策略**: 
    *   **完全禁用 HTTP**。
    *   Edge Server 启动时直接加载 TLS Config。
    *   任何不带有效 Client Certificate 的请求直接被 TLS 握手层拒绝，业务逻辑层（Axum Handler）无需关心鉴权。

### 5.2 TCP 消息总线
*   **端口**: `9626` (TLS only)
*   **策略**:
    *   与 HTTP 保持一致，拒绝非加密连接。
    *   握手成功后，Stream 的读写内容即为受信任的 `OrderIntent` / `OrderSync` 消息。

## 6. 灾难恢复与长期生存 (Resilience & DR)

面对“20年后 CA 过期”或“域名被封锁”等极端情况，系统需要具备极强的生存能力。

### 6.1 信任锚点轮替 (Trust Anchor Rotation)
硬编码的 Root CA 总有一天会过期（例如 20 年后），或者算法被攻破（如 RSA-2048 不再安全）。
*   **策略 A: 软件更新 (App Update)**: 最常规的路径。发布新版 App，内置新的 Root CA 公钥。
*   **策略 B: 多锚点共存 (Multi-Anchor)**: 在代码中预置**多个**不同生命周期的 Root CA（例如 `Root_2024`, `Root_2030`）。只要其中任何一个能通过验证，连接即被允许。
*   **策略 C: 交叉签名过渡 (Cross-Signing)**: 在旧 CA 过期前，用旧 CA 的私钥去给新 CA 签名。这样旧版 App 依然能通过“旧 Root -> 新 Root -> 目标证书”的信任链工作，直到所有用户完成升级。

### 6.2 接入点逃生 (Endpoint Fallback)
如果 `api.crab.com` 域名过期、被劫持或被政策性封锁，设备不能变砖。
*   **配置策略**: 客户端不应只硬编码一个域名，而应持有一个 **Bootstrap List**：
    ```rust
    const BOOTSTRAP_ENDPOINTS: &[&str] = &[
        "https://api.crab.com",          // 1. 主域名
        "https://backup-crab.io",        // 2. 备用域名 (不同注册商/TLD)
        "https://123.45.67.89",          // 3. 硬编码 IP (最后手段)
        "https://[2001:db8::1]"          // 4. IPv6 地址
    ];
    ```
*   **连接逻辑**: 启动时按顺序尝试连接。一旦某个端点握手成功（且通过了 Root CA 验真），就将其标记为“当前活跃端点”并持久化。
*   **公共信标 (Public Beacon)**: 利用 GitHub、S3 或 DNS TXT 记录作为最后的“死信箱” (Dead Drop)。
    *   **思路**: 当所有预置域名都连不上时，去读取 `https://raw.githubusercontent.com/crab-org/beacon/main/endpoints.signed` 获取最新的 IP/域名列表。
    *   **安全**: 必须校验**数字签名**。即使 GitHub 账号被盗，黑客也无法伪造合法的签名，只能制造拒绝服务，而无法劫持流量。
*   **防封锁**: 结合**IP 直连**与**SNI 伪装**技术，确保即使 DNS 解析失效，只要 IP 路由可达，业务即可恢复。

## 7. 物理安全与防盗 (Physical Security)

由于 Edge Server 运行在不可控的物理环境（餐厅柜台下），必须假设**物理接触即失陷**。

### 7.1 存储加密
*   **私钥加密**: `edge_server.key` 在磁盘上**必须**是加密存储的（AES-256-GCM）。
*   **密钥派生**: 解密密钥 (KEK) 不存盘。
    *   **方案 A (人工值守)**: 每次启动时要求店长输入密码/PIN 码解密。优点是成本低，缺点是运维麻烦。
    *   **方案 B (硬件级安全 - 推荐)**: 利用 **TPM 2.0 / Secure Enclave**。
        *   **进阶用法**: 不仅仅是加密文件，而是将私钥**生成并锁定在 TPM 芯片内部 (Non-exportable)**。
        *   **效果**: 私钥**永远不会**出现在内存 (RAM) 中。当需要建立 mTLS 连接时，CPU 把数据发给 TPM，TPM 签好名后返回结果。即使黑客完全控制了操作系统，他也无法导出私钥，只能把整台机器搬走（配合云端挂失，搬走也没用）。

### 7.2 云端挂失与吊销
*   **场景**: Edge Server 被盗。
*   **对策**: 商户在云端后台点击“挂失”。
*   **机制**: Auth Server 将该 Edge Server 的 `Uplink Cert` 加入 CRL (吊销列表) 或直接从数据库移除白名单。
*   **效果**: 
    *   被盗设备无法连接云端，无法同步数据。
    *   由于缺乏云端最新的时间戳签名，该设备产生的发票在税务系统也是无效的。
    *   **法律免责 (Liability Protection)**: 见下文。

### 7.3 法律免责 (Liability Protection) - "我会被处罚吗？"
*   **用户顾虑**: 设备被黑客偷走后，如果黑客提取出私钥并伪造了大量虚假发票，商户是否需要承担税务责任？
*   **系统保障**: 只要商户及时挂失，通常**不需要**承担责任。系统提供了数学层面的铁证：
    1.  **时间戳证据**: 挂失时间点之后签名的所有发票，云端一律拒收，不计入税务系统。
    2.  **链分叉证据 (Chain Fork)**: 如果黑客试图伪造挂失时间点*之前*的发票，由于他无法接续云端已同步的最新链头 (Chain Head)，他伪造的链条会与云端链条形成**分叉**。这种分叉是**不可抵赖的数学证据**，证明了“数据来源于非法设备”，从而帮助商户自证清白。

## 8. 跨平台实现策略 (Cross-Platform Implementation)

面对 PC (TPM), Android (KeyStore), iOS (Secure Enclave) 等碎片化的硬件接口，为了避免代码维护地狱，我们需要在 Rust 层抽象出一套统一的接口。

### 8.1 核心抽象 (The Trait)
我们在 `shared` 包中定义一个 `SecureSigner` trait。**注意**: 这个 trait **没有** `get_private_key()` 方法，因为我们永远拿不到私钥。

```rust
#[async_trait]
pub trait SecureSigner: Send + Sync {
    /// 获取公钥 (用于生成 CSR 或握手)
    fn public_key(&self) -> Vec<u8>;

    /// 核心能力：让硬件帮我签名
    /// 此时私钥在 TPM/TEE 内部，数据进去，签名出来
    async fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError>;
    
    /// 硬件类型 (用于日志/调试)
    fn provider_type(&self) -> ProviderType; // e.g., Tpm, Android, Software
}
```

### 8.2 适配层 (Adapters)
业务逻辑只依赖 `Box<dyn SecureSigner>`，具体的脏活累活由适配层处理：

| 平台 | 实现结构体 | 底层依赖 | 复杂度 |
| :--- | :--- | :--- | :--- |
| **Dev / Fallback** | `SoftwareSigner` | `ring` / `openssl` | 低 (直接读文件) |
| **Linux / Win** | `TpmSigner` | `tss-esapi` crate | 中 (C FFI) |
| **Android** | `AndroidSigner` | JNI -> `AndroidKeyStore` | 高 (需写 Java/Kotlin 桥接) |
| **iOS / macOS** | `SecureEnclaveSigner` | `security-framework` | 中 (系统 API) |

### 8.3 编译时隔离
利用 Rust 的 `#[cfg]` 特性，让编译器只编译当前平台需要的代码，保持二进制轻量。

```rust
#[cfg(target_os = "android")]
pub fn create_signer() -> Box<dyn SecureSigner> {
    Box::new(AndroidSigner::new())
}

#[cfg(not(target_os = "android"))]
pub fn create_signer() -> Box<dyn SecureSigner> {
    // 自动检测是否有 TPM，没有则降级为软件模拟
    if tpm_available() {
        Box::new(TpmSigner::new())
    } else {
        Box::new(SoftwareSigner::new())
    }
}
```

## 附录：硬件与术语 (Glossary)

### A.1 TPM 2.0 (Trusted Platform Module)
*   **定义**: 一种焊在 PC/服务器主板上的安全芯片，专门用于生成和储存加密密钥。
*   **适用设备**: x86 服务器, 工控机, Windows PC。
*   **类比**: 相当于在你的电脑里住了一个**公证人**。
    *   你想签名文件时，把文件塞进公证人的窗口（发给 TPM）。
    *   公证人在他的密室里签好名，把文件递出来（返回签名结果）。
    *   **关键点**: 公证人的私章（私钥）**永远不会离开密室**，连你也拿不到。
*   **价值**: 即使黑客攻破了操作系统（拿到了电脑的管理员权限），他也无法通过复制文件的方式盗走私钥。

### A.2 Android KeyStore / StrongBox
*   **定义**: 安卓设备的安全存储机制，对应 PC 的 TPM。
*   **层级**:
    *   **TEE (Trusted Execution Environment)**: 大多数安卓手机都有。私钥存储在主 CPU 的一个隔离区域（TrustZone），操作系统无法直接读取。
    *   **StrongBox (Google Pixel / 三星 Knox)**: 类似 TPM 的独立安全芯片。比 TEE 更安全，因为它有独立的 CPU、内存和真随机数生成器，物理上与主 CPU 隔离。
*   **用法**: App 调用 `AndroidKeystore` API 生成密钥对。私钥生成后被硬件保护，App 只能通过 `sign()` 接口让硬件代为签名，无法导出私钥本身。
