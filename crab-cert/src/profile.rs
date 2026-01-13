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
pub const OID_HARDWARE_ID: &[u64] = &[1, 3, 6, 1, 4, 1, 99999, 4];
#[allow(dead_code)]
pub const OID_CRAB_PROTOCOL: &[u64] = &[1, 3, 6, 1, 4, 1, 99999, 3];

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
    pub hardware_id: Option<String>,
    pub key_type: KeyType,
}

impl CertProfile {
    pub fn new_server(common_name: &str, sans: Vec<String>) -> Self {
        Self {
            common_name: common_name.to_string(),
            organization: "Crab Tenant".to_string(),
            sans,
            validity_days: 365, // 1 year for LAN cert
            is_client: false,
            is_server: true,
            tenant_id: None,
            device_id: None,
            hardware_id: None,
            key_type: KeyType::default(),
        }
    }

    pub fn new_client(
        common_name: &str,
        tenant_id: Option<String>,
        device_id: Option<String>,
        hardware_id: Option<String>,
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
            hardware_id,
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
            hardware_id: None,
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

    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(profile.path_len.unwrap_or(0)));
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];

    // Set validity
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(profile.validity_days as i64);

    params
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
        let mut ext =
            CustomExtension::from_oid_content(OID_TENANT_ID, tenant_id.as_bytes().to_vec());
        ext.set_criticality(false);
        params.custom_extensions.push(ext);
    }

    if let Some(device_id) = &profile.device_id {
        let mut ext =
            CustomExtension::from_oid_content(OID_DEVICE_ID, device_id.as_bytes().to_vec());
        ext.set_criticality(false);
        params.custom_extensions.push(ext);
    }

    if let Some(hardware_id) = &profile.hardware_id {
        let mut ext =
            CustomExtension::from_oid_content(OID_HARDWARE_ID, hardware_id.as_bytes().to_vec());
        ext.set_criticality(false);
        params.custom_extensions.push(ext);
    }

    // SANs are already set by CertificateParams::new

    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(profile.validity_days as i64);

    params
}
