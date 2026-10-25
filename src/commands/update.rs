use std::collections::HashMap;

use anyhow::Result;

use crate::{
    analysis,
    api,
    cache,
    cli::UpdateArgs,
    client::RateLimitedClient,
    models::{
        self,
        DropSource,
    },
    util::{
        eta_secs,
        log_progress,
        positive_ducats,
        timestamp,
    },
};

pub async fn run(
    args: UpdateArgs,
    client: &RateLimitedClient,
) -> Result<()> {
    // Quick single-call endpoints
    eprintln!("[{}]: 📦 Fetching items...", timestamp());
    let items = api::get_items(client).await?;
    cache::write(&cache::items_path(), &items)?;

    eprintln!("[{}]: 🌍 Fetching locations...", timestamp());
    let locations = api::get_locations(client).await?;
    cache::write(&cache::locations_path(), &locations)?;

    eprintln!("[{}]: 🎯 Fetching missions...", timestamp());
    let missions = api::get_missions(client).await?;
    cache::write(&cache::missions_path(), &missions)?;

    // Dropsource targets
    let prime_parts: Vec<_> = items
        .iter()
        .filter(|i| positive_ducats(i.ducats).is_some())
        .collect();
    let relics: Vec<_> = items
        .iter()
        .filter(|i| i.tags.contains(&"relic".to_string()))
        .collect();

    eprintln!(
        "📊 {} prime parts, {} relics identified.",
        prime_parts.len(),
        relics.len()
    );

    // Combine into one target list; deduplicate by slug just in case.
    let mut targets: Vec<&models::ItemShort> = Vec::new();
    for item in prime_parts.iter().chain(relics.iter()) {
        if !targets.iter().any(|t| t.slug == item.slug) {
            targets.push(item);
        }
    }

    let to_fetch: Vec<_> = if args.force {
        targets.to_vec()
    } else {
        targets
            .iter()
            .copied()
            .filter(|item| !cache::dropsources_path(&item.slug).exists())
            .collect()
    };

    if to_fetch.is_empty() {
        eprintln!(
            "[{}]: ✅ All dropsource files already cached (use --force to refresh).",
            timestamp()
        );
    } else {
        eprintln!(
            "⬇️  Fetching dropsources for {} items (~{:.0}s at rate limit)...",
            to_fetch.len(),
            eta_secs(to_fetch.len())
        );

        for (i, item) in to_fetch.iter().enumerate() {
            log_progress(i + 1, to_fetch.len(), "fetched");
            match api::get_dropsources(client, &item.slug).await {
                Ok(sources) => {
                    let _ = cache::write(&cache::dropsources_path(&item.slug), &sources);
                },
                Err(e) => {
                    eprintln!("[{}]:   ⚠ {} — {e}", timestamp(), item.slug);
                },
            }
        }
    }

    // Compute & cache relic values
    eprintln!(
        "[{}]: 🪙  Computing expected ducat values per relic...",
        timestamp()
    );

    let mut dropsources_by_slug: HashMap<String, Vec<DropSource>> = HashMap::new();
    for part in &prime_parts {
        if let Some(sources) = cache::read::<Vec<DropSource>>(&cache::dropsources_path(&part.slug))
        {
            dropsources_by_slug.insert(part.slug.clone(), sources);
        }
    }

    let relic_values = analysis::build_relic_values(&prime_parts, &dropsources_by_slug);
    cache::write(&cache::ducats_per_relic_path(), &relic_values)?;

    eprintln!(
        "✅ Update complete — {} relics valued, data written to ./data/",
        relic_values.len()
    );
    Ok(())
}
