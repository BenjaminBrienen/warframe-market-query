use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub r#type: String,
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

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderWithUser {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub platinum: u32,
    pub quantity: u32,
    pub item_id: String,
    pub user: UserShort,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserShort {
    pub ingame_name: String,
    pub status: String,
    pub reputation: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// Is the unique identifier of the order.
    pub id: String,

    #[serde(rename = "type")]
    /// Specifies whether the order is a 'buy' or 'sell'.
    pub r#type: String,

    /// Is the total platinum currency involved in the order.
    pub platinum: i32,

    /// Represents the number of items included in the order.
    pub quantity: i32,

    /// (optional) indicates the items quantity per transaction.
    pub per_trade: Option<i8>,

    /// (optional) specifies the rank or level of the item in the order.
    pub rank: Option<i8>,

    /// (optional) specifies number of charges left (used in requiem mods).
    pub charges: Option<i8>,

    /// (optional) defines the specific subtype or category of the item.
    pub subtype: Option<String>,

    /// (optional) denotes the count of amber stars in a sculpture order.
    pub amber_stars: Option<i8>,

    /// (optional) denotes the count of cyan stars in a sculpture order.
    pub cyan_stars: Option<i8>,

    /// (auth\mod) Indicates whether the order is publicly visible or not.
    pub visible: bool,

    /// Records the creation time of the order.
    pub created_at: String,

    /// Records the last modification time of the order.
    pub updated_at: String,

    /// Is the unique identifier of the item involved in the order.
    pub item_id: String,

    /// User-defined group to which the order belongs
    pub group: Option<String>,
}

/// User represents a public user profile with full information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Unique identifier of the user.
    pub id: String,

    /// User's in-game name.
    pub ingame_name: String,

    /// User's slug - a human-readable unique user identifier.
    pub slug: String,

    /// Optional link to the user's avatar image.
    pub avatar: Option<String>,

    /// Optional link to the user's profile background image.
    pub background: Option<String>,

    /// Optional HTML-formatted text about the user.
    pub about: Option<String>,

    /// User's reputation score.
    pub reputation: i16,

    /// Optional in-game mastery level.
    pub mastery_level: Option<i8>,

    /// Platform the user plays on.
    pub platform: String,

    /// Indicates if the user is open to cross-platform trading.
    pub crossplay: bool,

    /// User's locale or preferred language.
    pub locale: String,

    // /// List of achievements the user chose to showcase.
    // pub achievement_showcase: Vec<Achievement>,
    /// Current status of the user.
    pub status: String,

    // /// Current activity the user is engaged in.
    // #[serde(rename = "activity")]
    // pub activity: Activity,
    /// Timestamp of the user's last online presence.
    pub last_seen: String,

    /// Indicates whether the user is currently banned.
    pub banned: Option<bool>,

    /// End date of the current ban, if applicable.
    pub ban_until: Option<String>,

    // Fields below are accessible only to moderators and admins.
    /// Indicates whether the user has been warned.
    pub warned: Option<bool>,

    /// Warning message, if any.
    pub warn_message: Option<String>,

    /// Ban message or reason for the ban, if any.
    pub ban_message: Option<String>,
}
