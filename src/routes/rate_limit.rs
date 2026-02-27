//! IP-based rate limiting helpers for Store API endpoints.
//!
//! This module is intentionally route-scoped (HTTP-layer) and uses an in-memory,
//! per-process limiter.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use actix_web::HttpRequest;

pub const MAX_REQUESTS: u32 = 10;
pub const WINDOW_SECONDS: u64 = 60;
pub const TRUST_FORWARDED_HEADERS: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitExceeded {
    pub ip: IpAddr,
    pub retry_after: Duration,
}

#[derive(Debug)]
pub struct StoreOtpIpRateLimiter {
    state: Mutex<RateLimitState>,
}

#[derive(Debug, Default)]
struct RateLimitState {
    buckets: HashMap<IpAddr, VecDeque<Instant>>,
    last_global_cleanup: Option<Instant>,
}

impl StoreOtpIpRateLimiter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RateLimitState::default()),
        }
    }

    pub fn check(&self, req: &HttpRequest) -> Result<(), RateLimitExceeded> {
        let Some(ip) = extract_client_ip(req) else {
            return Ok(());
        };

        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();

        Self::check_ip_with_state(ip, now, &mut guard)
    }

    #[cfg(test)]
    fn check_ip_at(&self, ip: IpAddr, now: Instant) -> Result<(), RateLimitExceeded> {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        Self::check_ip_with_state(ip, now, &mut guard)
    }

    fn check_ip_with_state(
        ip: IpAddr,
        now: Instant,
        state: &mut RateLimitState,
    ) -> Result<(), RateLimitExceeded> {
        let window = Duration::from_secs(WINDOW_SECONDS);
        let max_requests = MAX_REQUESTS as usize;

        let should_cleanup = state
            .last_global_cleanup
            .is_none_or(|last| now.saturating_duration_since(last) >= window);

        if should_cleanup {
            cleanup_stale_buckets(state, now, window);
            state.last_global_cleanup = Some(now);
        }

        let entries = state.buckets.entry(ip).or_default();
        prune_bucket(entries, now, window);

        if entries.len() >= max_requests {
            let retry_after = entries
                .front()
                .map(|&oldest| window.saturating_sub(now.saturating_duration_since(oldest)))
                .unwrap_or(window);

            return Err(RateLimitExceeded { ip, retry_after });
        }

        entries.push_back(now);

        Ok(())
    }
}

fn extract_client_ip(req: &HttpRequest) -> Option<IpAddr> {
    if TRUST_FORWARDED_HEADERS
        && let Some(real_ip) = req.connection_info().realip_remote_addr()
        && let Some(ip) = parse_forwarded_ip(real_ip)
    {
        return Some(ip);
    }

    req.peer_addr().map(|addr| addr.ip())
}

impl Default for StoreOtpIpRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

fn cleanup_stale_buckets(state: &mut RateLimitState, now: Instant, window: Duration) {
    for entries in state.buckets.values_mut() {
        prune_bucket(entries, now, window);
    }
    state.buckets.retain(|_, entries| !entries.is_empty());
}

fn prune_bucket(entries: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while let Some(&front) = entries.front() {
        if now.saturating_duration_since(front) >= window {
            entries.pop_front();
        } else {
            break;
        }
    }
}

