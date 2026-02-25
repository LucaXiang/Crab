/// 获取当前 UTC 时间戳（毫秒）
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Generate a Snowflake-style i64 for use as resource ID.
///
/// Layout (53 bits, fits in JavaScript's Number.MAX_SAFE_INTEGER):
///   - 41 bits: milliseconds since 2024-01-01 UTC (~69 years)
///   - 12 bits: random (4096 values per ms, collision-free at POS scale)
///
/// Used by both edge-server and crab-cloud for unified ID generation.
pub fn snowflake_id() -> i64 {
    use rand::Rng;
    // Custom epoch: 2024-01-01 00:00:00 UTC
    const EPOCH_MS: i64 = 1_704_067_200_000;
    let now = now_millis();
    let ts = (now - EPOCH_MS) & 0x1FF_FFFF_FFFF; // 41 bits
    let rand_bits: i64 = rand::thread_rng().gen_range(0..0x1000); // 12 bits
    (ts << 12) | rand_bits
}
