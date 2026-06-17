//! Ducat-farming analysis built entirely from cached data.
//!
//! Step 1 — build_relic_values
//!   For each prime part, its dropsources list which relics contain it and at
//!   what rates (intact / exceptional / flawless / radiant).  Aggregating
//!   across all prime parts gives the expected-ducat value of opening each
//!   relic at each refinement level.
//!
//! Step 2 — build_mission_scores
//!   For each relic, its dropsources list which missions drop it and at what
//!   rate.  Multiplying (relic drop rate × relic intact value) and summing
//!   over all relics gives expected ducats per run for each
//!   (location, mission, rotation) combination.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::{DropSource, ItemShort, Location, Mission};

// ---------------------------------------------------------------------------
// Intermediate / cached output
// ---------------------------------------------------------------------------

/// Expected ducat yield per relic opened at each refinement level.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelicValue {
    pub intact: f32,
    pub exceptional: f32,
    pub flawless: f32,
    pub radiant: f32,
}

// ---------------------------------------------------------------------------
// Step 1
// ---------------------------------------------------------------------------

/// Build a map from relic item-ID → expected ducat value at each refinement.
///
/// Input:  prime parts (items where ducats > 0)  +  their cached dropsources.
/// Output: HashMap<relic_id, RelicValue>
pub fn build_relic_values(
    prime_parts: &[&ItemShort],
    dropsources_by_slug: &HashMap<String, Vec<DropSource>>,
) -> HashMap<String, RelicValue> {
    // relic_id  →  list of (ducats, intact%, exceptional%, flawless%, radiant%)
    let mut contents: HashMap<String, Vec<(u32, f32, f32, f32, f32)>> = HashMap::new();

    for part in prime_parts {
        let ducats = match part.ducats {
            Some(d) if d > 0 => d,
            _ => continue,
        };

        let Some(sources) = dropsources_by_slug.get(&part.slug) else {
            continue;
        };

        for src in sources {
            if src.source_type != "relic" {
                continue;
            }
            let Some(ref relic_id) = src.relic else {
                continue;
            };
            let Some(ref rates) = src.rates else { continue };

            contents.entry(relic_id.clone()).or_default().push((
                ducats,
                rates.intact,
                rates.exceptional,
                rates.flawless,
                rates.radiant,
            ));
        }
    }

    contents
        .into_iter()
        .map(|(relic_id, parts)| {
            let value = RelicValue {
                intact: parts
                    .iter()
                    .map(|&(d, r, _, _, _)| d as f32 * r / 100.0)
                    .sum(),
                exceptional: parts
                    .iter()
                    .map(|&(d, _, r, _, _)| d as f32 * r / 100.0)
                    .sum(),
                flawless: parts
                    .iter()
                    .map(|&(d, _, _, r, _)| d as f32 * r / 100.0)
                    .sum(),
                radiant: parts
                    .iter()
                    .map(|&(d, _, _, _, r)| d as f32 * r / 100.0)
                    .sum(),
            };
            (relic_id, value)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Step 2
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MissionScore {
    pub location_id: String,
    pub mission_id: String,
    /// `None` for non-rotation missions (Spy, Capture, etc.)
    pub rotation: Option<String>,
    /// Expected ducats per run assuming intact relics.
    pub expected_ducats: f32,
}

/// Build and sort mission scores from relic drop sources + precomputed relic values.
pub fn build_mission_scores(
    relics: &[&ItemShort],
    dropsources_by_slug: &HashMap<String, Vec<DropSource>>,
    relic_values: &HashMap<String, RelicValue>,
) -> Vec<MissionScore> {
    // (location_id, mission_id, rotation)  →  accumulated expected ducats
    let mut scores: HashMap<(String, String, Option<String>), f32> = HashMap::new();

    for relic in relics {
        let Some(value) = relic_values.get(&relic.id) else {
            continue;
        };
        let Some(sources) = dropsources_by_slug.get(&relic.slug) else {
            continue;
        };

        for src in sources {
            if src.source_type != "mission" {
                continue;
            }
            let Some(ref loc_id) = src.location else {
                continue;
            };
            let Some(ref mis_id) = src.mission else {
                continue;
            };
            let Some(rate) = src.rate else { continue };

            // Normalise rotation to uppercase for consistent grouping.
            let rotation = src.rotation.as_deref().map(|r| r.to_uppercase());
            let key = (loc_id.clone(), mis_id.clone(), rotation);

            *scores.entry(key).or_default() += rate / 100.0 * value.intact;
        }
    }

    let mut result: Vec<MissionScore> = scores
        .into_iter()
        .map(
            |((location_id, mission_id, rotation), expected_ducats)| MissionScore {
                location_id,
                mission_id,
                rotation,
                expected_ducats,
            },
        )
        .collect();

    result.sort_by(|a, b| {
        b.expected_ducats
            .partial_cmp(&a.expected_ducats)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

pub fn format_score(
    score: &MissionScore,
    locations: &HashMap<String, Location>,
    missions: &HashMap<String, Mission>,
) -> String {
    let loc = locations.get(&score.location_id);
    let mis = missions.get(&score.mission_id);

    let system = loc.and_then(|l| l.system_name()).unwrap_or("?");

    let node = loc
        .and_then(|l| l.node_name())
        .unwrap_or(score.location_id.as_str());

    let mission_name = mis
        .and_then(|m| m.name())
        .unwrap_or(score.mission_id.as_str());

    let rotation_str = match &score.rotation {
        Some(r) => format!(" / {r}"),
        None => String::new(),
    };

    format!(
        "{system} / {node} ({mission_name}{rotation_str}) | {:.2} ducats",
        score.expected_ducats
    )
}
