// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The real client adapter against a local fake Bot API server.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use serde_json::{Value, json};
use telegram::gateway::{
    Button, FrankensteinGateway, GatewayError, Keyboard, MessageEdit, OutgoingDocument,
    OutgoingMessage, TelegramGateway, UpdateKind,
};

/// Records requests and answers each method with a canned response.
#[derive(Clone, Default)]
struct FakeBotApi {
    requests: Arc<Mutex<Vec<(String, Value)>>>,
}

/// JSON bodies are parsed; multipart uploads are kept as text so tests
/// can look for the file inside.
async fn answer(
    State(api): State<FakeBotApi>,
    Path(method): Path<String>,
    body: axum::body::Bytes,
) -> (StatusCode, Json<Value>) {
    let body = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    api.requests.lock().unwrap().push((method.clone(), body.clone()));
    canned(&method, &body)
}

fn canned(method: &str, body: &Value) -> (StatusCode, Json<Value>) {
    let message = json!({"message_id": 77, "date": 0, "chat": {"id": -1, "type": "group", "title": "Casa"}, "text": "ok"});
    match method {
        "getUpdates" if body["offset"] == 429 => {
            error(429, "Too Many Requests: retry after 9", &json!({"retry_after": 9}))
        }
        "getUpdates" if body["offset"] == 409 => {
            error(409, "Conflict: terminated by other getUpdates", &Value::Null)
        }
        "getUpdates" => ok(&json!([{"update_id": 3, "message": {"message_id": 1, "date": 0,
            "chat": {"id": -1, "type": "group", "title": "Casa"}, "from": {"id": 11, "is_bot": false, "first_name": "Ana"}, "text": "/saldo"}}])),
        "sendMessage" | "sendDocument" => ok(&message),
        "editMessageText" if body["message_id"] == 404 => {
            error(400, "Bad Request: message to edit not found", &Value::Null)
        }
        "editMessageText" => error(400, "Bad Request: message is not modified", &Value::Null),
        _ => ok(&json!(true)),
    }
}

fn ok(result: &Value) -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({"ok": true, "result": result})))
}

fn error(code: u16, description: &str, parameters: &Value) -> (StatusCode, Json<Value>) {
    let body = json!({"ok": false, "error_code": code, "description": description, "parameters": parameters});
    (StatusCode::from_u16(code).unwrap(), Json(body))
}

async fn start_server() -> (FakeBotApi, FrankensteinGateway) {
    let api = FakeBotApi::default();
    let router =
        axum::Router::new().route("/botTOKEN/{method}", post(answer)).with_state(api.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (api, FrankensteinGateway::with_api_url(&format!("http://{address}/botTOKEN")))
}

fn last_request(api: &FakeBotApi) -> (String, Value) {
    api.requests.lock().unwrap().last().cloned().unwrap()
}

#[tokio::test]
async fn polls_and_maps_updates() {
    let (api, gateway) = start_server().await;
    let updates = gateway.get_updates(Some(3), Duration::from_secs(1)).await.unwrap();
    assert!(matches!(&updates[0].kind, UpdateKind::Text(message) if message.text == "/saldo"));
    let (method, body) = last_request(&api);
    assert_eq!((method.as_str(), body["timeout"].as_u64()), ("getUpdates", Some(1)));
    assert_eq!(body["allowed_updates"], json!(["message", "callback_query", "my_chat_member"]));
}

#[tokio::test]
async fn maps_rate_limit_and_conflict() {
    let (_, gateway) = start_server().await;
    let limited = gateway.get_updates(Some(429), Duration::from_secs(1)).await.unwrap_err();
    assert_eq!(limited, GatewayError::RateLimited { retry_after: Duration::from_secs(9) });
    let conflict = gateway.get_updates(Some(409), Duration::from_secs(1)).await.unwrap_err();
    assert_eq!(conflict, GatewayError::Conflict);
}

#[tokio::test]
async fn sends_html_with_inline_keyboard() {
    let (api, gateway) = start_server().await;
    let keyboard =
        Keyboard { rows: vec![vec![Button { label: "Hoje".into(), data: "n|dt".into() }]] };
    let message =
        OutgoingMessage { chat_id: -1, html: "<b>Oi</b>".into(), keyboard: Some(keyboard) };
    assert_eq!(gateway.send_message(&message).await.unwrap(), 77);
    let (_, body) = last_request(&api);
    assert_eq!(
        (body["parse_mode"].as_str(), body["text"].as_str()),
        (Some("HTML"), Some("<b>Oi</b>"))
    );
    assert_eq!(body["reply_markup"]["inline_keyboard"][0][0]["callback_data"], "n|dt");
}

#[tokio::test]
async fn edits_ignore_not_modified_but_report_other_errors() {
    let (_, gateway) = start_server().await;
    let edit = MessageEdit { chat_id: -1, message_id: 5, html: "x".into(), keyboard: None };
    gateway.edit_message(&edit).await.unwrap();
    let missing = gateway.edit_message(&MessageEdit { message_id: 404, ..edit }).await.unwrap_err();
    assert!(matches!(missing, GatewayError::Api { code: 400, .. }));
}

#[tokio::test]
async fn other_methods_reach_the_api() {
    let (api, gateway) = start_server().await;
    gateway.delete_webhook().await.unwrap();
    gateway.set_commands(&[("gasto", "Registrar um gasto")]).await.unwrap();
    gateway.remove_keyboard(-1, 5).await.unwrap();
    gateway.answer_button("cb1", Some("feito")).await.unwrap();
    gateway.leave_chat(-9).await.unwrap();
    let methods: Vec<String> =
        api.requests.lock().unwrap().iter().map(|(method, _)| method.clone()).collect();
    assert_eq!(
        methods,
        vec![
            "deleteWebhook",
            "setMyCommands",
            "editMessageReplyMarkup",
            "answerCallbackQuery",
            "leaveChat"
        ]
    );
}

#[tokio::test]
async fn documents_are_uploaded_as_multipart_and_unstaged() {
    let (api, gateway) = start_server().await;
    let document = OutgoingDocument {
        chat_id: -1,
        file_name: "finbot-2026-03.csv".into(),
        contents: b"data;valor\n05/03/2026;10,50\n".to_vec(),
        caption_html: "<b>Março</b>".into(),
    };
    gateway.send_document(&document).await.unwrap();
    let requests = api.requests.lock().unwrap().clone();
    let (method, body) = requests.last().unwrap();
    let upload = body.as_str().unwrap();
    assert_eq!(method, "sendDocument");
    assert!(
        upload.contains("finbot-2026-03.csv") && upload.contains("05/03/2026;10,50"),
        "{upload}"
    );
    let leftovers = std::fs::read_dir(std::env::temp_dir()).unwrap().filter_map(Result::ok);
    let staged = leftovers.filter(|dir| dir.path().join("finbot-2026-03.csv").exists()).count();
    assert_eq!(staged, 0, "the staged file must be removed after upload");
}

#[tokio::test]
async fn unreachable_server_is_transport_error() {
    let gateway = FrankensteinGateway::with_api_url("http://127.0.0.1:1/botTOKEN");
    let error = gateway.leave_chat(1).await.unwrap_err();
    assert!(matches!(error, GatewayError::Transport(_)), "{error:?}");
}

#[tokio::test]
async fn debug_output_never_shows_the_token() {
    let gateway = FrankensteinGateway::new("123:SECRET");
    assert!(!format!("{gateway:?}").contains("SECRET"));
}
