//! HTTP client retry tests using mockito to stub the Tetrac API.
//!
//! Locks in the contract that:
//! - 429 (RateLimited) is retried up to max_retries+1 attempts.
//! - 5xx Api responses are NOT retried today (matches is_retryable() in error.rs).
//! - 4xx auth/validation responses (401/403/422) are not retried.
//! - max_retries config is honored.
//! - Retry backoff sleeps `retry_delay_ms × attempt` between attempts.
//! - Transport errors (e.g. unreachable host) ARE retried via the separate
//!   client.rs:94 arm, even though `is_retryable()` is not consulted on them.

use mockito::{Matcher, Server};
use skill_trading::api::Client;
use skill_trading::config::{ApiConfig, AppConfig};
use skill_trading::error::TtcError;
use skill_trading::models::ExchangeCredentials;
use std::time::Instant;

fn config_for(server_url: &str, max_retries: u32, retry_delay_ms: u64) -> AppConfig {
    AppConfig {
        api_key: Some("test-auth-token".into()),
        public_key: Some("test-public-key".into()),
        api: ApiConfig {
            base_url: server_url.to_string(),
            timeout: 5,
            max_retries,
            retry_delay_ms,
        },
        ..Default::default()
    }
}

fn fake_creds() -> ExchangeCredentials {
    ExchangeCredentials {
        api_key: "x".into(),
        api_secret: "y".into(),
        passphrase: None,
        wallet_address: None,
    }
}

const SUCCESS_BODY: &str =
    r#"{"success":true,"data":[{"asset":"USDT","balance":100.0,"available":100.0}]}"#;

// ============================================================================
// Happy path
// ============================================================================

#[tokio::test]
async fn success_first_try_makes_one_call() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SUCCESS_BODY)
        .expect(1)
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 3, 1);
    let client = Client::new(&cfg).unwrap();
    client
        .get_balance("orderly", fake_creds())
        .await
        .expect("ok");
    m.assert_async().await;
}

// ============================================================================
// 429 — retryable
// ============================================================================

#[tokio::test]
async fn transient_429_then_200_retries_and_succeeds() {
    let mut server = Server::new_async().await;
    // First call: 429 with retry-after. Second call: 200.
    let first = server
        .mock("POST", "/exchanges")
        .with_status(429)
        .with_header("retry-after", "0")
        .with_body(r#"{"error":"slow down"}"#)
        .expect(1)
        .create_async()
        .await;
    let second = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(SUCCESS_BODY)
        .expect(1)
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 3, 1);
    let client = Client::new(&cfg).unwrap();
    client
        .get_balance("orderly", fake_creds())
        .await
        .expect("ok after retry");

    first.assert_async().await;
    second.assert_async().await;
}

#[tokio::test]
async fn persistent_429_fails_after_max_retries_plus_one() {
    let mut server = Server::new_async().await;
    let max_retries = 3;
    // Initial attempt + max_retries retries = 4 calls total
    let m = server
        .mock("POST", "/exchanges")
        .with_status(429)
        .with_header("retry-after", "0")
        .with_body(r#"{"error":"rate limit"}"#)
        .expect((max_retries + 1) as usize)
        .create_async()
        .await;

    let cfg = config_for(&server.url(), max_retries, 1);
    let client = Client::new(&cfg).unwrap();
    let err = client
        .get_balance("orderly", fake_creds())
        .await
        .expect_err("must fail");

    assert!(
        matches!(err, TtcError::RateLimited(_)),
        "expected RateLimited, got {err:?}"
    );
    m.assert_async().await;
}

#[tokio::test]
async fn max_retries_one_means_two_total_attempts() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/exchanges")
        .with_status(429)
        .with_header("retry-after", "0")
        .with_body("{}")
        .expect(2) // 1 initial + 1 retry
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 1, 1);
    let client = Client::new(&cfg).unwrap();
    let _ = client.get_balance("orderly", fake_creds()).await;
    m.assert_async().await;
}

// ============================================================================
// 5xx — locks in current behavior: NOT retried (Api{code:5xx} fails is_retryable)
// ============================================================================

#[tokio::test]
async fn api_503_is_not_retried_today() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/exchanges")
        .with_status(503)
        .with_body("Service Unavailable")
        .expect(1) // current behavior: only 1 attempt
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 3, 1);
    let client = Client::new(&cfg).unwrap();
    let err = client
        .get_balance("orderly", fake_creds())
        .await
        .expect_err("must fail");

    assert!(
        matches!(err, TtcError::Api { code: 503, .. }),
        "expected Api{{code:503}}, got {err:?}"
    );
    m.assert_async().await;
}

