//! 价格转换工具模块
//!
//! 提供元 <-> 分之间的转换函数，确保金额计算的精度。

/// 将元转换为分 (四舍五入)
///
/// # Examples
///
/// ```
/// use app_lib::utils::price::yuan_to_cents;
///
/// assert_eq!(yuan_to_cents(12.50), 1250);
/// assert_eq!(yuan_to_cents(0.01), 1);
/// assert_eq!(yuan_to_cents(100.00), 10000);
/// ```
pub fn yuan_to_cents(yuan: f64) -> i64 {
    (yuan * 100.0).round() as i64
}

/// 将分转换为元
///
/// # Examples
///
/// ```
/// use app_lib::utils::price::cents_to_yuan;
///
/// assert!((cents_to_yuan(1250) - 12.50).abs() < 0.001);
/// assert!((cents_to_yuan(1) - 0.01).abs() < 0.001);
/// ```
pub fn cents_to_yuan(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// 安全地将 Optional 元转换为 Optional 分
pub fn opt_yuan_to_opt_cents(yuan: Option<f64>) -> Option<i64> {
    yuan.map(yuan_to_cents)
}

/// 安全地将 Optional 分转换为 Optional 元
pub fn opt_cents_to_opt_yuan(cents: Option<i64>) -> Option<f64> {
    cents.map(cents_to_yuan)
}

/// 格式化金额为货币字符串 (欧元)
///
/// # Examples
///
/// ```
/// use app_lib::utils::price::format_yuan;
///
/// assert_eq!(format_yuan(12.50), "12.50€");
/// assert_eq!(format_yuan(100.00), "100.00€");
/// ```
pub fn format_yuan(yuan: f64) -> String {
    format!("{:.2}€", yuan)
}

/// 计算折扣后金额
///
/// - `discount_type`: "PERCENTAGE" 或 "FIXED_AMOUNT"
/// - `discount_value`: 折扣值 (百分比或固定金额)
pub fn apply_discount(cents: i64, discount_type: &str, discount_value: f64) -> i64 {
    match discount_type {
        "PERCENTAGE" => {
            // 使用简单的浮点运算，精度对于折扣场景足够
            let result = (cents as f64 * (1.0 - discount_value / 100.0)).round() as i64;
            result.max(0)
        }
        "FIXED_AMOUNT" => {
            let fixed = yuan_to_cents(discount_value);
            cents.saturating_sub(fixed).max(0)
        }
        _ => cents,
    }
}

/// 计算附加费
///
/// - `surcharge_type`: "PERCENTAGE" 或 "FIXED_AMOUNT"
/// - `surcharge_value`: 附加费值
pub fn apply_surcharge(cents: i64, surcharge_type: &str, surcharge_value: f64) -> i64 {
    match surcharge_type {
        "PERCENTAGE" => {
            // 使用简单的浮点运算
            (cents as f64 * (1.0 + surcharge_value / 100.0)).round() as i64
        }
        "FIXED_AMOUNT" => {
            let fixed = yuan_to_cents(surcharge_value);
            cents.saturating_add(fixed)
        }
        _ => cents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yuan_to_cents() {
        assert_eq!(yuan_to_cents(12.50), 1250);
        assert_eq!(yuan_to_cents(0.01), 1);
        assert_eq!(yuan_to_cents(100.00), 10000);
        assert_eq!(yuan_to_cents(0.00), 0);
    }

    #[test]
    fn test_cents_to_yuan() {
        assert!((cents_to_yuan(1250) - 12.50).abs() < 0.001);
        assert!((cents_to_yuan(1) - 0.01).abs() < 0.001);
        assert!((cents_to_yuan(10000) - 100.00).abs() < 0.001);
    }

    #[test]
    fn test_round_trip() {
        for price in [0.01, 0.99, 1.00, 12.50, 99.99, 100.00, 999.99] {
            let cents = yuan_to_cents(price);
            let back = cents_to_yuan(cents);
            assert!((back - price).abs() < 0.001, "Failed for {}", price);
        }
    }

    #[test]
    fn test_apply_discount_percentage() {
        assert_eq!(apply_discount(1000, "PERCENTAGE", 10.0), 900);
        assert_eq!(apply_discount(1000, "PERCENTAGE", 50.0), 500);
        assert_eq!(apply_discount(1000, "PERCENTAGE", 100.0), 0);
    }

    #[test]
    fn test_apply_discount_fixed() {
        assert_eq!(apply_discount(1000, "FIXED_AMOUNT", 5.00), 500);
        assert_eq!(apply_discount(1000, "FIXED_AMOUNT", 10.00), 0);
        assert_eq!(apply_discount(1000, "FIXED_AMOUNT", 0.50), 950);
    }

    #[test]
    fn test_apply_surcharge_percentage() {
        assert_eq!(apply_surcharge(1000, "PERCENTAGE", 10.0), 1100);
        assert_eq!(apply_surcharge(1000, "PERCENTAGE", 5.0), 1050);
    }

    #[test]
    fn test_apply_surcharge_fixed() {
        assert_eq!(apply_surcharge(1000, "FIXED_AMOUNT", 5.00), 1500);
        assert_eq!(apply_surcharge(1000, "FIXED_AMOUNT", 0.50), 1050);
    }

    #[test]
    fn test_format_yuan() {
        assert_eq!(format_yuan(12.50), "12.50€");
        assert_eq!(format_yuan(100.00), "100.00€");
        assert_eq!(format_yuan(0.01), "0.01€");
    }
}
