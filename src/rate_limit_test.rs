use crate::rate_limit::RateLimitConfig;

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
