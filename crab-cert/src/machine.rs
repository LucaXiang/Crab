use sha2::{Digest, Sha256};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// Generate a stable Hardware ID for the machine
///
/// This function aggregates hardware characteristics to produce a stable SHA-256 fingerprint.
/// It aims to be stable across reboots and unique to the physical hardware.
///
/// Factors considered:
/// - System name and architecture
/// - CPU brand and vendor ID  
/// - Number of physical cores
/// - Total memory size
pub fn generate_hardware_id() -> String {
    let mut hasher = Sha256::new();

    // System name (e.g., "Darwin", "Linux", "Windows")
    if let Some(name) = System::name() {
        hasher.update(name.as_bytes());
        hasher.update(b"|");
    }

    // CPU information - stable characteristics
    let refresh_kind = RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::everything())
        .with_memory(MemoryRefreshKind::everything());
    let sys = System::new_with_specifics(refresh_kind);

    // Use first CPU as representative
    if let Some(cpu) = sys.cpus().first() {
        hasher.update(cpu.brand().as_bytes());
        hasher.update(b"|");
        hasher.update(cpu.vendor_id().as_bytes());
        hasher.update(b"|");
    }

    // Number of physical cores (more stable than thread count)
    let physical_cores = System::physical_core_count().unwrap_or(sys.cpus().len());
    hasher.update(physical_cores.to_string().as_bytes());
    hasher.update(b"|");

    // Total memory - stable characteristic
    hasher.update(sys.total_memory().to_string().as_bytes());

    hex::encode(hasher.finalize())
}

/// Generate a lightweight hardware fingerprint for quick verification
///
/// This returns the first 16 characters of the full hardware ID
/// for rapid checks where full security isn't needed.
pub fn generate_quick_hardware_id() -> String {
    let hardware_id = generate_hardware_id();
    hardware_id[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_id_stability() {
        let id1 = generate_hardware_id();
        let id2 = generate_hardware_id();
        assert_eq!(id1, id2, "Hardware ID should be stable across calls");
        assert_eq!(id1.len(), 64, "Hardware ID should be SHA256 hex string");

        let quick_id1 = generate_quick_hardware_id();
        let quick_id2 = generate_quick_hardware_id();
        assert_eq!(quick_id1, quick_id2, "Quick hardware ID should be stable");
        assert_eq!(quick_id1.len(), 16, "Quick hardware ID should be 16 chars");
    }

    #[test]
    fn test_hardware_id_not_empty() {
        let id = generate_hardware_id();
        let quick_id = generate_quick_hardware_id();

        println!("Generated Hardware ID: {}", id);
        println!("Generated Quick Hardware ID: {}", quick_id);

        assert!(!id.is_empty(), "Hardware ID should not be empty");
        assert!(
            !quick_id.is_empty(),
            "Quick hardware ID should not be empty"
        );
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "Hardware ID should be valid hex"
        );
        assert!(
            quick_id.chars().all(|c| c.is_ascii_hexdigit()),
            "Quick hardware ID should be valid hex"
        );
    }
}
