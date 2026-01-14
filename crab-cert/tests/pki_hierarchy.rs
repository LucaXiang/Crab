use crab_cert::{
    CaProfile, CertProfile, CertificateAuthority, SkipHostnameVerifier, to_rustls_certs,
    verify_ca_signature, verify_chain_against_root,
};
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types::{ServerName, UnixTime};
use std::sync::Once;

static INIT: Once = Once::new();

fn init_crypto() {
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// 测试 PKI (公钥基础设施) 的层级结构流程
///
/// 本测试模拟了一个完整的两级 CA 结构：
/// Root CA (根证书) -> Intermediate CA (中间证书) -> Leaf Certificate (终端证书)
///
/// 验证了以下关键场景：
/// 1. 证书链的签发：Root CA 签发 Tenant/Uplink CA，后者再签发 Server/Client 证书。
/// 2. 证书链的验证：验证终端证书是否能通过硬编码的 Root CA 进行信任链追溯。
/// 3. 签名验证：直接验证 CA 证书本身的数字签名。
#[test]
fn test_pki_hierarchy_flow() {
    init_crypto();
    // 1. 创建 Root CA (根证书)
    // 注意：在真实生产系统中，Root CA 是硬编码在二进制文件中的 (trust anchor)。
    // 这里我们从文件系统加载生成的 Root CA 证书和私钥，以便在测试中进行签发操作。
    // 该文件必须与 `lib.rs` 中硬编码的 `ROOT_CA_PEM` 内容一致，否则 `verify_chain_against_root` 会失败。
    let root_cert_pem =
        std::fs::read_to_string("certs/root_ca.pem").expect("Failed to read root cert");
    let root_key_pem =
        std::fs::read_to_string("certs/root_key.pem").expect("Failed to read root key");

    let root_ca =
        CertificateAuthority::load(&root_cert_pem, &root_key_pem).expect("Failed to load Root CA");

    // 2. 创建 Tenant CA (中间证书 - 租户级)
    // 模拟为特定租户颁发的中间 CA，用于隔离不同租户的证书签发权限。
    let tenant_profile = CaProfile::intermediate("Tenant CA 001", "Crab Tenant 001");
    let tenant_ca = CertificateAuthority::new_intermediate(tenant_profile, &root_ca)
        .expect("Failed to create Tenant CA");

    // 3. 创建 Edge Server Cert (终端证书 - 服务端)
    // 由 Tenant CA 签发给边缘服务器，包含 IP 地址等 SAN 信息。
    let server_profile = CertProfile::new_server("edge-server", vec!["192.168.1.100".to_string()]);
    let (server_cert_pem, _server_key_pem) = tenant_ca
        .issue_cert(&server_profile)
        .expect("Failed to issue server cert");

    // 4. 验证 Edge Server 证书链
    // 证书链结构: Server Cert -> Tenant CA -> Root CA
    // 在 TLS 握手中，Server 会发送 [Server Cert, Tenant CA] 给 Client。
    // Client 本地持有受信任的 Root CA。

    // 构建证书链 PEM (Server Cert + Tenant CA)
    let chain_pem = format!("{}{}", server_cert_pem, tenant_ca.cert_pem());

    // 使用硬编码的 Root CA 验证整个证书链。
    // `verify_chain_against_root` 内部使用了 `include_str!("../certs/root_ca.pem")` 加载的根证书。
    // 因为我们上面加载的 `root_ca` 就是来源于同一个文件，所以验证应该通过。
    verify_chain_against_root(&chain_pem).expect("Failed to verify server chain against root");

    println!("Server chain verification passed!");

    // 5. 创建 Uplink CA (中间证书 - 上行链路级)
    // 模拟用于云端通信的中间 CA。
    let uplink_profile = CaProfile::intermediate("Uplink CA", "Crab Cloud Uplink");
    let uplink_ca = CertificateAuthority::new_intermediate(uplink_profile, &root_ca)
        .expect("Failed to create Uplink CA");

    // 6. 创建 Edge Client Cert (终端证书 - 客户端)
    // 由 Uplink CA 签发，用于边缘设备连接云端时的 mTLS 身份认证。
    let client_profile = CertProfile::new_uplink("edge-client-uuid-123");
    let (client_cert_pem, _client_key_pem) = uplink_ca
        .issue_cert(&client_profile)
        .expect("Failed to issue client cert");

    // 7. 验证 Client 证书链
    // 证书链结构: Client Cert -> Uplink CA -> Root CA
    let client_chain_pem = format!("{}{}", client_cert_pem, uplink_ca.cert_pem());

    // 这里使用 `verify_client_cert` 进行验证，需要显式传入 Root CA PEM。
    // 客户端证书验证通常比服务端更严格（取决于配置），但基础的签名链验证逻辑是一致的。
    crab_cert::verify_client_cert(&client_chain_pem, &root_cert_pem)
        .expect("Failed to verify client chain");

    println!("Client chain verification passed!");

    // 8. 验证 Tenant CA 本身的签名
    // 直接检查 Tenant CA 是否确实由 Root CA 签发，这是一个底层的签名验证操作。
    verify_ca_signature(tenant_ca.cert_pem()).expect("Failed to verify Tenant CA signature");
    println!("Tenant CA signature verified!");
}

/// 测试 SkipHostnameVerifier (跳过主机名验证)
///
/// 在边缘计算离线场景或 IP 直连模式下，客户端可能无法通过域名访问服务端，
/// 或者服务端的 IP 地址经常变动。
/// `SkipHostnameVerifier` 允许客户端在 TLS 握手时忽略 Hostname/IP 的匹配检查，
/// 仅验证证书链的有效性和受信任的根证书签名。
#[test]
fn test_skip_hostname_verifier() {
    init_crypto();
    // 1. 加载 Root CA
    let root_cert_pem =
        std::fs::read_to_string("certs/root_ca.pem").expect("Failed to read root cert");
    let root_key_pem =
        std::fs::read_to_string("certs/root_key.pem").expect("Failed to read root key");
    let root_ca =
        CertificateAuthority::load(&root_cert_pem, &root_key_pem).expect("Failed to load Root CA");

    // 2. 创建 Tenant CA (中间证书)
    let tenant_profile = CaProfile::intermediate("Tenant CA 002", "Crab Tenant 002");
    let tenant_ca = CertificateAuthority::new_intermediate(tenant_profile, &root_ca)
        .expect("Failed to create Tenant CA");

    // 3. 签发 Server Cert (指定了特定 IP)
    // 注意：证书中绑定的 IP 是 "10.0.0.5"
    let server_profile = CertProfile::new_server("edge-server", vec!["10.0.0.5".to_string()]);
    let (server_cert_pem, _server_key_pem) = tenant_ca
        .issue_cert(&server_profile)
        .expect("Failed to issue server cert");

    // 4. 设置 SkipHostnameVerifier
    // 构建一个包含 Root CA 的信任存储 (RootCertStore)
    let mut root_store = rustls::RootCertStore::empty();
    for cert in to_rustls_certs(&root_cert_pem).unwrap() {
        root_store.add(cert).unwrap();
    }
    // 实例化自定义验证器
    let verifier = SkipHostnameVerifier::new(root_store);

    // 5. 使用错误的 IP 进行验证测试
    // 模拟客户端尝试连接 "192.168.1.99"，但这与证书中的 "10.0.0.5" 不匹配。
    // 在标准 TLS 验证中这会失败，但在 SkipHostnameVerifier 中应该通过。
    let certs = to_rustls_certs(&server_cert_pem).unwrap();
    let intermediates = to_rustls_certs(tenant_ca.cert_pem()).unwrap();

    let wrong_ip = ServerName::try_from("192.168.1.99").unwrap();

    let result =
        verifier.verify_server_cert(&certs[0], &intermediates, &wrong_ip, &[], UnixTime::now());

    assert!(
        result.is_ok(),
        "SkipHostnameVerifier 应该通过验证，即使 IP 不匹配 (预期行为)"
    );

    println!("SkipHostnameVerifier passed for mismatched IP!");
}
