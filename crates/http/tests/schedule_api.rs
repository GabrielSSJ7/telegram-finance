// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::ApiHarness;
use serde_json::json;

#[tokio::test]
async fn recurrence_create_list_deactivate() {
    let api = ApiHarness::new().await;
    let account = api.open_checking("Nubank", 0).await;
    let salary = api.category("salário", "income").await;
    let body = json!({"kind": "income", "amount_cents": 800_000, "description": "Salário", "category_id": salary, "account_id": account, "day_of_month": 5});
    let (status, created) = api.post("/api/v1/recurrences", body).await;
    assert_eq!(
        (status, created["mode"].as_str()),
        (StatusCode::CREATED, Some("auto")),
        "{created}"
    );
    let (_, listed) = api.get("/api/v1/recurrences").await;
    assert_eq!(listed[0]["day_of_month"], 5);
    let uri = format!("/api/v1/recurrences/{}", created["id"].as_str().unwrap());
    assert_eq!(api.call(Method::DELETE, &uri, None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(api.call(Method::DELETE, &uri, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn recurrence_needs_exactly_one_target_and_valid_day() {
    let api = ApiHarness::new().await;
    let salary = api.category("salário", "income").await;
    let neither = json!({"kind": "income", "amount_cents": 1, "description": "x", "category_id": salary, "day_of_month": 5});
    let (status, problem) = api.post("/api/v1/recurrences", neither).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("exactly one"));
    let account = api.open_checking("Nubank", 0).await;
    let bad_day = json!({"kind": "income", "amount_cents": 1, "description": "x", "category_id": salary, "account_id": account, "day_of_month": 0});
    assert_eq!(api.post("/api/v1/recurrences", bad_day).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn daily_and_cycle_reports() {
    let api = ApiHarness::new().await;
    api.open_checking("Nubank", 100_000).await;
    let (status, daily) = api.get("/api/v1/reports/daily?date=2026-03-10").await;
    assert_eq!((status, daily["date"].as_str()), (StatusCode::OK, Some("2026-03-10")), "{daily}");
    assert_eq!(daily["balances"]["position"]["available"], 100_000);
    let (status, cycle) = api.get("/api/v1/reports/cycle").await;
    assert_eq!(
        (status, cycle["cycle"]["start"].as_str()),
        (StatusCode::OK, Some("2026-03-01")),
        "{cycle}"
    );
}
