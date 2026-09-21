// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::MemberProfile;
use app::ports::MemberStore;
use axum::http::{Method, StatusCode, header};
use common::ApiHarness;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

/// Status, content type, disposition and body of a CSV download.
async fn download(api: &ApiHarness, uri: &str) -> (StatusCode, String, String, String) {
    let request = api.request(Method::GET, uri).body(axum::body::Body::empty()).unwrap();
    let response = api.router.clone().oneshot(request).await.unwrap();
    let header_text = |name| {
        response
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default()
    };
    let (content_type, disposition) =
        (header_text(header::CONTENT_TYPE), header_text(header::CONTENT_DISPOSITION));
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, content_type, disposition, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn record_expense(api: &ApiHarness, cents: i64, date: &str) {
    let checking = api.open_checking(&format!("Conta {date}"), 0).await;
    let groceries = api.category(&format!("mercado {date}"), "expense").await;
    let body = json!({"kind": "expense", "account_id": checking, "category_id": groceries,
        "amount_cents": cents, "description": "feira", "date": date});
    assert_eq!(api.post("/api/v1/entries", body).await.0, StatusCode::CREATED);
}

#[tokio::test]
async fn csv_export_of_the_current_cycle() {
    let api = ApiHarness::new().await;
    record_expense(&api, 1050, "2026-03-05").await;
    record_expense(&api, 999, "2026-02-27").await;
    let (status, content_type, disposition, csv) =
        download(&api, "/api/v1/exports/entries.csv").await;
    assert_eq!(status, StatusCode::OK, "{csv}");
    assert_eq!(content_type, "text/csv; charset=utf-8");
    assert_eq!(disposition, "attachment; filename=\"finbot-2026-03-01_2026-03-31.csv\"");
    assert!(csv.starts_with('\u{feff}'), "BOM keeps accents readable in spreadsheets");
    assert!(
        csv.contains(
            "05/03/2026;Gasto;feira;mercado 2026-03-05;Conta 2026-03-05;;;;10,50;automático"
        ),
        "{csv}"
    );
    assert!(!csv.contains("27/02/2026"), "{csv}");
}

#[tokio::test]
async fn csv_export_of_an_explicit_range() {
    let api = ApiHarness::new().await;
    record_expense(&api, 999, "2026-02-27").await;
    let uri = "/api/v1/exports/entries.csv?from=2026-02-01&to=2026-02-28";
    let (status, _, disposition, csv) = download(&api, uri).await;
    assert_eq!(status, StatusCode::OK);
    assert!(disposition.contains("finbot-2026-02-01_2026-02-28.csv"));
    assert!(csv.contains("27/02/2026"), "{csv}");
}

#[tokio::test]
async fn csv_export_rejects_half_or_inverted_ranges() {
    let api = ApiHarness::new().await;
    let (status, problem) = api.get("/api/v1/exports/entries.csv?from=2026-02-01").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("both from and to"), "{problem}");
    let (status, problem) =
        api.get("/api/v1/exports/entries.csv?from=2026-03-01&to=2026-02-01").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("2026-02-01"), "{problem}");
}

#[tokio::test]
async fn members_list_shows_who_gets_backups() {
    let api = ApiHarness::new().await;
    let (status, members) = api.get("/api/v1/members").await;
    assert_eq!((status, members), (StatusCode::OK, json!([])));
    let profile = MemberProfile { telegram_user_id: 11, display_name: "Ana".into() };
    let ana = api.set.store.upsert_member(profile).await.unwrap().id;
    api.set.services.members.register_dm(ana, 11).await.unwrap();
    let (_, members) = api.get("/api/v1/members").await;
    assert_eq!(members[0]["display_name"], "Ana");
    assert_eq!(members[0]["has_private_chat"], true);
}
