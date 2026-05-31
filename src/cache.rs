//! Thin JSON cache on top of the local filesystem.
//!
//! * Static data  (items, locations, missions, dropsources) — no TTL, simply
//!   check whether the file exists.
//! * Dynamic data (orders) — TTL-based; check file mtime against a configured
//!   number of minutes.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

pub fn items_path()            -> PathBuf { PathBuf::from("data/items.json") }
pub fn locations_path()        -> PathBuf { PathBuf::from("data/locations.json") }
pub fn missions_path()         -> PathBuf { PathBuf::from("data/missions.json") }
pub fn ducats_per_relic_path() -> PathBuf { PathBuf::from("data/ducats_per_relic.json") }

pub fn dropsources_path(slug: &str) -> PathBuf {
    PathBuf::from(format!("data/dropsources/{slug}.json"))
}

pub fn orders_path(slug: &str) -> PathBuf {
    PathBuf::from(format!("data/orders/{slug}.json"))
}

// ---------------------------------------------------------------------------
// Core operations
// ---------------------------------------------------------------------------

/// Deserialise a cached JSON file. Returns `None` if the file is absent or
/// malformed (caller decides whether that warrants a warning).
pub fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("  ⚠ cache parse error in {}: {e}", path.display());
            None
        }
    }
}

/// Serialise `data` to a pretty-printed JSON file, creating parent directories
/// as needed. Errors are non-fatal — the caller should log them.
pub fn write<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(data)?;
    fs::write(path, text)?;
    Ok(())
}

/// Returns `true` if the file exists AND its last-modified time is within
/// `max_age_minutes` of now.
pub fn is_fresh(path: &Path, max_age_minutes: u64) -> bool {
    let Ok(meta) = fs::metadata(path) else { return false };
    let Ok(modified) = meta.modified() else { return false };
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else { return false };
    elapsed.as_secs() < max_age_minutes * 60
}
