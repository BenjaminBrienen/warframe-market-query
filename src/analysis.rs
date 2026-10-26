//! Ducat-farming analysis built entirely from cached data.

use std::collections::HashMap;

use serde::{
    Deserialize,
    Serialize,
};

use crate::models::{
    DropSource,
    ItemShort,
    Location,
    Mission,
};

// Intermediate / cached output

/// Expected ducat yield per relic opened at each refinement level.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelicValue {
    pub intact: f32,
    pub exceptional: f32,
    pub flawless: f32,
    pub radiant: f32,
}

type RelicProbabilities = (u32, f32, f32, f32, f32);

/// Build a map from relic item-ID → expected ducat value at each refinement.
pub fn build_relic_values(
    prime_parts: &[&ItemShort],
    dropsources_by_slug: &HashMap<String, Vec<DropSource>>,
) -> HashMap<String, RelicValue> {
    // relic_id  →  list of (ducats, intact%, exceptional%, flawless%, radiant%)
    let mut contents: HashMap<String, Vec<RelicProbabilities>> = HashMap::new();

    for part in prime_parts {
        let ducats = match part.ducats {
            Some(ducats) if ducats > 0 => ducats,
            _ => continue,
        };

        let Some(sources) = dropsources_by_slug.get(&part.slug) else {
            continue;
        };

        for source in sources {
            if source.r#type != "relic" {
                continue;
            }
            let Some(relic_id) = &source.relic else {
                continue;
            };
            let Some(rates) = &source.rates else {
                continue;
            };

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
                    .map(|&(ducats, rate, _, _, _)| ducats as f32 * rate / 100.0)
                    .sum(),
                exceptional: parts
                    .iter()
                    .map(|&(ducats, _, rate, _, _)| ducats as f32 * rate / 100.0)
                    .sum(),
                flawless: parts
                    .iter()
                    .map(|&(ducats, _, _, rate, _)| ducats as f32 * rate / 100.0)
                    .sum(),
                radiant: parts
                    .iter()
                    .map(|&(ducats, _, _, _, rate)| ducats as f32 * rate / 100.0)
                    .sum(),
            };
            (relic_id, value)
        })
        .collect()
}

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

        for source in sources {
            if source.r#type != "mission" {
                continue;
            }
            let Some(loc_id) = &source.location else {
                continue;
            };
            let Some(mis_id) = &source.mission else {
                continue;
            };
            let Some(rate) = source.rate else { continue };

            // Normalise rotation to uppercase for consistent grouping.
            let rotation = source.rotation.as_deref().map(str::to_uppercase);
            let key = (loc_id.clone(), mis_id.clone(), rotation);

            let increase = rate / 100.0 * value.intact;
            *scores.entry(key).or_default() += increase;
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

    result.sort_by(|first, other| {
        other
            .expected_ducats
            .partial_cmp(&first.expected_ducats)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

pub fn format_score(
    score: &MissionScore,
    locations: &HashMap<String, Location>,
    missions: &HashMap<String, Mission>,
) -> String {
    let location = locations.get(&score.location_id);
    let mission = missions.get(&score.mission_id);

    let system = location
        .and_then(|location| location.system_name())
        .unwrap_or("?");

    let node = location
        .and_then(|location| location.node_name())
        .unwrap_or(score.location_id.as_str());

    let mission_name = mission
        .and_then(|mission| mission.name())
        .unwrap_or(score.mission_id.as_str());

    let rotation_str = match &score.rotation {
        Some(rotation) => format!(" / {rotation}"),
        None => String::new(),
    };

    format!(
        "{system} / {node} ({mission_name}{rotation_str}) | {:.2} ducats",
        score.expected_ducats
    )
}
