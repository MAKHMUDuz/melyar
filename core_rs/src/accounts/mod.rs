//! Account providers: linking external music services to Melyar.
//!
//! Each provider can be connected (OAuth / API key), disconnected, and asked
//! to import its library into Melyar's local SQLite store. The UI talks to
//! them through the bridge via `list_accounts_json`, `connect_account`,
//! `disconnect_account` and `import_account`. State changes are emitted as
//! JSON events on the same channel the import centre uses, so the interface
//! can show progress without polling.

pub mod apple;
pub mod custom;
pub mod google;
pub mod spotify;
pub mod yandex;

use crate::db::storage::DbStorage;
use crate::ffi;
use serde_json::{json, Value};
use std::sync::Arc;

/// Emit a JSON event to the UI.
fn emit(value: Value) {
    ffi::on_import_event(value.to_string());
}

/// One account provider's status, as rendered on the Plugins page.
#[derive(Clone)]
pub struct AccountStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub connected: bool,
    pub user: String,
    pub requires_config: bool,
    pub config_hint: String,
}

impl AccountStatus {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "connected": self.connected,
            "user": self.user,
            "requiresConfig": self.requires_config,
            "configHint": self.config_hint,
        })
    }
}

/// Build the status list for every provider.
pub fn list_accounts_json() -> String {
    let core = crate::get_core();
    let storage = core.storage();
    let mut statuses = Vec::new();
    statuses.push(spotify::status(&storage));
    statuses.push(google::status(&storage));
    statuses.push(yandex::status(&storage));
    statuses.push(apple::status(&storage));
    statuses.push(custom::status(&storage));
    let arr: Vec<Value> = statuses.iter().map(|s| s.to_json()).collect();
    serde_json::Value::Array(arr).to_string()
}

/// Start the connect flow for one provider. Returns false for an unknown id.
pub fn connect_account(id: &str) -> bool {
    let core = crate::get_core();
    let storage = core.storage();
    let runtime = core.runtime();
    match id {
        "spotify" => spotify::connect(storage.clone(), &runtime),
        "google" => google::connect(storage.clone(), &runtime),
        "yandex" => yandex::connect(storage.clone(), &runtime),
        "apple" => apple::connect(storage.clone(), &runtime),
        "custom" => custom::connect(storage.clone(), &runtime),
        _ => {
            emit(
                json!({"type":"error","code":"unknown_provider","message":format!("Unknown provider {id}")}),
            );
            false
        }
    }
}

/// Disconnect one provider. Returns false for an unknown id.
pub fn disconnect_account(id: &str) -> bool {
    let core = crate::get_core();
    let storage = core.storage();
    match id {
        "spotify" => spotify::disconnect(&storage),
        "google" => google::disconnect(&storage),
        "yandex" => yandex::disconnect(&storage),
        "apple" => apple::disconnect(&storage),
        "custom" => custom::disconnect(&storage),
        _ => false,
    }
}

/// Import one provider's library. Returns false for an unknown id or a
/// provider that is not connected.
pub fn import_account(id: &str, options_json: &str) -> bool {
    let core = crate::get_core();
    let storage = core.storage();
    let runtime = core.runtime();
    let innertube = core.innertube();
    match id {
        "spotify" => spotify::import(storage.clone(), &runtime, options_json),
        "google" => google::import(storage.clone(), &runtime, &innertube, options_json),
        "yandex" => yandex::import(storage.clone(), &runtime, options_json),
        "apple" => apple::import(storage.clone(), &runtime, options_json),
        "custom" => custom::import(storage.clone(), &runtime, options_json),
        _ => false,
    }
}

/// Helper to read a setting as a trimmed non-empty string.
pub fn setting(storage: &DbStorage, key: &str) -> Option<String> {
    storage
        .get_setting(key)
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Helper to store a setting.
pub fn set_setting(storage: &DbStorage, key: &str, value: &str) {
    let _ = storage.set_setting(key, value);
}
