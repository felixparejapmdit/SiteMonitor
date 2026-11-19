//! Thin HTTP client for the Axum API (same origin).

use std::cell::RefCell;

use futures::future::{select, Either};
use gloo_net::http::{Method, RequestBuilder};
use gloo_timers::future::TimeoutFuture;
use serde::de::DeserializeOwned;
use serde::Serialize;
use shared::api::ApiError;

thread_local! {
    static ON_SIGNED_OUT: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Called whenever the server says the session has ended (HTTP 401), so the
/// app can show the sign-in page. Requests ride on the HttpOnly session cookie.
pub fn on_signed_out(f: impl Fn() + 'static) {
    ON_SIGNED_OUT.with(|c| *c.borrow_mut() = Some(Box::new(f)));
}

/// Requests give up after this long and are treated as "no signal".
const TIMEOUT_MS: u32 = 15_000;

#[derive(Debug, Clone, PartialEq)]
pub enum ApiFail {
    /// No connection, or it timed out (eligible for the offline queue).
    Network,
    /// The server answered with an error message.
    Server(u16, String),
}

impl ApiFail {
    pub fn message(&self) -> String {
        match self {
            ApiFail::Network => "No connection right now. Please try again when you have signal.".into(),
            ApiFail::Server(_, m) => m.clone(),
        }
    }
}

/// Sends a request and returns the raw JSON body text.
pub async fn send_raw(method: &str, path: &str, body: Option<&serde_json::Value>) -> Result<String, ApiFail> {
    let method = match method {
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        _ => Method::GET,
    };
    let builder = RequestBuilder::new(&format!("/api{path}")).method(method);
    let request = match body {
        Some(b) => builder.json(b).map_err(|_| ApiFail::Server(0, "Could not prepare the request.".into()))?,
        None => builder.build().map_err(|_| ApiFail::Server(0, "Could not prepare the request.".into()))?,
    };
    let send = Box::pin(request.send());
    // Big map pictures are uploaded and cut into zoom tiles in one request.
    let limit = if path.starts_with("/estate/base-maps") && !path.ends_with("/activate") { 300_000 } else { TIMEOUT_MS };
    let timeout = Box::pin(TimeoutFuture::new(limit));
    let response = match select(send, timeout).await {
        Either::Left((Ok(r), _)) => r,
        Either::Left((Err(_), _)) | Either::Right(_) => return Err(ApiFail::Network),
    };
    let status = response.status();
    let text = response.text().await.map_err(|_| ApiFail::Network)?;
    if response.ok() {
        Ok(text)
    } else if status == 401 && !path.starts_with("/auth/") {
        ON_SIGNED_OUT.with(|c| {
            if let Some(f) = c.borrow().as_ref() {
                f()
            }
        });
        Err(ApiFail::Server(401, "Your session has ended. Please sign in again.".into()))
    } else if status == 502 || status == 503 || status == 504 {
        Err(ApiFail::Network)
    } else {
        let message = serde_json::from_str::<ApiError>(&text)
            .map(|e| e.message)
            .unwrap_or_else(|_| "Something went wrong. Please try again.".into());
        Err(ApiFail::Server(status, message))
    }
}

fn parse<T: DeserializeOwned>(text: &str) -> Result<T, ApiFail> {
    serde_json::from_str(text).map_err(|e| ApiFail::Server(0, format!("Unexpected reply from the server ({e}).")))
}

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, ApiFail> {
    parse(&send_raw("GET", path, None).await?)
}

pub async fn post<B: Serialize, T: DeserializeOwned>(path: &str, body: &B) -> Result<T, ApiFail> {
    let value = serde_json::to_value(body).unwrap_or(serde_json::Value::Null);
    parse(&send_raw("POST", path, Some(&value)).await?)
}

pub async fn put<B: Serialize, T: DeserializeOwned>(path: &str, body: &B) -> Result<T, ApiFail> {
    let value = serde_json::to_value(body).unwrap_or(serde_json::Value::Null);
    parse(&send_raw("PUT", path, Some(&value)).await?)
}
