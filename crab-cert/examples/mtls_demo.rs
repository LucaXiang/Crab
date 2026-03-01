//! Crab mTLS 架构演示
//!
//! 本示例演示了 `docs/design/mtls_architecture.md` 中描述的证书生命周期。
//!
//! 场景：
//! 1. **Root CA**：Crab 平台的全局信任锚点。
//! 2. **Tenant CA**：为特定餐厅（“美味蟹堡”）签发的中间 CA。
//! 3. **Edge Server Cert**：餐厅内 Edge Server 节点的服务器证书。
//! 4. **Client Cert**：POS 设备（iPad）的客户端证书。
//!
//! 我们将验证：
//! - POS 可以信任 Edge Server（服务器认证）。
//! - Edge Server 可以信任 POS（客户端认证）。
//! - 硬件 ID 已绑定到证书。

use crab_cert::{
    CaProfile, CertMetadata, CertProfile, CertificateAuthority, KeyType, generate_hardware_id,
    verify_client_cert, verify_server_cert,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    println!("🦀 Crab mTLS 证书管理演示\n");

    // ============================================================================================
    // 步骤 1：引导 Root CA（全局信任锚点）
    // 在生产环境中，此私钥应离线/冷存储。
    // ============================================================================================
    println!("--- 步骤 1：引导 Root CA ---");
    let root_profile = CaProfile {
        common_name: "Crab Global Root CA".to_string(),
        organization: "Crab Inc.".to_string(),
        validity_days: 365 * 20, // 20 years
        key_type: KeyType::P256, // 高效的 ECC 密钥
        ..Default::default()
    };

    let root_ca = CertificateAuthority::new_root(root_profile)?;
    println!("✅ Root CA 已生成");
    println!(
        "   指纹：{}",
        CertMetadata::from_pem(root_ca.cert_pem())?.fingerprint_sha256
    );

    // ============================================================================================
    // 步骤 2：配置 Tenant CA（中间 CA）
    // 当新餐厅签约时发生这种情况。
    // ============================================================================================
    println!("\n--- 步骤 2：配置 Tenant CA (美味蟹堡) ---");
    let tenant_id: i64 = 1001;

    let tenant_profile = CaProfile {
        common_name: "Tasty Crab CA".to_string(), // 实际上可能包含 Tenant ID
        organization: "Tasty Crab Restaurant".to_string(),
        validity_days: 365 * 5, // 5 years
        path_len: Some(0),      // 不能签署其他 CA，只能签署叶证书
        ..Default::default()
    };

    // Root CA 签发 Tenant CA 证书
    let tenant_ca = CertificateAuthority::new_intermediate(tenant_profile, &root_ca)?;
    println!("✅ Tenant CA 已由 Root CA 签发");
    println!(
        "   主题：{}",
        CertMetadata::from_pem(tenant_ca.cert_pem())?
            .common_name
            .unwrap_or_default()
    );

    // ============================================================================================
    // 步骤 3：签发 Edge Server 证书 (L3 Edge LAN 证书)
    // Edge Server 用于向 POS 设备证明其身份。
    // ============================================================================================
    println!("\n--- 步骤 3：签发 Edge Server LAN 证书 ---");
    let server_hardware_id = generate_hardware_id(); // 在现实生活中，这在服务器上运行
    println!("   检测到服务器硬件 ID：{}", server_hardware_id);

    let mut server_profile = CertProfile::new_server(
        "edge-server",
        vec![
            "edge-server".to_string(),
            "127.0.0.1".to_string(),
            "192.168.1.100".to_string(),
        ],
        Some(tenant_id),
        server_hardware_id,
    );
    // server_profile.common_name = "edge-server".to_string(); // 已由 new_server 设置
    // server_profile.organization = "Tasty Crab Restaurant".to_string(); // 默认为 "Crab Tenant"
    server_profile.organization = "Tasty Crab Restaurant".to_string();
    server_profile.validity_days = 365; // 1 year

    // Tenant CA 签发 Server 证书
    let (server_cert_pem, _server_key_pem) = tenant_ca.issue_cert(&server_profile)?;
    println!("✅ Edge Server 证书已签发");

    // 验证元数据
    let server_meta = CertMetadata::from_pem(&server_cert_pem)?;
    println!("   绑定的 Tenant ID：{:?}", server_meta.tenant_id);
    println!("   绑定的硬件 ID (Device ID)：{:?}", server_meta.device_id);

    // ============================================================================================
    // 步骤 4：签发 Client 证书 (L3 Client 证书)
    // POS 设备（iPad）用于向 Edge Server 验证其身份。
    // ============================================================================================
    println!("\n--- 步骤 4：签发 POS 客户端证书 ---");
    let mut client_profile = CertProfile::new_client(
        "pos-ipad-01",
        Some(tenant_id),
        Some("device-pos-01".to_string()),
        Some("iPad Front Desk".to_string()),
    );
    client_profile.organization = "Tasty Crab Restaurant".to_string();
    client_profile.validity_days = 90; // 移动设备的有效期较短
    // client_profile.is_server = false;
    // client_profile.is_client = true;    // 用法：客户端认证

    let (client_cert_pem, _client_key_pem) = tenant_ca.issue_cert(&client_profile)?;
    println!("✅ 客户端证书已签发");

    // 验证客户端元数据
    let client_meta = CertMetadata::from_pem(&client_cert_pem)?;
    println!("   客户端元数据：");
    println!("   - 设备 ID (UID): {:?}", client_meta.device_id);
    println!("   - 终端名称: {:?}", client_meta.client_name);

    // ============================================================================================
    // 步骤 5：验证模拟
    // 演示 verify_server_cert 和 verify_client_cert 在实践中如何工作。
    // ============================================================================================
    println!("\n--- 步骤 5：正在模拟 mTLS 验证 ---");

    // 场景 A：POS 连接到 Edge Server
    // POS 持有 Tenant CA（信任根）并验证服务器证书。
    println!("A. POS 验证 Edge Server 身份：");
    // 注意：我们使用 Tenant CA 作为 LAN 的信任根。
    // 在真实的浏览器/操作系统中，我们可能需要完整的证书链（Root + Tenant），
    // 但对于我们的自定义 mTLS，信任 Tenant CA 足以实现 LAN 隔离。
    match verify_server_cert(&server_cert_pem, tenant_ca.cert_pem()) {
        Ok(_) => println!("   ✅ 验证成功：POS 信任 Edge Server。"),
        Err(e) => println!("   ❌ 验证失败：{}", e),
    }

    // 场景 B：Edge Server 验证 POS
    // Edge Server 检查连接的客户端是否具有由 Tenant CA 签名的有效证书。
    println!("B. Edge Server 验证 POS 身份：");
    match verify_client_cert(&client_cert_pem, tenant_ca.cert_pem()) {
        Ok(_) => println!("   ✅ 验证成功：Edge Server 信任 POS。"),
        Err(e) => println!("   ❌ 验证失败：{}", e),
    }

    // 场景 C：跨租户隔离测试（安全）
    // 如果来自“汉堡王”（另一个租户）的黑客试图连接会怎样？
    println!("C. 安全测试：跨租户隔离：");

    let hacker_tenant_profile = CaProfile {
        common_name: "Burger King CA".to_string(),
        ..Default::default()
    };
    let hacker_ca = CertificateAuthority::new_intermediate(hacker_tenant_profile, &root_ca)?;

    let hacker_client_profile = CertProfile::new_client("hacker-pos", None, None, None);
    // hacker_client_profile.common_name = "hacker-pos".to_string();
    // hacker_client_profile.is_client = true;
    let (hacker_cert_pem, _) = hacker_ca.issue_cert(&hacker_client_profile)?;

    // Edge Server（信任 Tasty Crab CA）尝试验证黑客的证书（由 Burger King CA 签名）
    match verify_client_cert(&hacker_cert_pem, tenant_ca.cert_pem()) {
        Ok(_) => println!("   ❌ 严重失败：黑客被接受了！"),
        Err(_) => println!("   ✅ 安全成功：黑客被拒绝（不受信任的发行者）。"),
    }

    // ============================================================================================
    // 步骤 6：导出以供使用
    // ============================================================================================
    println!("\n--- 步骤 6：准备就绪 ---");
    println!("证书和密钥通常保存到文件：");
    println!("- tenant_ca.crt");
    println!("- edge_server.crt / edge_server.key");
    println!("- client.crt / client.key");
    println!("\n使用 `CertificateAuthority::save()` 将它们写入磁盘。");

    Ok(())
}
