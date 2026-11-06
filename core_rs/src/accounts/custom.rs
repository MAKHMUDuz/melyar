//! Custom music API binding (Subsonic-compatible + generic REST).
//!
//! Supports Subsonic API (Navidrome, Airsonic, Gonic, ...) out of the box,
//! plus a generic REST mode where the user supplies a base URL and an API key.

use crate::accounts::{emit, set_setting, setting};
use crate::db::storage::DbStorage;
use serde_json::json;

pub fn status(storage: &DbStorage) -> crate::accounts::AccountStatus {
    let connected = setting(storage, "custom.baseUrl").is_some();
    crate::accounts::AccountStatus {
        id: "custom",
        name: "Custom API",
        connected,
        user: setting(storage, "custom.baseUrl").unwrap_or_default(),
        requires_config: !connected,
        config_hint: "Enter your server base URL (e.g. http://localhost:4533) and credentials. Subsonic API is supported.".to_string(),
    }
}

pub fn connect(
    storage: std::sync::Arc<DbStorage>,
    _runtime: &std::sync::Arc<tokio::runtime::Runtime>,
) -> bool {
    // Custom API is configured via settings (baseUrl, username, password/token),
    // not via OAuth. Mark as connected when baseUrl is present.
    if setting(&storage, "custom.baseUrl").is_none() {
        emit(
            json!({"type":"error","code":"requires_configuration","message":"Set custom.baseUrl in settings first."}),
        );
        return false;
    }
    emit(json!({"type":"provider_connected","provider":"custom"}));
    true
}

pub fn disconnect(storage: &DbStorage) -> bool {
    let _ = storage.delete_setting("custom.baseUrl");
    emit(json!({"type":"provider_disconnected","provider":"custom"}));
    true
}

pub fn import(
    storage: std::sync::Arc<DbStorage>,
    _runtime: &std::sync::Arc<tokio::runtime::Runtime>,
    _options_json: &str,
) -> bool {
    emit(
        json!({"type":"error","code":"not_implemented","message":"Custom API import is not implemented yet."}),
    );
    false
}
