// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::ApiHarness;
use serde_json::json;

#[tokio::test]
async fn categories_carry_the_essential_mark() {
    let api = ApiHarness::new().await;
    let body = json!({"name": "aluguel", "kind": "expense", "essential": true});
    let (status, rent) = api.post("/api/v1/categories", body).await;
    assert_eq!((status, rent["essential"].as_bool()), (StatusCode::CREATED, Some(true)));
    let uri = format!("/api/v1/categories/{}", rent["id"].as_str().unwrap());
    let (status, cleared) = api.call(Method::PATCH, &uri, Some(json!({"essential": false}))).await;
    assert_eq!((status, cleared["essential"].as_bool()), (StatusCode::OK, Some(false)));
    let income = json!({"name": "bônus", "kind": "income", "essential": true});
    assert_eq!(api.post("/api/v1/categories", income).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn living_cost_report_sums_essential_spending() {
    let api = ApiHarness::new().await;
    let checking = api.open_checking("Nubank", 0).await;
    let body = json!({"name": "mercado", "kind": "expense", "essential": true});
    let market = api.post("/api/v1/categories", body).await.1["id"].as_str().unwrap().to_owned();
    let expense = json!({"kind": "expense", "account_id": checking, "category_id": market,
        "amount_cents": 12_345, "description": "feira"});
    assert_eq!(api.post("/api/v1/entries", expense).await.0, StatusCode::CREATED);
    let (status, report) = api.get("/api/v1/reports/living-cost").await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["projected_cents"].as_i64(), Some(12_345));
    assert_eq!(report["cost"]["by_category"][0][1].as_i64(), Some(12_345), "{report}");
    assert_eq!(report["reserve_target_cents"].as_i64(), Some(74_070));
}
