//! Shared harness: the real router over the in-memory service set, with an
//! issued API key, driven through `tower::ServiceExt::oneshot`.

#![allow(dead_code)]

use std::sync::Arc;

use app::fakes::FakeServiceSet;
use app::services::AllowedUsers;
use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::NaiveDate;
use http_api::{ApiState, HealthCheck, HealthProbe, HealthReport, RouterOptions, api_router};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Health probe with a fixed answer.
pub struct StaticHealthProbe {
    pub healthy: bool,
}

#[async_trait]
impl HealthProbe for StaticHealthProbe {
    async fn report(&self) -> HealthReport {
        let detail = if self.healthy { "ok" } else { "connection refused" };
        let check =
            HealthCheck { name: "database".into(), healthy: self.healthy, detail: detail.into() };
        HealthReport::from_checks(vec![check])
    }
}

pub struct ApiHarness {
    pub set: FakeServiceSet,
    pub router: Router,
    pub token: String,
}

pub fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()
}

pub fn test_options() -> RouterOptions {
    RouterOptions { swagger: true, rate_limit: None, ..RouterOptions::default() }
}

impl ApiHarness {
    pub async fn new() -> Self {
        Self::with(test_options(), true).await
    }

    pub async fn with(options: RouterOptions, healthy: bool) -> Self {
        let set = FakeServiceSet::new(today(), AllowedUsers::default());
        let token = set.services.api_keys.issue("tests").await.unwrap().token;
        let state = ApiState {
            services: set.services.clone(),
            health: Arc::new(StaticHealthProbe { healthy }),
        };
        let router = api_router(state, &options);
        Self { set, router, token }
    }

    pub async fn call(
        &self,
        method: Method,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let request = self.request(method, uri).body(json_body(body)).unwrap();
        send(&self.router, request).await
    }

    pub fn request(&self, method: Method, uri: &str) -> axum::http::request::Builder {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::CONTENT_TYPE, "application/json")
    }

    pub async fn post(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, uri, Some(body)).await
    }

    pub async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.call(Method::GET, uri, None).await
    }

    /// Opens a checking account and returns its id.
    pub async fn open_checking(&self, name: &str, initial_cents: i64) -> String {
        let body = serde_json::json!({"name": name, "kind": "checking", "initial_balance_cents": initial_cents});
        let (status, account) = self.post("/api/v1/accounts", body).await;
        assert_eq!(status, StatusCode::CREATED, "{account}");
        account["id"].as_str().unwrap().to_owned()
    }

    /// Creates a category and returns its id.
    pub async fn category(&self, name: &str, kind: &str) -> String {
        let (status, category) =
            self.post("/api/v1/categories", serde_json::json!({"name": name, "kind": kind})).await;
        assert_eq!(status, StatusCode::CREATED, "{category}");
        category["id"].as_str().unwrap().to_owned()
    }
}

pub fn json_body(body: Option<Value>) -> Body {
    body.map_or_else(Body::empty, |value| Body::from(value.to_string()))
}

pub async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()));
    (status, value)
}
