use crate::error::{CertError, Result};
use sha2::{Digest, Sha256};
use x509_parser::oid_registry;

/// AEAT (@firma) 认可的西班牙 CA 根证书 SHA256 指纹
///
/// Verifactu 要求证书能通过 @firma 平台验证。
/// 这些是 @firma 认可的主要西班牙 CA 根证书指纹。
///
/// 来源: macOS/Mozilla Root Store + 官方 CA 网站
const TRUSTED_ROOT_FINGERPRINTS: &[&str] = &[
    // AC RAIZ FNMT-RCM (西班牙皇家铸币厂, 最常见)
    "ebc5570c29018c4d67b1aa127baf12f703b4611ebc17b7dab5573894179b93fa",
    // ACCVRAIZ1 (Agencia de Tecnología y Certificación Electrónica)
    "9a6ec012e1a7da9dbe34194d478ad7c0db1822fb071df12981496ed104384113",
    // Izenpe.com (Basque Country)
    "2530cc8e98321502bad96f9b1fba1b099e2d299e0f4548bb914f363bc0d4531f",
    // Autoridad de Certificacion Firmaprofesional
    "04048028bf1f2864d48f9ad4d83294366a828856553f3b14303f90147f5d40ef",
];

/// P12 证书解析结果 (西班牙电子签名证书)
#[derive(Debug, Clone)]
pub struct P12CertInfo {
    /// SHA256 证书指纹
    pub fingerprint: String,
    /// 过期时间 (Unix millis)
    pub expires_at: i64,
    /// 生效时间 (Unix millis)
    pub not_before: i64,

    // === Subject DN 字段 ===
    /// Common Name (公司名 + 税号, 或法人代表全名)
    pub common_name: String,
    /// serialNumber (OID 2.5.4.5) — 税号 NIF/CIF, 如 "IDCES-B12345678"
    pub serial_number: Option<String>,
    /// organizationIdentifier (OID 2.5.4.97) — 欧盟增值税标识, 如 "VATES-B12345678"
    pub organization_id: Option<String>,
    /// organizationName (OID 2.5.4.10) — 公司名称
    pub organization: Option<String>,
    /// givenName (OID 2.5.4.42) — 法人代表名
    pub given_name: Option<String>,
    /// surname (OID 2.5.4.4) — 法人代表姓
    pub surname: Option<String>,
    /// countryName (OID 2.5.4.6) — 国家代码
    pub country: Option<String>,

    // === Issuer 信息 ===
    /// 签发机构名称
    pub issuer: String,
}

impl P12CertInfo {
    /// 从 serialNumber 或 organizationIdentifier 中提取纯税号
    ///
    /// "IDCES-B12345678" → "B12345678"
    /// "VATES-B12345678" → "B12345678"
    pub fn tax_id(&self) -> Option<&str> {
        // 优先从 serialNumber 提取
        if let Some(ref sn) = self.serial_number {
            if let Some(nif) = sn.strip_prefix("IDCES-") {
                return Some(nif);
            }
            // 有些证书直接放 NIF
            if sn.len() == 9 {
                return Some(sn);
            }
        }
        // 其次从 organizationIdentifier 提取
        if let Some(ref oid) = self.organization_id
            && let Some(nif) = oid.strip_prefix("VATES-")
        {
            return Some(nif);
        }
        None
    }
}

