// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::ApiHarness;
use serde_json::json;

#[tokio::test]
async fn goal_lifecycle_through_api() {
    let api = ApiHarness::new().await;
    let checking = api.open_checking("Nubank", 500_000).await;
    let body = json!({"name": "Casa própria", "target_cents": 10_000_000, "already_saved_cents": 2_000_000});
    let (status, goal) = api.post("/api/v1/goals", body).await;
    assert_eq!(status, StatusCode::CREATED, "{goal}");
    let goal_uri = format!("/api/v1/goals/{}", goal["id"].as_str().unwrap());
    let deposit = json!({"account_id": checking, "amount_cents": 100_000});
    assert_eq!(api.post(&format!("{goal_uri}/deposits"), deposit).await.0, StatusCode::CREATED);
    let withdraw = json!({"account_id": checking, "amount_cents": 50_000});
    assert_eq!(api.post(&format!("{goal_uri}/withdrawals"), withdraw).await.0, StatusCode::CREATED);
    let (_, goals) = api.get("/api/v1/goals").await;
    assert_eq!(
        (goals[0]["saved_cents"].as_i64(), goals[0]["progress_bp"].as_i64()),
        (Some(2_050_000), Some(2050))
    );
    let target = json!({"target_cents": 20_000_000, "target_date": "2030-12-31"});
    let (status, updated) =
        api.call(Method::PUT, &format!("{goal_uri}/target"), Some(target)).await;
    assert_eq!((status, updated["target_cents"].as_i64()), (StatusCode::OK, Some(20_000_000)));
}

#[tokio::test]
async fn settings_read_and_patch() {
    let api = ApiHarness::new().await;
    let (status, settings) = api.get("/api/v1/settings").await;
    assert_eq!((status, settings["cycle_start_day"].as_u64()), (StatusCode::OK, Some(1)));
    let patch = json!({"cycle_start_day": 5, "daily_report_time": "20:30:00"});
    let (status, updated) = api.call(Method::PATCH, "/api/v1/settings", Some(patch)).await;
    assert_eq!((status, updated["daily_report_time"].as_str()), (StatusCode::OK, Some("20:30:00")));
    let (status, problem) =
        api.call(Method::PATCH, "/api/v1/settings", Some(json!({"cycle_start_day": 40}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("40"));
}

#[tokio::test]
async fn categories_list_by_kind_and_archive() {
    let api = ApiHarness::new().await;
    let groceries = api.category("mercado", "expense").await;
    api.category("salário", "income").await;
    let (_, expenses) = api.get("/api/v1/categories?kind=expense").await;
    assert_eq!(expenses.as_array().unwrap().len(), 1);
    let (status, _) =
        api.call(Method::DELETE, &format!("/api/v1/categories/{groceries}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) =
        api.call(Method::DELETE, &format!("/api/v1/categories/{groceries}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
