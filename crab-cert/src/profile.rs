use rcgen::{
    BasicConstraints, CertificateParams, CustomExtension, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyUsagePurpose,
};
use time::{Duration, OffsetDateTime};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum KeyType {
    #[default]
    P256,
    Rsa2048,
    Rsa4096,
}

// OIDs for internal usage
pub const OID_TENANT_ID: &[u64] = &[1, 3, 6, 1, 4, 1, 99999, 1];
pub const OID_DEVICE_ID: &[u64] = &[1, 3, 6, 1, 4, 1, 99999, 2];
pub const OID_CLIENT_NAME: &[u64] = &[1, 3, 6, 1, 4, 1, 99999, 5];

#[derive(Clone, Debug)]
pub struct CaProfile {
    pub common_name: String,
    pub organization: String,
    pub validity_days: u32,
    pub path_len: Option<u8>,
    pub key_type: KeyType,
}

impl CaProfile {
    pub fn root(common_name: &str) -> Self {
        Self {
            common_name: common_name.to_string(),
            organization: "Crab Inc.".to_string(),
            validity_days: 365 * 20,
            path_len: Some(1), // Allow 1 level of intermediate CA
            key_type: KeyType::default(),
        }
    }

    pub fn intermediate(common_name: &str, organization: &str) -> Self {
        Self {
            common_name: common_name.to_string(),
            organization: organization.to_string(),
            validity_days: 365 * 5,
            path_len: Some(0), // Can sign leaf certs, but not other CAs
            key_type: KeyType::default(),
        }
    }
}

impl Default for CaProfile {
    fn default() -> Self {
        Self {
            common_name: "Crab Root CA".to_string(),
            organization: "Crab Inc.".to_string(),
            validity_days: 365 * 20, // 20 years
            path_len: None,
            key_type: KeyType::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CertProfile {
    pub common_name: String,
    pub organization: String,
    pub sans: Vec<String>,
    pub validity_days: u32,
    pub is_client: bool,
    pub is_server: bool,
    pub tenant_id: Option<String>,
    pub device_id: Option<String>,
    pub client_name: Option<String>,
    pub key_type: KeyType,
}

impl CertProfile {
    pub fn new_server(
        common_name: &str,
        mut sans: Vec<String>,
        tenant_id: Option<String>,
        device_id: String,
    ) -> Self {
        if !sans.contains(&common_name.to_string()) {
            sans.push(common_name.to_string());
        }
        Self {
            common_name: common_name.to_string(),
            organization: "Crab Tenant".to_string(),
            sans,
            validity_days: 365, // 1 year for LAN cert
            is_client: false,
            is_server: true,
            tenant_id,
            device_id: Some(device_id),
            client_name: None,
            key_type: KeyType::default(),
        }
    }

    pub fn new_client(
        common_name: &str,
        tenant_id: Option<String>,
        device_id: Option<String>,
        client_name: Option<String>,
    ) -> Self {
        Self {
            common_name: common_name.to_string(),
            organization: "Crab Tenant".to_string(),
            sans: vec![],
            validity_days: 365,
            is_client: true,
            is_server: false,
            tenant_id,
            device_id,
            client_name,
            key_type: KeyType::default(),
        }
    }

    pub fn new_uplink(common_name: &str) -> Self {
        Self {
            common_name: common_name.to_string(),
            organization: "Crab Cloud".to_string(),
            sans: vec![],
            validity_days: 30, // 30 days for Uplink
            is_client: true,
            is_server: false,
            tenant_id: None,
            device_id: None,
            client_name: None,
            key_type: KeyType::default(),
        }
    }
}

pub(crate) fn create_ca_params(profile: &CaProfile) -> CertificateParams {
    let mut params = CertificateParams::new(Vec::new()).unwrap();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &profile.common_name);
    dn.push(DnType::OrganizationName, &profile.organization);
    params.distinguished_name = dn;

    // Set CA flag based on profile type - only intermediate/root CAs should be CA
    // 简单修复：所有终端证书设为非CA，CA证书按需处理
    let is_leaf_cert = profile.common_name.starts_with("edge-")
        || profile.common_name.starts_with("server-")
        || profile.common_name.starts_with("client-");

    params.is_ca = if is_leaf_cert {
        IsCa::NoCa
    } else {
        IsCa::Ca(BasicConstraints::Unconstrained)
    };
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];

    // Set validity
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(profile.validity_days as i64);

    params
}

fn encode_utf8_string(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut encoded = Vec::new();

    // Tag for UTF8String is 0x0C
    encoded.push(0x0C);

    // Length
    if len < 128 {
        encoded.push(len as u8);
    } else {
        // Calculate number of bytes needed for length
        let mut len_bytes = Vec::new();
        let mut l = len;
        while l > 0 {
            len_bytes.push((l & 0xFF) as u8);
            l >>= 8;
        }
        len_bytes.reverse();

        encoded.push(0x80 | len_bytes.len() as u8);
        encoded.extend(len_bytes);
    }

    // Value
    encoded.extend_from_slice(bytes);
    encoded
}

pub(crate) fn create_cert_params(profile: &CertProfile) -> CertificateParams {
    let mut params = CertificateParams::new(profile.sans.clone()).unwrap();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &profile.common_name);
    dn.push(DnType::OrganizationName, &profile.organization);
    params.distinguished_name = dn;

    params.is_ca = IsCa::NoCa;

    let key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    let mut extended_key_usages = vec![];

    if profile.is_server {
        extended_key_usages.push(ExtendedKeyUsagePurpose::ServerAuth);
    }
    if profile.is_client {
        extended_key_usages.push(ExtendedKeyUsagePurpose::ClientAuth);
    }

    params.key_usages = key_usages;
    params.extended_key_usages = extended_key_usages;

    // Custom Extensions
    if let Some(tenant_id) = &profile.tenant_id {
        let content = encode_utf8_string(tenant_id);
        let mut ext = CustomExtension::from_oid_content(OID_TENANT_ID, content);
        ext.set_criticality(false);
        params.custom_extensions.push(ext);
    }

    if let Some(device_id) = &profile.device_id {
        let content = encode_utf8_string(device_id);
        let mut ext = CustomExtension::from_oid_content(OID_DEVICE_ID, content);
        ext.set_criticality(false);
        params.custom_extensions.push(ext);
    }

    if let Some(client_name) = &profile.client_name {
        let content = encode_utf8_string(client_name);
        let mut ext = CustomExtension::from_oid_content(OID_CLIENT_NAME, content);
        ext.set_criticality(false);
        params.custom_extensions.push(ext);
    }

    // SANs are already set by CertificateParams::new

    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(profile.validity_days as i64);

    params
}
