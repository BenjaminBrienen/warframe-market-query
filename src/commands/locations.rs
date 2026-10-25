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
    cli::LocationsArgs,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    models::DropSource,
    util::timestamp,
};

pub async fn run(
    args: LocationsArgs,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    // These three are quick and auto-fetch if needed.
    eprintln!("[{}]: 📦 Loading game data...", timestamp());
    let items = store.items().await?;
    let locs_list = store.locations().await?;
    let mis_list = store.missions().await?;

    // Relic values must have been built by `update`.
    let relic_values: HashMap<String, RelicValue> = match cache::read(
        &cache::ducats_per_relic_path(),
    ) {
        Some(v) => v,
        None => bail!(
            "No relic value cache found. Please run `wfmq update` first to populate drop-source data."
        ),
    };

    let relics: Vec<_> = items
        .iter()
        .filter(|i| i.tags.contains(&"relic".to_string()))
        .collect();
    eprintln!(
        "[{}]: 🗺️  Loading dropsources for {} relics...",
        timestamp(),
        relics.len()
    );

    let mut relic_dropsources: HashMap<String, Vec<DropSource>> = HashMap::new();
    let mut missing = 0usize;

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
    let locations_map: HashMap<String, _> =
        locs_list.into_iter().map(|l| (l.id.clone(), l)).collect();
    let missions_map: HashMap<String, _> =
        mis_list.into_iter().map(|m| (m.id.clone(), m)).collect();

    let scores = analysis::build_mission_scores(&relics, &relic_dropsources, &relic_values);

    // Filter out entries where we couldn't resolve at least the location name.
    let resolved: Vec<_> = scores
        .iter()
        .filter(|s| locations_map.contains_key(&s.location_id))
        .collect();

    let total = resolved.len();
    eprintln!(
        "[{}]: 🏆 Showing top {} of {} location/rotation combinations (intact relics):\n",
        timestamp(),
        args.limit.min(total),
        total
    );

    for score in resolved.iter().take(args.limit) {
        println!(
            "[{}]: {}",
            timestamp(),
            analysis::format_score(score, &locations_map, &missions_map)
        );
    }

    Ok(())
}
