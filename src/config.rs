use serde::Deserialize;

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    /// How many minutes an orders cache file stays valid before re-fetching.
    pub orders_cache_minutes: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            orders_cache_minutes: 5,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let text = match std::fs::read_to_string("wfmq.json") {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("⚠  wfmq.json is invalid ({e}), using defaults.");
            Self::default()
        })
    }
}
