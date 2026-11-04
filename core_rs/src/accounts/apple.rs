//! Apple Music account binding.
//!
//! Requires an Apple Developer Program membership ($99/year) to generate
//! the MusicKit developer token (JWT). Without it the API cannot be reached,
//! so this module is a stub.

use crate::accounts::emit;
use crate::db::storage::DbStorage;
use serde_json::json;

pub fn status(storage: &DbStorage) -> crate::accounts::AccountStatus {
    crate::accounts::AccountStatus {
        id: "apple",
        name: "Apple Music",
        connected: false,
        user: String::new(),
        requires_config: true,
        config_hint: "Requires an Apple Developer Program membership ($99/year) to mint a MusicKit developer token. Not available in this build.".to_string(),
    }
}

pub fn connect(_storage: std::sync::Arc<DbStorage>, _runtime: &std::sync::Arc<tokio::runtime::Runtime>) -> bool {
    emit(json!({"type":"error","code":"requires_developer_membership","message":"Apple Music requires an Apple Developer Program membership."}));
    false
}

pub fn disconnect(_storage: &DbStorage) -> bool {
    true
}

pub fn import(_storage: std::sync::Arc<DbStorage>, _runtime: &std::sync::Arc<tokio::runtime::Runtime>, _options_json: &str) -> bool {
    emit(json!({"type":"error","code":"not_implemented","message":"Apple Music import is not implemented."}));
    false
}