#[tokio::test]
async fn api_500_is_not_retried_today() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/exchanges")
        .with_status(500)
        .with_body("Internal Server Error")
        .expect(1)
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 3, 1);
    let client = Client::new(&cfg).unwrap();
    let err = client
        .get_balance("orderly", fake_creds())
        .await
        .expect_err("must fail");
    assert!(matches!(err, TtcError::Api { code: 500, .. }));
    m.assert_async().await;
}

// ============================================================================
// 4xx — never retried
// ============================================================================

async fn assert_no_retry_for_status(status: u16) {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/exchanges")
        .with_status(status as usize)
        .with_body(format!(r#"{{"error":"status {status}"}}"#))
        .expect(1)
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 3, 1);
    let client = Client::new(&cfg).unwrap();
    let err = client
        .get_balance("orderly", fake_creds())
        .await
        .expect_err("must fail");
    match err {
        TtcError::Api { code, .. } => assert_eq!(code, status, "wrong code"),
        other => panic!("expected Api error, got {other:?}"),
    }
    m.assert_async().await;
}

#[tokio::test]
async fn api_401_is_not_retried() {
    assert_no_retry_for_status(401).await;
}

#[tokio::test]
async fn api_403_is_not_retried() {
    assert_no_retry_for_status(403).await;
}

#[tokio::test]
async fn api_422_is_not_retried() {
    assert_no_retry_for_status(422).await;
}

#[tokio::test]
async fn api_400_is_not_retried() {
    assert_no_retry_for_status(400).await;
}

// ============================================================================
// Backoff is observable
// ============================================================================

#[tokio::test]
async fn retry_backoff_sleeps_between_attempts() {
    // retry_delay_ms = 100, max_retries = 2 → sleeps after attempt 1 (100ms)
    // and after attempt 2 (200ms). Minimum total sleep ≈ 300ms.
    // We allow a generous upper bound to tolerate slow CI.
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(429)
        .with_header("retry-after", "0")
        .with_body("{}")
        .expect(3) // 1 initial + 2 retries
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 2, 100);
    let client = Client::new(&cfg).unwrap();

    let start = Instant::now();
    let _ = client.get_balance("orderly", fake_creds()).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() >= 250,
        "backoff too short: {:?} (want ≥250ms)",
        elapsed
    );
    assert!(
        elapsed.as_millis() <= 5_000,
        "backoff too long: {:?} (want ≤5s)",
        elapsed
    );
}

// ============================================================================
// Transport-layer error retry (network/connection failures)
//
// These errors come back as Err on `inner.execute(request)` BEFORE
// handle_response runs, so they're retried via the separate arm at
// client.rs:94, regardless of is_retryable(). Verified by pointing the
// client at an unreachable port and watching it fail after taking longer
// than a single attempt's timeout.
// ============================================================================

#[tokio::test]
async fn transport_error_is_retried_via_separate_arm() {
    // Port 1 is reserved (tcpmux); connection refused on every attempt.
    let cfg = AppConfig {
        api_key: Some("x".into()),
        public_key: Some("y".into()),
        api: ApiConfig {
            base_url: "http://127.0.0.1:1".into(),
            timeout: 1,
            max_retries: 2,
            retry_delay_ms: 50,
        },
        ..Default::default()
    };
    let client = Client::new(&cfg).unwrap();

    let start = Instant::now();
    let err = client
        .get_balance("orderly", fake_creds())
        .await
        .expect_err("connection refused");
    let elapsed = start.elapsed();

    // After max_retries are exhausted, becomes TtcError::Request.
    assert!(matches!(err, TtcError::Request(_)), "got {err:?}");

    // 1 initial + 2 retries = 3 attempts, with sleeps of 50ms + 100ms = 150ms
    // between them. Even with very fast connection-refused responses, this
    // should take at least 100ms — proving retries happened.
    assert!(
        elapsed.as_millis() >= 100,
        "transport retry too fast (likely no retry happened): {:?}",
        elapsed
    );
}

// ============================================================================
// Sanity: request shape sent to Tetrac looks correct (auth headers, JSON body)
// ============================================================================

#[tokio::test]
async fn request_carries_auth_headers_and_json_body() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("POST", "/exchanges")
        .match_header("ttc-auth-token", "test-auth-token")
        .match_header("ttc-public-key", "test-public-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJsonString(
            r#"{"exchangeName":"orderly","method":"getBalance"}"#.to_string(),
        ))
        .with_status(200)
        .with_body(SUCCESS_BODY)
        .expect(1)
        .create_async()
        .await;

    let cfg = config_for(&server.url(), 0, 1);
    let client = Client::new(&cfg).unwrap();
    client
        .get_balance("orderly", fake_creds())
        .await
        .expect("ok");
    m.assert_async().await;
}
