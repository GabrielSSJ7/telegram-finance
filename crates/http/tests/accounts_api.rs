// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::ApiHarness;
use serde_json::json;

#[tokio::test]
async fn open_list_and_archive_account() {
    let api = ApiHarness::new().await;
    let id = api.open_checking("Nubank", 150_000).await;
    let (status, listed) = api.get("/api/v1/accounts").await;
    assert_eq!((status, listed[0]["name"].as_str()), (StatusCode::OK, Some("Nubank")));
    assert_eq!(listed[0]["opened_on"], "2026-03-10");
    let (status, _) = api.call(Method::DELETE, &format!("/api/v1/accounts/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, active) = api.get("/api/v1/accounts").await;
    assert_eq!(active.as_array().unwrap().len(), 0);
    let (_, all) = api.get("/api/v1/accounts?include_archived=true").await;
    assert_eq!(all[0]["archived"], true);
}

#[tokio::test]
async fn duplicate_name_is_conflict_and_pot_is_unprocessable() {
    let api = ApiHarness::new().await;
    api.open_checking("Nubank", 0).await;
    let (status, problem) =
        api.post("/api/v1/accounts", json!({"name": "nubank", "kind": "cash"})).await;
    assert_eq!((status, problem["status"].as_u64()), (StatusCode::CONFLICT, Some(409)));
    let (status, problem) =
        api.post("/api/v1/accounts", json!({"name": "Casa", "kind": "pot"})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("pot"));
}

#[tokio::test]
async fn balances_report_available_money() {
    let api = ApiHarness::new().await;
    api.open_checking("Nubank", 150_000).await;
    api.open_checking("Itaú", 50_000).await;
    let (status, sheet) = api.get("/api/v1/accounts/balances").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (sheet["available_cents"].as_i64(), sheet["reserved_in_pots_cents"].as_i64()),
        (Some(200_000), Some(0))
    );
    assert_eq!(sheet["accounts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn malformed_json_and_unknown_kind_are_bad_requests() {
    let api = ApiHarness::new().await;
    let (status, problem) =
        api.post("/api/v1/accounts", json!({"name": "X", "kind": "crypto"})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{problem}");
    let request = api.request(Method::POST, "/api/v1/accounts").body("{not json".into()).unwrap();
    let (status, problem) = common::send(&api.router, request).await;
    assert_eq!((status, problem["title"].as_str()), (StatusCode::BAD_REQUEST, Some("Bad Request")));
    let (status, _) = api.call(Method::DELETE, "/api/v1/accounts/not-a-uuid", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reconcile_records_the_gap_once() {
    let api = ApiHarness::new().await;
    let checking = api.open_checking("Nubank", 100_000).await;
    let uri = format!("/api/v1/accounts/{checking}/reconcile");
    let (status, entry) = api.post(&uri, json!({"actual_balance_cents": 95_000})).await;
    assert_eq!(status, StatusCode::CREATED, "{entry}");
    assert_eq!(
        (entry["kind"].as_str(), entry["amount_cents"].as_i64()),
        (Some("adjust_out"), Some(5_000))
    );
    let (status, problem) = api.post(&uri, json!({"actual_balance_cents": 95_000})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{problem}");
    let missing = format!("/api/v1/accounts/{}/reconcile", uuid::Uuid::nil());
    assert_eq!(
        api.post(&missing, json!({"actual_balance_cents": 1})).await.0,
        StatusCode::NOT_FOUND
    );
}
