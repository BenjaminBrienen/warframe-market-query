use std::collections::HashMap;

use anyhow::Result;

use crate::{
    bans,
    cli::DucatsArgs,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    hits::{
        Hit,
        print_user_hits,
        without_bans,
    },
    util::{
        eta_secs,
        log_progress,
        positive_ducats,
        timestamp,
    },
};

pub async fn run(
    args: DucatsArgs,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    eprintln!("[{}]: 📦 Loading item list...", timestamp());
    let ducat_items: Vec<_> = store
        .items()
        .await?
        .into_iter()
        .filter(|i| positive_ducats(i.ducats).is_some())
        .collect();

    eprintln!(
        "[{}]: 🪙  Scanning {} ducat items at ratio ≥{} (~{:.0}s)...",
        timestamp(),
        ducat_items.len(),
        args.ratio,
        eta_secs(ducat_items.len())
    );

    let mut by_seller: HashMap<String, Vec<Hit>> = HashMap::new();

    for (i, item) in ducat_items.iter().enumerate() {
        log_progress(i + 1, ducat_items.len(), "scanned");

        let ducats = item.ducats.unwrap_or(0);
        let orders = match store.orders_by_item(&item.slug).await {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[{}]:   ⚠ {}: {e}", timestamp(), item.slug);
                continue;
            },
        };

        for order in orders {
            if order.r#type != "sell" {
                continue;
            }
            if order.user.status != "ingame" && !args.include_offline {
                continue;
            }
            if order.platinum == 0 {
                continue;
            }
            if (ducats as f32 / order.platinum as f32) < args.ratio {
                continue;
            }
            by_seller
                .entry(order.user.ingame_name.clone())
                .or_default()
                .push(Hit {
                    item_name: item.name().to_owned(),
                    platinum: order.platinum,
                    quantity: order.quantity,
                    ducats,
                });
        }
    }

    let mut qualified: Vec<(String, Vec<Hit>)> = by_seller
        .into_iter()
        .filter(|(_, hits)| hits.iter().map(|h| h.quantity).sum::<u32>() >= args.quantity)
        .collect();

    // Sort by weighted ducat efficiency descending.
    qualified.sort_by(|(_, a), (_, b)| {
        let score = |hits: &[Hit]| -> f32 {
            hits.iter()
                .map(|h| h.ducats as f32 / h.platinum as f32 * h.quantity as f32)
                .sum()
        };
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    eprintln!(
        "[{}]: ✅ {} seller(s) qualify.\n",
        timestamp(),
        qualified.len()
    );

    for (username, hits) in without_bans(&qualified, bans::get()) {
        print_user_hits(username, hits);
    }

    Ok(())
}
