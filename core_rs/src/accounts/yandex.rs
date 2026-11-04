//! Yandex Music account binding (experimental).
//!
//! Yandex shut down official third-party API access in 2023. Community
//! libraries still work but are unofficial and fragile. This module is a
//! stub that reports the requirement; full implementation is pending.

use crate::accounts::{emit, set_setting, setting};
use crate::db::storage::DbStorage;
use serde_json::json;

pub fn status(storage: &DbStorage) -> crate::accounts::AccountStatus {
    crate::accounts::AccountStatus {
        id: "yandex",
        name: "Yandex Music",
        connected: false,
        user: String::new(),
        requires_config: true,
        config_hint: "Experimental: Yandex shut down official third-party API access. Implementation pending; contributions welcome.".to_string(),
    }
}

pub fn connect(_storage: std::sync::Arc<DbStorage>, _runtime: &std::sync::Arc<tokio::runtime::Runtime>) -> bool {
    emit(json!({"type":"error","code":"not_implemented","message":"Yandex Music binding is not implemented yet."}));
    false
}

pub fn disconnect(_storage: &DbStorage) -> bool {
    true
}

pub fn import(_storage: std::sync::Arc<DbStorage>, _runtime: &std::sync::Arc<tokio::runtime::Runtime>, _options_json: &str) -> bool {
    emit(json!({"type":"error","code":"not_implemented","message":"Yandex Music import is not implemented yet."}));
    false
}
