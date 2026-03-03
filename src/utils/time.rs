use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time in milliseconds since UNIX epoch.
///
/// Returns 0 if the system clock is set before the Unix epoch (1970-01-01),
/// which should never happen on correctly-configured systems. Returns `u64::MAX`
/// if the milliseconds value overflows u64 (won't happen for ~584 million years).
pub fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
