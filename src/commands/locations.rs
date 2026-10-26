use std::collections::HashMap;

use anyhow::{
    Result,
    bail,
};

use crate::{
    analysis::{
        self,
        RelicValue,
    },
    cache,
    cli::LocationsArguments,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    models::DropSource,
    util::timestamp,
};

pub async fn run(
    arguments: &LocationsArguments,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    // These three are quick and auto-fetch if needed.
    eprintln!("[{}]: 📦 Loading game data...", timestamp());
    let items = store.items().await?;
    let locations = store.locations().await?;
    let missions = store.missions().await?;

    // Relic values must have been built by `update`.
    let relic_values: HashMap<String, RelicValue> = match cache::read(
        &cache::ducats_per_relic_path(),
    ) {
        Some(value) => value,
        None => bail!(
            "No relic value cache found. Please run `wfmq update` first to populate drop-source data."
        ),
    };

    let relics: Vec<_> = items
        .iter()
        .filter(|item| item.tags.contains(&"relic".to_owned()))
        .collect();
    eprintln!(
        "[{}]: 🗺️  Loading dropsources for {} relics...",
        timestamp(),
        relics.len()
    );

    let mut relic_dropsources: HashMap<String, Vec<DropSource>> = HashMap::new();
    let mut missing = 0_usize;

    for relic in &relics {
        match store.dropsources_cached(&relic.slug) {
            Some(sources) => {
                relic_dropsources.insert(relic.slug.clone(), sources);
            },
            None => {
                missing += 1;
            },
        }
    }

    if missing > 0 {
        eprintln!(
            "  ⚠  Dropsource data missing for {missing} relics — run `wfmq update` to populate."
        );
    }

    // Build lookup maps keyed by ID.
    let locations_map: HashMap<String, _> = locations
        .into_iter()
        .map(|location| (location.id.clone(), location))
        .collect();
    let missions_map: HashMap<String, _> = missions
        .into_iter()
        .map(|mission| (mission.id.clone(), mission))
        .collect();

    let scores = analysis::build_mission_scores(&relics, &relic_dropsources, &relic_values);

    // Filter out entries where we couldn't resolve at least the location name.
    let resolved: Vec<_> = scores
        .iter()
        .filter(|score| locations_map.contains_key(&score.location_id))
        .collect();

    let total = resolved.len();
    eprintln!(
        "[{}]: 🏆 Showing top {} of {total} location/rotation combinations (intact relics):\n",
        timestamp(),
        arguments.limit.min(total)
    );

    for score in resolved.iter().take(arguments.limit) {
        println!(
            "[{}]: {}",
            timestamp(),
            analysis::format_score(score, &locations_map, &missions_map)
        );
    }

    Ok(())
}
