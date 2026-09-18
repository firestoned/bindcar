use crate::rate_limit::{RateLimitConfig, MIN_BURST_FOR_ZONE_REPLAY};

#[test]
fn test_rate_limit_config_default() {
    let config = RateLimitConfig::default();
    assert_eq!(config.requests_per_period, 600);
    assert_eq!(config.period_secs, 60);
    assert_eq!(config.burst_size, MIN_BURST_FOR_ZONE_REPLAY);
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
    assert_eq!(config.requests_per_period, 600);
    assert_eq!(config.period_secs, 60);
    assert_eq!(config.burst_size, MIN_BURST_FOR_ZONE_REPLAY);
    assert!(config.enabled);
}

/// The configured budget must survive translation into governor's replenish
/// interval. Computing it as `requests_per_period / period_secs` in integer
/// arithmetic floors 100-per-60s to 1 request per second, which is a 40% cut
/// nobody configured — and it silently rate-limits the operator's zone replay
/// down to a crawl.
#[test]
fn test_replenish_interval_does_not_floor_the_configured_rate() {
    // The ratio that shipped as the default and exposed the bug.
    let config = RateLimitConfig {
        requests_per_period: 100,
        period_secs: 60,
        ..Default::default()
    };

    // 100 requests per 60s is one cell every 600ms, not every 1000ms.
    assert_eq!(config.replenish_period().as_millis(), 600);
}

/// A budget that is not a whole number of requests per second must not collapse
/// to 1/s either.
#[test]
fn test_replenish_interval_for_sub_second_rates() {
    let config = RateLimitConfig {
        requests_per_period: 30,
        period_secs: 60,
        ..Default::default()
    };
    // 30 per 60s is one cell every 2s.
    assert_eq!(config.replenish_period().as_millis(), 2000);
}

/// The operator replays a whole zone — create, freeze, several record updates,
/// thaw, notify — in one burst when a Pod comes back empty. A burst smaller
/// than that guarantees 429s on every recovery.
#[test]
fn test_default_burst_absorbs_a_zone_replay() {
    let config = RateLimitConfig::default();
    assert!(
        config.burst_size >= MIN_BURST_FOR_ZONE_REPLAY,
        "default burst {} cannot absorb a zone replay ({} calls)",
        config.burst_size,
        MIN_BURST_FOR_ZONE_REPLAY
    );
}
