use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A single entry in the shared key-history list. Mirrors the Pinia
/// `keyHistory` store: newest first, no duplicate names. Owned by this crate
/// (rather than `app`) since it's persisted data shared across every feature
/// that reads/writes the key history (key generator, encrypter, decrypter).
#[derive(Debug, Clone)]
pub struct KeyEntry {
    pub name: String,
    pub bits: usize,
}

/// A key/IV persisted as part of a session's history. Mirrors `KeyEntry`
/// but derives `Serialize`/`Deserialize` on its own — `KeyEntry` itself stays
/// plain since it's also embedded in view state that has no business knowing
/// about serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredKeyEntry {
    pub name: String,
    pub bits: usize,
}

impl From<&KeyEntry> for StoredKeyEntry {
    fn from(entry: &KeyEntry) -> Self {
        Self {
            name: entry.name.clone(),
            bits: entry.bits,
        }
    }
}

impl From<StoredKeyEntry> for KeyEntry {
    fn from(entry: StoredKeyEntry) -> Self {
        Self {
            name: entry.name,
            bits: entry.bits,
        }
    }
}

/// A saved session: a name, a creation timestamp, and the key/IV history the
/// user had built up in it. This is the only piece of app state a session
/// persists — per-screen form fields are transient UI state, not something a
/// session needs to restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at_unix: u64,
    #[serde(default)]
    pub key_history: Vec<StoredKeyEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SessionStore {
    #[serde(default)]
    sessions: Vec<Session>,
}

fn store_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("gpui-crittoutil").join("sessions.json"))
}

/// Load all saved sessions, newest first. Returns an empty list if nothing has
/// been saved yet, or if the store can't be read (never treat this as fatal —
/// worst case the user just sees no recent sessions).
pub fn load_all() -> Vec<Session> {
    let Some(path) = store_path() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut store: SessionStore = serde_json::from_str(&contents).unwrap_or_default();
    store.sessions.sort_by(|a, b| b.created_at_unix.cmp(&a.created_at_unix));
    store.sessions
}

/// Persist the full session list, overwriting whatever was there. Best-effort:
/// a write failure (e.g. no disk access) is silently ignored rather than
/// surfaced as an app error, since losing session history isn't fatal.
pub fn save_all(sessions: &[Session]) {
    let Some(path) = store_path() else { return };
    let Some(parent) = path.parent() else { return };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let store = SessionStore {
        sessions: sessions.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(&path, json);
    }
}

/// Format a unix timestamp as `YYYY-MM-DD HH:MM` (UTC), dependency-free via
/// Howard Hinnant's civil-calendar algorithm.
pub fn format_created_at(unix: u64) -> String {
    let days = (unix / 86400) as i64;
    let secs_of_day = unix % 86400;
    let (y, m, d) = civil_from_days(days);
    let h = secs_of_day / 3600;
    let min = (secs_of_day % 3600) / 60;
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// A new, empty session with a unique id and a "Session <n>" default name.
pub fn new_session(existing_count: usize) -> Session {
    let created_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Session {
        id: format!("{created_at_unix}-{}", rand::random::<u32>()),
        name: format!("Session {}", existing_count + 1),
        created_at_unix,
        key_history: Vec::new(),
    }
}
