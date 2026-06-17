use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Generic API envelopes
// ---------------------------------------------------------------------------

/// Standard v2 response wrapper.
#[derive(Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

/// v1 endpoints wrap results in `payload` instead.
#[derive(Deserialize)]
pub struct V1DropSourcesResponse {
    pub payload: V1DropSourcesPayload,
}

#[derive(Deserialize)]
pub struct V1DropSourcesPayload {
    pub dropsources: Vec<DropSource>,
}

// ---------------------------------------------------------------------------
// Items  (GET /v2/items)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ItemShort {
    pub id: String,
    pub slug: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Ducat trade-in value — only present on prime parts.
    #[serde(default)]
    pub ducats: Option<u32>,
    pub i18n: Option<ItemI18nMap>,
}

impl ItemShort {
    pub fn name(&self) -> &str {
        self.i18n
            .as_ref()
            .map(|i| i.en.name.as_str())
            .unwrap_or(&self.slug)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ItemI18nMap {
    pub en: ItemI18nEntry,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ItemI18nEntry {
    pub name: String,
}

// ---------------------------------------------------------------------------
// Locations  (GET /v2/locations)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    pub slug: String,
    #[serde(default)]
    pub faction: Option<String>,
    #[serde(default)]
    pub min_level: Option<i32>,
    #[serde(default)]
    pub max_level: Option<i32>,
    #[serde(default)]
    pub i18n: Option<HashMap<String, LocationI18n>>,
}

impl Location {
    pub fn node_name(&self) -> Option<&str> {
        self.i18n.as_ref()?.get("en").map(|e| e.node_name.as_str())
    }
    pub fn system_name(&self) -> Option<&str> {
        self.i18n.as_ref()?.get("en")?.system_name.as_deref()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocationI18n {
    pub node_name: String,
    #[serde(default)]
    pub system_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Missions  (GET /v2/missions)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Mission {
    pub id: String,
    pub slug: String,
    #[serde(default)]
    pub i18n: Option<HashMap<String, MissionI18n>>,
}

impl Mission {
    pub fn name(&self) -> Option<&str> {
        self.i18n.as_ref()?.get("en").map(|e| e.name.as_str())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MissionI18n {
    pub name: String,
}

// ---------------------------------------------------------------------------
// Drop sources  (GET /v1/items/{slug}/dropsources)
// ---------------------------------------------------------------------------

/// Unified struct for both drop-source shapes:
///
/// type = "relic"   → prime part found inside a relic
///   fields: relic (relic item-id), rates
///
/// type = "mission" → relic/item found in a mission rotation
///   fields: location, mission, rate, rarity, rotation (optional)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DropSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub item: String,
    // relic-type fields
    pub relic: Option<String>,
    pub rates: Option<RelicRates>,
    // mission-type fields
    pub location: Option<String>,
    pub mission: Option<String>,
    pub rate: Option<f32>,
    pub rarity: Option<String>,
    pub rotation: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RelicRates {
    pub intact: f32,
    pub exceptional: f32,
    pub flawless: f32,
    pub radiant: f32,
}

// ---------------------------------------------------------------------------
// Orders  (GET /v2/orders/item/{slug})
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderWithUser {
    pub id: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub platinum: u32,
    pub quantity: u32,
    #[serde(default)]
    pub item_id: Option<String>,
    pub user: UserShort,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserShort {
    pub ingame_name: String,
    pub status: String,
    pub reputation: i32,
}
