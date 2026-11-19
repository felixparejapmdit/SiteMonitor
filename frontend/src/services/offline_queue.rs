//! Offline-first IndexedDB mutation queue (`staff_mutation_queue`).
//!
//! When field staff lose signal (deepwell pump rooms, basements, far
//! gardens) their form submissions are serialised here and replayed FIFO
//! as soon as the connection returns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbObjectStoreParameters, IdbOpenDbRequest, IdbRequest, IdbTransactionMode};

const DB_NAME: &str = "sitemonitor";
const DB_VERSION: u32 = 1;
pub const STORE: &str = "staff_mutation_queue";

/// One queued write, replayed exactly as it would have been sent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfflineMutation {
    /// IndexedDB auto-increment key (filled when read back).
    #[serde(default)]
    pub key: f64,
    /// Plain-language description, e.g. "Phase completed: Paving".
    pub label: String,
    pub method: String,
    pub path: String,
    pub body: serde_json::Value,
    /// Replayed as the same user who made the change.
    pub user_id: Uuid,
    pub queued_at: DateTime<Utc>,
}

// Queued writes can hold personal details, so they are encrypted at rest with
// AES-256-GCM (WebCrypto). The key is made on this device, cannot be exported,
// and is kept in its own IndexedDB store. WebCrypto needs HTTPS (or localhost);
// on a plain-HTTP address the queue falls back to storing the text as is.
#[wasm_bindgen(inline_js = r#"
function req(r) { return new Promise((ok, no) => { r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); }); }
async function key() {
  const open = indexedDB.open("sitemonitor-keys", 1);
  open.onupgradeneeded = () => open.result.createObjectStore("k");
  const db = await req(open);
  let k = await req(db.transaction("k").objectStore("k").get("queue"));
  if (!k) {
    k = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
    await req(db.transaction("k", "readwrite").objectStore("k").put(k, "queue"));
  }
  db.close();
  return k;
}
function b64(bytes) { let s = ""; for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000)); return btoa(s); }
export async function seal_text(text) {
  if (!(globalThis.crypto && crypto.subtle)) return text;
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, await key(), new TextEncoder().encode(text)));
  const all = new Uint8Array(12 + ct.length);
  all.set(iv);
  all.set(ct, 12);
  return "smq1:" + b64(all);
}
export async function open_text(s) {
  if (!s.startsWith("smq1:")) return s;
  const raw = Uint8Array.from(atob(s.slice(5)), (c) => c.charCodeAt(0));
  const pt = await crypto.subtle.decrypt({ name: "AES-GCM", iv: raw.subarray(0, 12) }, await key(), raw.subarray(12));
  return new TextDecoder().decode(pt);
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn seal_text(text: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn open_text(sealed: &str) -> Result<JsValue, JsValue>;
}

fn js_err(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

/// Resolves when an IndexedDB request succeeds (or fails).
fn request_done(req: &IdbRequest) -> JsFuture {
    let req = req.clone();
    let promise = js_sys::Promise::new(&mut |resolve: js_sys::Function, reject: js_sys::Function| {
        let ok_req = req.clone();
        let on_success = Closure::once_into_js(move |_e: web_sys::Event| {
            let value = ok_req.result().unwrap_or(JsValue::UNDEFINED);
            let _ = resolve.call1(&JsValue::NULL, &value);
        });
        let on_error = Closure::once_into_js(move |_e: web_sys::Event| {
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("IndexedDB request failed"));
        });
        req.set_onsuccess(Some(on_success.unchecked_ref()));
        req.set_onerror(Some(on_error.unchecked_ref()));
    });
    JsFuture::from(promise)
}

async fn open() -> Result<IdbDatabase, String> {
    let factory = web_sys::window()
        .ok_or("no window")?
        .indexed_db()
        .map_err(js_err)?
        .ok_or("IndexedDB is not available in this browser")?;
    let open_req: IdbOpenDbRequest = factory.open_with_u32(DB_NAME, DB_VERSION).map_err(js_err)?;
    let upgrade_req = open_req.clone();
    let on_upgrade = Closure::once_into_js(move |_e: web_sys::Event| {
        if let Ok(result) = upgrade_req.result() {
            if let Ok(db) = result.dyn_into::<IdbDatabase>() {
                let params = IdbObjectStoreParameters::new();
                params.set_auto_increment(true);
                let _ = db.create_object_store_with_optional_parameters(STORE, &params);
            }
        }
    });
    open_req.set_onupgradeneeded(Some(on_upgrade.unchecked_ref()));
    let db = request_done(&open_req).await.map_err(js_err)?;
    db.dyn_into::<IdbDatabase>().map_err(|_| "could not open the offline queue".to_string())
}

/// Adds a mutation to the end of the queue.
pub async fn enqueue(m: &OfflineMutation) -> Result<(), String> {
    // Encrypt before opening the transaction: an IndexedDB transaction closes while we await.
    let json = serde_json::to_string(m).map_err(|e| e.to_string())?;
    let sealed = seal_text(&json).await.map_err(js_err)?;
    let db = open().await?;
    let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite).map_err(js_err)?;
    let store = tx.object_store(STORE).map_err(js_err)?;
    let req = store.add(&sealed).map_err(js_err)?;
    request_done(&req).await.map_err(js_err)?;
    db.close();
    Ok(())
}

/// Every queued mutation, oldest first (FIFO).
pub async fn list() -> Result<Vec<OfflineMutation>, String> {
    let db = open().await?;
    let tx = db.transaction_with_str(STORE).map_err(js_err)?;
    let store = tx.object_store(STORE).map_err(js_err)?;
    let keys = request_done(&store.get_all_keys().map_err(js_err)?).await.map_err(js_err)?;
    let values = request_done(&store.get_all().map_err(js_err)?).await.map_err(js_err)?;
    db.close();
    let keys = js_sys::Array::from(&keys);
    let values = js_sys::Array::from(&values);
    let mut out = Vec::new();
    for i in 0..values.length() {
        let Some(stored) = values.get(i).as_string() else { continue };
        // Unreadable entries (e.g. the device key was cleared) are skipped, not fatal.
        let Some(text) = open_text(&stored).await.ok().and_then(|v| v.as_string()) else { continue };
        if let Ok(mut m) = serde_json::from_str::<OfflineMutation>(&text) {
            m.key = keys.get(i).as_f64().unwrap_or_default();
            out.push(m);
        }
    }
    out.sort_by(|a, b| a.key.partial_cmp(&b.key).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

/// Removes one mutation after it was delivered (or permanently rejected).
pub async fn remove(key: f64) -> Result<(), String> {
    let db = open().await?;
    let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite).map_err(js_err)?;
    let store = tx.object_store(STORE).map_err(js_err)?;
    let req = store.delete(&JsValue::from_f64(key)).map_err(js_err)?;
    request_done(&req).await.map_err(js_err)?;
    db.close();
    Ok(())
}
