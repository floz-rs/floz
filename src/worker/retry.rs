//! Retry policies for failed tasks.

use std::time::Duration;

/// Calculate the exponential backoff delay for the given retry attempt.
/// Returns delay in seconds.
pub fn exponential_backoff(retry_count: u32) -> Duration {
    // Base delay: 2^retry_count seconds, capped at ~4 hours
    // Retry 1 = 2s
    // Retry 2 = 4s
    // Retry 3 = 8s
    // Retry 4 = 16s
    let exp = std::cmp::min(retry_count, 14); // 2^14 = 16384s ≈ 4.5 hours
    let secs = 1_u64.checked_shl(exp).unwrap_or(16384);
    
    // Add jitter: up to 20%
    let jitter = (secs as f64 * 0.2) as u64;
    let actual = secs + (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64 % (jitter.max(1)));
    
    Duration::from_secs(actual)
}
