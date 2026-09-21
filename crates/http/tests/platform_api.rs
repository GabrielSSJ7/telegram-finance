// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{ApiHarness, send, test_options};
use http_api::{RateLimit, RouterOptions};

fn plain_get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn missing_or_wrong_token_is_unauthorized() {
    let api = ApiHarness::new().await;
    let (status, problem) = send(&api.router, plain_get("/api/v1/accounts")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(problem["detail"].as_str().unwrap().contains("Bearer"));
    let wrong =
        Request::builder().uri("/api/v1/accounts").header(header::AUTHORIZATION, "Bearer fbk_nope");
    let (status, _) = send(&api.router, wrong.body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn healthz_reports_status_without_auth() {
    let healthy = ApiHarness::new().await;
    let (status, report) = send(&healthy.router, plain_get("/healthz")).await;
    assert_eq!((status, report["healthy"].as_bool()), (StatusCode::OK, Some(true)));
    let sick = ApiHarness::with(test_options(), false).await;
    let (status, report) = send(&sick.router, plain_get("/healthz")).await;
    assert_eq!(
        (status, report["checks"][0]["detail"].as_str()),
        (StatusCode::SERVICE_UNAVAILABLE, Some("connection refused"))
    );
}

#[tokio::test]
async fn swagger_serves_openapi_only_when_enabled() {
    let with_docs = ApiHarness::new().await;
    let (status, spec) = send(&with_docs.router, plain_get("/api-docs/openapi.json")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(spec["paths"]["/api/v1/entries"].is_object());
    assert!(spec["components"]["securitySchemes"]["api_key"].is_object());
    let without = ApiHarness::with(RouterOptions { swagger: false, ..test_options() }, true).await;
    let (status, _) = send(&without.router, plain_get("/api-docs/openapi.json")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rate_limit_rejects_bursts_per_client_ip() {
    let limit = RateLimit { replenish_every: Duration::from_mins(1), burst: 2 };
    let api =
        ApiHarness::with(RouterOptions { rate_limit: Some(limit), ..test_options() }, true).await;
    let from = |ip: &str| {
        Request::builder()
            .uri("/healthz")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(send(&api.router, from("10.0.0.1")).await.0, StatusCode::OK);
    assert_eq!(send(&api.router, from("10.0.0.1")).await.0, StatusCode::OK);
    assert_eq!(send(&api.router, from("10.0.0.1")).await.0, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(send(&api.router, from("10.0.0.2")).await.0, StatusCode::OK);
}
