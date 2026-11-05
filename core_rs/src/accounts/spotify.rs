//! Spotify account binding — thin adapter over `importer::spotify`.
//!
//! The full OAuth (PKCE) + import pipeline lives in `importer::spotify`; this
//! module only translates its status into the `AccountStatus` shape the
//! Plugins page expects, so there is a single source of truth for tokens and
//! the connected account name.

use crate::accounts::{setting, AccountStatus};
use crate::db::storage::DbStorage;
use serde_json::Value;

pub fn status(storage: &DbStorage) -> AccountStatus {
    let connected_row = storage.get_provider_account("spotify").ok().flatten();
    let (connected, user) = match connected_row {
        Some(row) => (row.connected, row.account_name),
        None => (false, String::new()),
    };
    AccountStatus {
        id: "spotify",
        name: "Spotify",
        connected,
        user,
        requires_config: setting(storage, "spotify.clientId").is_none(),
        config_hint: "Spotify Developer Dashboard → create an app → copy the Client ID → paste it in Configure. Redirect URI: http://127.0.0.1:43821/callback".to_string(),
    }
}

pub fn connect(_storage: std::sync::Arc<DbStorage>, _runtime: &std::sync::Arc<tokio::runtime::Runtime>) -> bool {
    crate::importer::spotify::connect()
}

pub fn disconnect(storage: &DbStorage) -> bool {
    let _ = crate::importer::spotify::disconnect();
    true
}

pub fn import(_storage: std::sync::Arc<DbStorage>, _runtime: &std::sync::Arc<tokio::runtime::Runtime>, _options_json: &str) -> bool {
    crate::importer::spotify::preview(r#"{"autoCommit":true,"tracks":true,"albums":true,"playlists":true}"#.to_string())
}