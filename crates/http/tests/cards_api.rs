// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::ApiHarness;
use serde_json::{Value, json};

struct CardScene {
    api: ApiHarness,
    card: String,
    checking: String,
    groceries: String,
}

async fn scene() -> CardScene {
    let api = ApiHarness::new().await;
    let checking = api.open_checking("Nubank conta", 500_000).await;
    let groceries = api.category("mercado", "expense").await;
    let body = json!({"name": "Roxinho", "closing_day": 3, "due_day": 10, "limit_cents": 800_000});
    let (status, card) = api.post("/api/v1/cards", body).await;
    assert_eq!(status, StatusCode::CREATED, "{card}");
    let card = card["id"].as_str().unwrap().to_owned();
    CardScene { api, card, checking, groceries }
}

impl CardScene {
    async fn buy(&self, total: i64, installments: u32) -> (StatusCode, Value) {
        let body = json!({"category_id": self.groceries, "total_cents": total, "installments": installments, "description": "tv"});
        self.api.post(&format!("/api/v1/cards/{}/purchases", self.card), body).await
    }

    async fn invoices(&self) -> Value {
        self.api.get(&format!("/api/v1/cards/{}/invoices", self.card)).await.1
    }
}

#[tokio::test]
async fn purchase_in_installments_shows_on_invoices() {
    let scene = scene().await;
    let (status, purchase) = scene.buy(30_000, 3).await;
    assert_eq!((status, purchase["installment_count"].as_u64()), (StatusCode::CREATED, Some(3)));
    let invoices = scene.invoices().await;
    let charges: Vec<i64> = invoices
        .as_array()
        .unwrap()
        .iter()
        .map(|invoice| invoice["charges_cents"].as_i64().unwrap())
        .collect();
    assert_eq!(charges, vec![10_000, 10_000, 10_000]);
    assert_eq!(
        (invoices[0]["reference_month"].as_str(), invoices[0]["status"].as_str()),
        (Some("2026-04"), Some("open"))
    );
}

#[tokio::test]
async fn credit_and_payment_reduce_outstanding() {
    let scene = scene().await;
    scene.buy(10_000, 1).await;
    let credit = json!({"category_id": scene.groceries, "amount_cents": 2_000});
    assert_eq!(
        scene.api.post(&format!("/api/v1/cards/{}/credits", scene.card), credit).await.0,
        StatusCode::CREATED
    );
    let invoice = scene.invoices().await[0]["id"].as_str().unwrap().to_owned();
    let payment = json!({"account_id": scene.checking, "amount_cents": 3_000});
    let (status, entry) =
        scene.api.post(&format!("/api/v1/invoices/{invoice}/payments"), payment).await;
    assert_eq!((status, entry["kind"].as_str()), (StatusCode::CREATED, Some("invoice_payment")));
    assert_eq!(scene.invoices().await[0]["outstanding_cents"], 5_000);
}

#[tokio::test]
async fn summaries_and_deleting_a_purchase() {
    let scene = scene().await;
    let (_, purchase) = scene.buy(9_000, 3).await;
    let (_, summaries) = scene.api.get("/api/v1/cards/summaries").await;
    assert_eq!(summaries[0]["current_invoice"]["charges_cents"], 3_000);
    assert_eq!(summaries[0]["future_committed_cents"], 6_000);
    let uri = format!("/api/v1/card-purchases/{}", purchase["id"].as_str().unwrap());
    assert_eq!(scene.api.call(Method::DELETE, &uri, None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(scene.api.call(Method::DELETE, &uri, None).await.0, StatusCode::NOT_FOUND);
    assert!(
        scene
            .invoices()
            .await
            .as_array()
            .unwrap()
            .iter()
            .all(|invoice| invoice["charges_cents"] == 0)
    );
}

#[tokio::test]
async fn card_validation_and_archive() {
    let scene = scene().await;
    let (status, problem) = scene
        .api
        .post("/api/v1/cards", json!({"name": "Ruim", "closing_day": 0, "due_day": 10}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(problem["detail"].as_str().unwrap().contains("closing_day"));
    assert_eq!(scene.buy(100, 49).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, cards) = scene.api.get("/api/v1/cards").await;
    assert_eq!(cards[0]["closing_day"], 3);
    let uri = format!("/api/v1/cards/{}", scene.card);
    assert_eq!(scene.api.call(Method::DELETE, &uri, None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(scene.buy(100, 1).await.0, StatusCode::NOT_FOUND);
}