/// 解析 PKCS#12 文件，提取西班牙电子签名证书元数据
///
/// 使用 OpenSSL 解析 P12 (支持 BER 编码的真实证书)，
/// 然后用 x509-parser 提取 Subject DN 字段并验证信任链。
///
/// 验证:
/// - P12 密码正确、文件格式有效、包含私钥
/// - 证书链签名完整性 (每一级由上级签发)
/// - 根证书指纹匹配 AEAT 认可的西班牙 CA
///
/// 提取:
/// - SHA256 指纹、有效期
/// - 公司税号 (NIF/CIF)、公司名称
/// - 法人代表信息 (姓名)
/// - 签发机构
pub fn parse_p12(data: &[u8], password: &str) -> Result<P12CertInfo> {
    // 用 OpenSSL 解析 P12 (能处理 BER 编码)
    let pkcs12 = openssl::pkcs12::Pkcs12::from_der(data)
        .map_err(|e| CertError::ValidationFailed(format!("Invalid P12 file: {e}")))?;

    let parsed = pkcs12.parse2(password).map_err(|e| {
        CertError::ValidationFailed(format!("Wrong P12 password or corrupted: {e}"))
    })?;

    // 必须包含私钥 (签名用)
    if parsed.pkey.is_none() {
        return Err(CertError::ValidationFailed(
            "P12 contains no private key for signing".into(),
        ));
    }

    // 叶子证书
    let leaf_cert = parsed
        .cert
        .ok_or_else(|| CertError::ValidationFailed("P12 contains no certificate".into()))?;

    // 收集完整证书链的 DER: [leaf, ...intermediates, root]
    let leaf_der = leaf_cert
        .to_der()
        .map_err(|e| CertError::ValidationFailed(format!("Cert DER encode error: {e}")))?;

    let mut cert_ders = vec![leaf_der];
    if let Some(ref ca_certs) = parsed.ca {
        for ca in ca_certs {
            let der = ca.to_der().map_err(|e| {
                CertError::ValidationFailed(format!("CA cert DER encode error: {e}"))
            })?;
            cert_ders.push(der);
        }
    }

    // === 证书链验证 ===
    let cert_der_refs: Vec<&[u8]> = cert_ders.iter().map(|d| d.as_slice()).collect();
    verify_chain(&cert_der_refs)?;

    // SHA256 指纹 (叶子证书)
    let fingerprint = hex::encode(Sha256::digest(&cert_ders[0]));

    // 用 x509-parser 解析叶子证书以提取 Subject DN
    let (_, x509) = x509_parser::parse_x509_certificate(&cert_ders[0])
        .map_err(|e| CertError::ValidationFailed(format!("Failed to parse X.509: {e}")))?;

    // === 提取 Issuer ===
    let issuer = x509
        .issuer()
        .iter_organization()
        .next()
        .and_then(|o| o.as_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    // === 提取 Subject DN 字段 ===
    let common_name = x509
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    let oid_org_identifier_str = "2.5.4.97";

    let mut serial_number = None;
    let mut organization_id = None;
    let mut organization = None;
    let mut given_name = None;
    let mut surname = None;
    let mut country = None;

    for rdn in x509.subject().iter_rdn() {
        for attr in rdn.iter() {
            let oid = attr.attr_type();
            let value = attr.as_str().ok();

            if oid == &oid_registry::OID_X509_SERIALNUMBER {
                serial_number = value.map(String::from);
            } else if oid.to_id_string() == oid_org_identifier_str {
                organization_id = value.map(String::from);
            } else if oid == &oid_registry::OID_X509_ORGANIZATION_NAME {
                organization = value.map(String::from);
            } else if oid == &oid_registry::OID_X509_GIVEN_NAME {
                given_name = value.map(String::from);
            } else if oid == &oid_registry::OID_X509_SURNAME {
                surname = value.map(String::from);
            } else if oid == &oid_registry::OID_X509_COUNTRY_NAME {
                country = value.map(String::from);
            }
        }
    }

    // === 有效期 ===
    let not_before = x509.validity().not_before.to_datetime().unix_timestamp() * 1000;
    let not_after = x509.validity().not_after.to_datetime().unix_timestamp() * 1000;

    Ok(P12CertInfo {
        fingerprint,
        expires_at: not_after,
        not_before,
        common_name,
        serial_number,
        organization_id,
        organization,
        given_name,
        surname,
        country,
        issuer,
    })
}

/// 验证 P12 内部证书链
///
/// 1. 验证链中每一级的签名 (cert[i] 由 cert[i+1] 签发)
/// 2. 链的最顶层证书 (root/anchor) 的 SHA256 指纹必须在受信列表中
fn verify_chain(cert_ders: &[&[u8]]) -> Result<()> {
    // 验证链中每一级签名
    for i in 0..cert_ders.len().saturating_sub(1) {
        let child = cert_ders[i];
        let parent = cert_ders[i + 1];

        let parent_pem = pem_from_der(parent);
        let child_tbs = crate::trust::extract_tbs_from_der(child)?;

        let (_, child_x509) = x509_parser::parse_x509_certificate(child)
            .map_err(|e| CertError::ValidationFailed(format!("Chain cert parse error: {e}")))?;

        crate::crypto::verify(&parent_pem, child_tbs, child_x509.signature_value.as_ref())
            .map_err(|_| {
                CertError::ValidationFailed(
                    "Certificate chain signature verification failed".into(),
                )
            })?;
    }

    // 链的最顶层证书应该是受信任的根 CA
    let anchor = cert_ders.last().unwrap();
    let anchor_fingerprint = hex::encode(Sha256::digest(anchor));

    if !TRUSTED_ROOT_FINGERPRINTS.contains(&anchor_fingerprint.as_str()) {
        let (_, anchor_x509) = x509_parser::parse_x509_certificate(anchor)
            .map_err(|e| CertError::ValidationFailed(format!("Anchor parse error: {e}")))?;

        let issuer_org = anchor_x509
            .issuer()
            .iter_organization()
            .next()
            .and_then(|o| o.as_str().ok())
            .unwrap_or("Unknown");

        return Err(CertError::ValidationFailed(format!(
            "Certificate root CA not recognized by AEAT (fingerprint: {anchor_fingerprint}, issuer: {issuer_org}). \
             Contact support to add your CA."
        )));
    }

    Ok(())
}

/// DER 转 PEM
fn pem_from_der(der: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(der);
    format!("-----BEGIN CERTIFICATE-----\n{b64}\n-----END CERTIFICATE-----")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用真实 FNMT 证书测试完整流程
    ///
    /// 需要: /tmp/test_fnmt.p12 (从 Documents 复制)
    /// 跳过条件: 文件不存在时自动 skip
    #[test]
    fn test_parse_real_fnmt_p12() {
        let p12_path = "/tmp/test_fnmt.p12";
        let password = "yuqingxiang";

        let data = match std::fs::read(p12_path) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("SKIP: {p12_path} not found");
                return;
            }
        };

        let info = parse_p12(&data, password).expect("parse_p12 should succeed");

        // 基础字段
        assert!(!info.fingerprint.is_empty());
        assert!(info.not_before < info.expires_at);

        // Subject 字段
        assert!(
            info.common_name.contains("XIANG"),
            "got: {}",
            info.common_name
        );
        assert!(
            info.common_name.contains("YUQING"),
            "got: {}",
            info.common_name
        );

        // 税号
        let sn = info
            .serial_number
            .as_deref()
            .expect("serial_number should exist");
        assert!(sn.contains("Y1767970C"), "got: {sn}");

        // tax_id() 辅助方法
        let tax_id = info.tax_id().expect("tax_id() should return Some");
        assert_eq!(tax_id, "Y1767970C");

        // 国家
        assert_eq!(info.country.as_deref(), Some("ES"));

        // Issuer
        assert_eq!(info.issuer, "FNMT-RCM");

        // 打印全部信息供人工检查
        eprintln!("=== P12CertInfo ===");
        eprintln!("fingerprint:     {}", info.fingerprint);
        eprintln!("common_name:     {}", info.common_name);
        eprintln!("serial_number:   {:?}", info.serial_number);
        eprintln!("organization_id: {:?}", info.organization_id);
        eprintln!("organization:    {:?}", info.organization);
        eprintln!("given_name:      {:?}", info.given_name);
        eprintln!("surname:         {:?}", info.surname);
        eprintln!("country:         {:?}", info.country);
        eprintln!("issuer:          {}", info.issuer);
        eprintln!("tax_id():        {:?}", info.tax_id());
        eprintln!("not_before:      {}", info.not_before);
        eprintln!("expires_at:      {}", info.expires_at);
    }

    #[test]
    fn test_parse_p12_wrong_password() {
        let p12_path = "/tmp/test_fnmt.p12";
        let data = match std::fs::read(p12_path) {
            Ok(d) => d,
            Err(_) => return,
        };

        let result = parse_p12(&data, "wrong_password");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_p12_invalid_data() {
        let result = parse_p12(b"not a p12 file", "password");
        assert!(result.is_err());
    }
}
