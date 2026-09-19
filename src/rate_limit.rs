//! Rate limiting middleware for HTTP requests
//!
//! This module provides rate limiting functionality using the tower-governor crate,
//! which implements the Generic Cell Rate Algorithm (GCRA). Rate limits are
//! keyed on the real TCP peer IP ([`PeerIpKeyExtractor`]) rather than the
//! spoofable `X-Forwarded-For` family of headers, so a client cannot evade the
//! limit or exhaust another client's bucket by forging a forwarding header.

use std::time::Duration;

/// Smallest burst that can absorb one zone replay without tripping the limiter.
///
/// When a BIND9 Pod is replaced it comes back with an empty zone directory, and
/// the operator immediately replays everything it should be serving: create the
/// zone, freeze it, apply each record, thaw, notify, then poll status. That is
/// comfortably more than a handful of calls, and it arrives as one burst per
/// zone. A burst below this guarantees HTTP 429 on every Pod restart — which the
/// operator answers with exponential backoff, turning a ~30s recovery into ~130s.
pub const MIN_BURST_FOR_ZONE_REPLAY: u32 = 50;

// Re-export commonly used types for convenience. These are the only part of
// this module that touches the HTTP stack — `RateLimitConfig` below is pure
// configuration and stays available to a library-only consumer.
#[cfg(feature = "server")]
pub use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorLayer,
};

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per period
    pub requests_per_period: u32,
    /// Period duration in seconds
    pub period_secs: u64,
    /// Burst size (max requests at once)
    pub burst_size: u32,
    /// Whether rate limiting is enabled
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_period: 600,
            period_secs: 60,
            // Sized for the operator's zone-replay burst, not for a browser.
            // bindcar's authenticated API has exactly one legitimate caller —
            // the operator's ServiceAccount, enforced by TokenReview — so a
            // burst this size is not an abuse surface, while a small one breaks
            // recovery.
            burst_size: MIN_BURST_FOR_ZONE_REPLAY,
            enabled: true,
        }
    }
}

impl RateLimitConfig {
    /// Create configuration from environment variables
    ///
    /// Environment variables:
    /// - `RATE_LIMIT_ENABLED`: Enable/disable rate limiting (default: true)
    /// - `RATE_LIMIT_REQUESTS`: Max requests per period (default: 600)
    /// - `RATE_LIMIT_PERIOD_SECS`: Period in seconds (default: 60)
    /// - `RATE_LIMIT_BURST`: Burst size (default: 50)
    pub fn from_env() -> Self {
        let enabled = std::env::var("RATE_LIMIT_ENABLED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let requests_per_period = std::env::var("RATE_LIMIT_REQUESTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600);

        let period_secs = std::env::var("RATE_LIMIT_PERIOD_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let burst_size = std::env::var("RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(MIN_BURST_FOR_ZONE_REPLAY);

        Self {
            requests_per_period,
            period_secs,
            burst_size,
            enabled,
        }
    }

    /// The interval at which governor replenishes one request cell.
    ///
    /// Governor is configured by *period per cell*, not by requests per period.
    /// Converting with `requests_per_period / period_secs` in integer arithmetic
    /// silently floors the budget — the 100-per-60s default became 1 request per
    /// second, a 40% cut, and any budget under one request per second collapsed
    /// to the same 1/s floor. Dividing in milliseconds keeps the configured rate.
    ///
    /// # Returns
    ///
    /// The delay between replenished cells; never zero, so governor always has a
    /// valid period even for an absurdly large `requests_per_period`.
    #[must_use]
    pub fn replenish_period(&self) -> Duration {
        let millis = self
            .period_secs
            .saturating_mul(1_000)
            .checked_div(u64::from(self.requests_per_period))
            .unwrap_or(1)
            .max(1);
        Duration::from_millis(millis)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.requests_per_period == 0 {
            return Err("requests_per_period must be greater than 0".to_string());
        }

        if self.period_secs == 0 {
            return Err("period_secs must be greater than 0".to_string());
        }

        if self.burst_size == 0 {
            return Err("burst_size must be greater than 0".to_string());
        }

        Ok(())
    }
}
