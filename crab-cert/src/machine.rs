use std::ptr::without_provenance;

use sha2::{Digest, Sha256};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// Generate a unique and stable Hardware ID for the machine.
///
/// This function aggregates various hardware characteristics to produce a SHA-256 fingerprint.
/// It aims to be stable across reboots and OS updates, but unique to the physical hardware.
///
/// Factors considered:
/// - CPU Brand and Vendor
/// - Number of Physical Cores
/// - Total Memory size
/// - System Name (OS Type)
///
/// Note: This does not currently include GPU information as it requires platform-specific calls
/// or heavier dependencies, but CPU and Memory provide a reasonable baseline for consistency.
pub fn generate_hardware_id() -> String {
    let mut hasher = Sha256::new();

    // Only refresh what we need to be efficient
    let refresh_kind = RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::everything())
        .with_memory(MemoryRefreshKind::everything());

    let sys = System::new_with_specifics(refresh_kind);

    // 1. System Name (e.g., "Darwin", "Linux", "Windows")
    // This is unlikely to change unless the OS is completely replaced (e.g., Windows -> Linux).
    if let Some(name) = System::name() {
        hasher.update(name.as_bytes());
        hasher.update(b"|");
    }

    // 2. CPU Information
    // We use the first CPU's info as representative of the package.
    if let Some(cpu) = sys.cpus().first() {
        hasher.update(cpu.brand().as_bytes());
        hasher.update(b"|");
        hasher.update(cpu.vendor_id().as_bytes());
        hasher.update(b"|");
    }
    // Number of CPUs (threads)
    hasher.update(sys.cpus().len().to_string().as_bytes());
    hasher.update(b"|");

    // 3. Total Memory
    // We assume total memory doesn't change frequently.
    hasher.update(sys.total_memory().to_string().as_bytes());

    // 4. (Optional) Network MACs could be added here, but they can be unstable (dongles, etc.)
    // We omit them for better stability.

    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_id_stability() {
        let id1 = generate_hardware_id();
        let id2 = generate_hardware_id();
        assert_eq!(id1, id2, "Hardware ID should be stable");
        assert_eq!(id1.len(), 64, "Hardware ID should be SHA256 hex string");
    }

    #[test]
    fn test_hardware_id_not_empty() {
        let id = generate_hardware_id();
        println!("Generated Hardware ID: {}", id);
        assert!(!id.is_empty());
    }
}
