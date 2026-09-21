// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::ApiHarness;
use serde_json::{Value, json};

struct Ledger {
    api: ApiHarness,
    checking: String,
    savings: String,
    groceries: String,
    salary: String,
}

async fn ledger() -> Ledger {
    let api = ApiHarness::new().await;
    let checking = api.open_checking("Nubank", 100_000).await;
    let savings = api.open_checking("Poupança", 0).await;
    let groceries = api.category("mercado", "expense").await;
    let salary = api.category("salário", "income").await;
    Ledger { api, checking, savings, groceries, salary }
}

impl Ledger {
    fn expense(&self, cents: i64) -> Value {
        json!({"kind": "expense", "account_id": self.checking, "category_id": self.groceries, "amount_cents": cents, "description": "feira"})
    }

    async fn create(&self, body: Value) -> (StatusCode, Value) {
        self.api.post("/api/v1/entries", body).await
    }
}

#[tokio::test]
async fn creates_each_entry_kind() {
    let ledger = ledger().await;
    let income = json!({"kind": "income", "account_id": ledger.checking, "category_id": ledger.salary, "amount_cents": 500_000});
    let transfer = json!({"kind": "transfer", "from_account_id": ledger.checking, "to_account_id": ledger.savings, "amount_cents": 1000});
    let adjust = json!({"kind": "adjust_out", "account_id": ledger.checking, "amount_cents": 3, "date": "2026-03-01"});
    for body in [ledger.expense(1050), income, transfer, adjust] {
        let (status, entry) = ledger.create(body).await;
        assert_eq!(status, StatusCode::CREATED, "{entry}");
    }
    let (_, sheet) = ledger.api.get("/api/v1/accounts/balances").await;
    assert_eq!(sheet["accounts"][0]["balance_cents"], 100_000 - 1050 + 500_000 - 1000 - 3);
}

#[tokio::test]
async fn idempotency_key_saves_once() {
    let ledger = ledger().await;
    let key = "0199a000-0000-7000-8000-000000000001";
    let request = |body: Value| {
        ledger
            .api
            .request(Method::POST, "/api/v1/entries")
            .header("Idempotency-Key", key)
            .body(body.to_string().into())
            .unwrap()
    };
    let (first, _) = common::send(&ledger.api.router, request(ledger.expense(10))).await;
    let (second, problem) = common::send(&ledger.api.router, request(ledger.expense(10))).await;
    assert_eq!((first, second), (StatusCode::CREATED, StatusCode::CONFLICT));
    assert!(problem["detail"].as_str().unwrap().contains("already saved"));
    let bad = ledger.api.request(Method::POST, "/api/v1/entries").header("Idempotency-Key", "abc");
    let (status, _) =
        common::send(&ledger.api.router, bad.body(ledger.expense(10).to_string().into()).unwrap())
            .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_patch_delete_entry() {
    let ledger = ledger().await;
    let (_, entry) = ledger.create(ledger.expense(500)).await;
    let uri = format!("/api/v1/entries/{}", entry["id"].as_str().unwrap());
    let (status, found) = ledger.api.get(&uri).await;
    assert_eq!((status, found["amount_cents"].as_i64()), (StatusCode::OK, Some(500)));
    let (status, patched) =
        ledger.api.call(Method::PATCH, &uri, Some(json!({"amount_cents": 700}))).await;
    assert_eq!((status, patched["amount_cents"].as_i64()), (StatusCode::OK, Some(700)));
    let (status, _) = ledger.api.call(Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, problem) = ledger.api.get(&uri).await;
    assert_eq!((status, problem["status"].as_u64()), (StatusCode::NOT_FOUND, Some(404)));
}

#[tokio::test]
async fn list_filters_by_kind_and_dates() {
    let ledger = ledger().await;
    ledger.create(ledger.expense(1)).await;
    let old = json!({"kind": "adjust_in", "account_id": ledger.checking, "amount_cents": 2, "date": "2026-01-15"});
    ledger.create(old).await;
    let (_, expenses) = ledger.api.get("/api/v1/entries?kind=expense").await;
    assert_eq!(expenses.as_array().unwrap().len(), 1);
    let (_, january) = ledger.api.get("/api/v1/entries?from=2026-01-01&to=2026-01-31").await;
    assert_eq!(january[0]["kind"], "adjust_in");
    let (_, limited) =
        ledger.api.get(&format!("/api/v1/entries?account_id={}&limit=1", ledger.checking)).await;
    assert_eq!(limited.as_array().unwrap().len(), 1);
    let (status, _) = ledger.api.get("/api/v1/entries?kind=bogus").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn validation_errors_are_unprocessable() {
    let ledger = ledger().await;
    let (status, problem) = ledger.create(ledger.expense(-5)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("-5"), "{problem}");
    let wrong_category = json!({"kind": "refund", "account_id": ledger.checking, "category_id": ledger.salary, "amount_cents": 5});
    let (status, _) = ledger.create(wrong_category).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
