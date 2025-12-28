//! Rate limiting middleware for HTTP requests
//!
//! This module provides rate limiting functionality using the tower-governor crate,
//! which implements the Generic Cell Rate Algorithm (GCRA). Rate limits can be
//! configured globally and are tracked per client IP address.

// Re-export commonly used types for convenience
pub use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
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
            requests_per_period: 100,
            period_secs: 60,
            burst_size: 10,
            enabled: true,
        }
    }
}

impl RateLimitConfig {
    /// Create configuration from environment variables
    ///
    /// Environment variables:
    /// - `RATE_LIMIT_ENABLED`: Enable/disable rate limiting (default: true)
    /// - `RATE_LIMIT_REQUESTS`: Max requests per period (default: 100)
    /// - `RATE_LIMIT_PERIOD_SECS`: Period in seconds (default: 60)
    /// - `RATE_LIMIT_BURST`: Burst size (default: 10)
    pub fn from_env() -> Self {
        let enabled = std::env::var("RATE_LIMIT_ENABLED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let requests_per_period = std::env::var("RATE_LIMIT_REQUESTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let period_secs = std::env::var("RATE_LIMIT_PERIOD_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let burst_size = std::env::var("RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            requests_per_period,
            period_secs,
            burst_size,
            enabled,
        }
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

#[cfg(test)]
mod rate_limit_test {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_period, 100);
        assert_eq!(config.period_secs, 60);
        assert_eq!(config.burst_size, 10);
        assert!(config.enabled);
    }

    #[test]
    fn test_rate_limit_config_validation() {
        let config = RateLimitConfig::default();
        assert!(config.validate().is_ok());

        let invalid_config = RateLimitConfig {
            requests_per_period: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = RateLimitConfig {
            period_secs: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());

        let invalid_config = RateLimitConfig {
            burst_size: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_rate_limit_config_from_env() {
        // Test with no env vars set - should use defaults
        std::env::remove_var("RATE_LIMIT_ENABLED");
        std::env::remove_var("RATE_LIMIT_REQUESTS");
        std::env::remove_var("RATE_LIMIT_PERIOD_SECS");
        std::env::remove_var("RATE_LIMIT_BURST");

        let config = RateLimitConfig::from_env();
        assert_eq!(config.requests_per_period, 100);
        assert_eq!(config.period_secs, 60);
        assert_eq!(config.burst_size, 10);
        assert!(config.enabled);
    }
}