fn parse_forwarded_ip(raw_value: &str) -> Option<IpAddr> {
    let first_value = raw_value.split(',').next()?.trim();
    let unprefixed = first_value
        .strip_prefix("for=")
        .unwrap_or(first_value)
        .trim_matches('"');

    if let Ok(addr) = unprefixed.parse::<IpAddr>() {
        return Some(addr);
    }
    if let Ok(addr) = unprefixed.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_within_window() {
        let limiter = StoreOtpIpRateLimiter::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let t0 = Instant::now();

        for idx in 0..(MAX_REQUESTS as u64) {
            assert!(
                limiter
                    .check_ip_at(ip, t0 + Duration::from_secs(idx))
                    .is_ok()
            );
        }

        assert!(
            limiter
                .check_ip_at(ip, t0 + Duration::from_secs(MAX_REQUESTS as u64))
                .is_err()
        );
    }

    #[test]
    fn resets_after_window() {
        let limiter = StoreOtpIpRateLimiter::new();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let t0 = Instant::now();

        for idx in 0..(MAX_REQUESTS as u64) {
            assert!(
                limiter
                    .check_ip_at(ip, t0 + Duration::from_secs(idx))
                    .is_ok()
            );
        }

        let limited_at = t0 + Duration::from_secs(MAX_REQUESTS as u64);
        assert!(limiter.check_ip_at(ip, limited_at).is_err());

        let ok_at = t0 + Duration::from_secs(WINDOW_SECONDS);
        assert!(limiter.check_ip_at(ip, ok_at).is_ok());
    }

    #[test]
    fn retry_after_is_non_zero_when_limited() {
        let limiter = StoreOtpIpRateLimiter::new();
        let ip: IpAddr = "192.168.1.5".parse().unwrap();
        let t0 = Instant::now();

        for idx in 0..(MAX_REQUESTS as u64) {
            assert!(
                limiter
                    .check_ip_at(ip, t0 + Duration::from_secs(idx))
                    .is_ok()
            );
        }

        match limiter.check_ip_at(ip, t0 + Duration::from_secs(MAX_REQUESTS as u64)) {
            Err(RateLimitExceeded { retry_after, .. }) => {
                assert!(retry_after > Duration::from_secs(0));
                assert!(retry_after <= Duration::from_secs(WINDOW_SECONDS));
            }
            other => panic!("expected rate limit error, got {other:?}"),
        }
    }

    #[test]
    fn evicts_stale_ip_buckets() {
        let limiter = StoreOtpIpRateLimiter::new();
        let first_ip: IpAddr = "192.168.1.10".parse().unwrap();
        let second_ip: IpAddr = "192.168.1.11".parse().unwrap();
        let t0 = Instant::now();

        assert!(limiter.check_ip_at(first_ip, t0).is_ok());
        assert!(
            limiter
                .check_ip_at(second_ip, t0 + Duration::from_secs(WINDOW_SECONDS + 1))
                .is_ok()
        );

        let guard = limiter
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        assert!(!guard.buckets.contains_key(&first_ip));
        assert!(guard.buckets.contains_key(&second_ip));
    }

    #[test]
    fn does_not_panic_when_timestamps_are_out_of_order() {
        let limiter = StoreOtpIpRateLimiter::new();
        let ip: IpAddr = "172.16.0.9".parse().unwrap();
        let t1 = Instant::now();
        let t0 = t1 - Duration::from_secs(1);

        assert!(limiter.check_ip_at(ip, t1).is_ok());
        assert!(limiter.check_ip_at(ip, t0).is_ok());
    }

    #[test]
    fn parse_forwarded_ip_supports_plain_ip() {
        let parsed = parse_forwarded_ip("203.0.113.9");
        assert_eq!(parsed, Some("203.0.113.9".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn parse_forwarded_ip_supports_socket_addr() {
        let parsed = parse_forwarded_ip("203.0.113.9:9000");
        assert_eq!(parsed, Some("203.0.113.9".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn parse_forwarded_ip_supports_forwarded_format() {
        let parsed = parse_forwarded_ip("for=\"203.0.113.9\"");
        assert_eq!(parsed, Some("203.0.113.9".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn parse_forwarded_ip_uses_first_comma_separated_value() {
        let parsed = parse_forwarded_ip("203.0.113.9, 198.51.100.2");
        assert_eq!(parsed, Some("203.0.113.9".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn runs_global_cleanup_once_per_window() {
        let limiter = StoreOtpIpRateLimiter::new();
        let ip: IpAddr = "192.168.1.99".parse().unwrap();
        let t0 = Instant::now();

        assert!(limiter.check_ip_at(ip, t0).is_ok());
        {
            let guard = limiter
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert_eq!(guard.last_global_cleanup, Some(t0));
        }

        let t1 = t0 + Duration::from_secs(1);
        assert!(limiter.check_ip_at(ip, t1).is_ok());
        {
            let guard = limiter
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert_eq!(guard.last_global_cleanup, Some(t0));
        }

        let t2 = t0 + Duration::from_secs(WINDOW_SECONDS);
        assert!(limiter.check_ip_at(ip, t2).is_ok());
        {
            let guard = limiter
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert_eq!(guard.last_global_cleanup, Some(t2));
        }
    }
}
