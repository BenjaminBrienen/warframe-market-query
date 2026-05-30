use serde::Deserialize;

// ---------------------------------------------------------------------------
// Generic envelope
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

/// Minimal item record returned by GET /v2/items.
/// Fields use `default` so they deserialise to None / empty if absent.
#[derive(Deserialize, Debug, Clone)]
pub struct ItemShort {
    pub id: String,
    pub slug: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Ducat trade-in value. Only present on prime parts.
    #[serde(default)]
    pub ducats: Option<u32>,
    pub i18n: Option<I18nMap>,
}

impl ItemShort {
    /// Human-readable English name, falling back to the slug.
    pub fn name(&self) -> &str {
        self.i18n
            .as_ref()
            .map(|i| i.en.name.as_str())
            .unwrap_or(&self.slug)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct I18nMap {
    pub en: I18nEntry,
}

#[derive(Deserialize, Debug, Clone)]
pub struct I18nEntry {
    pub name: String,
}

// ---------------------------------------------------------------------------
// Orders
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderWithUser {
    pub id: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub platinum: u32,
    pub quantity: u32,
    pub user: UserShort,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserShort {
    pub ingame_name: String,
    pub status: String,
    pub reputation: i32,
}
