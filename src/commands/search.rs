use anyhow::Result;

use crate::{
    bans,
    cli::SearchArguments,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    util::{
        PLATINUM,
        eta_secs,
        log_progress,
        timestamp,
    },
};

pub async fn run(
    arguments: &SearchArguments,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    let required_tags: Vec<String> = arguments
        .tags
        .as_deref()
        .map(|tags| {
            tags.split(',')
                .map(|tag| tag.trim().to_lowercase())
                .collect()
        })
        .unwrap_or_default();

    eprintln!("[{}]: 📦 Loading item list...", timestamp());
    let items: Vec<_> = store
        .items()
        .await?
        .into_iter()
        .filter(|item| {
            required_tags.is_empty() || required_tags.iter().all(|tag| item.tags.contains(tag))
        })
        .collect();

    eprintln!(
        "[{}]: 🔍 Scanning {} items (~{:.0}s)...",
        timestamp(),
        items.len(),
        eta_secs(items.len())
    );

    let mut hits = 0_usize;
    for (index, item) in items.iter().enumerate() {
        log_progress(index + 1, items.len(), "scanned");

        let orders = match store.orders_by_item(&item.slug).await {
            Ok(order) => order,
            Err(error) => {
                eprintln!("[{}]:   ⚠ {}: {error}", timestamp(), item.slug);
                continue;
            },
        };

        for order in &orders {
            if order.user.status != "ingame" && !arguments.include_offline {
                continue;
            }
            if bans::get().contains(&order.user.ingame_name) {
                continue;
            }
            if order.r#type != arguments.r#type {
                continue;
            }
            if arguments.platinum.is_some_and(|max| order.platinum > max) {
                continue;
            }
            if arguments.quantity.is_some_and(|min| order.quantity < min) {
                continue;
            }

            hits += 1;
            println!(
                "[{}] {}{PLATINUM} x{} | {} (rep:{}) | https://warframe.market/profile/{} | https://warframe.market/items/{}",
                item.name(),
                order.platinum,
                order.quantity,
                order.user.ingame_name,
                order.user.reputation,
                order.user.ingame_name,
                item.slug,
            );
        }
    }

    eprintln!("[{}]: ✅ Done — {hits} matching order(s).", timestamp());
    Ok(())
}
