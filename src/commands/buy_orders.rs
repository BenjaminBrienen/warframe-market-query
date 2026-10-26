use anyhow::{
    Result,
    bail,
};

use crate::{
    api::{
        self,
        OrderOutcome,
    },
    cli::BuyOrdersArguments,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    util::{
        PLATINUM,
        eta_secs,
        log_progress,
        positive_ducats,
        timestamp,
    },
};

/// Every buy order is for this many units.
const QUANTITY: u32 = 99;

/// Abort when this many orders in a row are rejected (not logged in,
/// order limit reached, ...) instead of hammering the API.
const MAX_CONSECUTIVE_FAILURES: usize = 5;

/// Price in platinum for a buy order on an item worth `ducats`.
/// Items with any other ducat value are skipped.
pub fn price_for_ducats(ducats: u32) -> Option<u32> {
    match ducats {
        45 => Some(2),
        100 => Some(5),
        d if d > 100 => Some(6),
        _ => None,
    }
}

pub async fn run(
    arguments: BuyOrdersArguments,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    eprintln!("[{}]: 📦 Loading item list...", timestamp());
    let items = store.items().await?;

    // (item, ducats, platinum to bid)
    let planned: Vec<_> = items
        .iter()
        .filter_map(|item| {
            let ducats = positive_ducats(item.ducats)?;
            let platinum = price_for_ducats(ducats)?;
            Some((item, ducats, platinum))
        })
        .collect();

    if planned.is_empty() {
        eprintln!(
            "[{}]: Nothing to do — no items worth 45, 100 or more than 100 ducats.",
            timestamp()
        );
        return Ok(());
    }

    let total_platinum: u32 = planned.iter().map(|(_, _, plat)| plat * QUANTITY).sum();
    eprintln!(
        "[{}]: 🛒 {} buy orders of {QUANTITY}x planned (up to {total_platinum}{PLATINUM} if every one fills).",
        timestamp(),
        planned.len(),
    );

    if arguments.dry_run {
        for (item, ducats, platinum) in &planned {
            println!(
                "{} ({ducats} ducats): buy {QUANTITY}x @ {platinum}{PLATINUM}",
                item.name()
            );
        }
        eprintln!("[{}]: Dry run — nothing was created.", timestamp());
        return Ok(());
    }

    eprintln!(
        "[{}]: ⬆️  Creating orders (~{:.0}s at rate limit)...",
        timestamp(),
        eta_secs(planned.len())
    );

    let mut created = 0usize;
    let mut already_existed = 0usize;
    let mut failed = 0usize;
    let mut consecutive_failures = 0usize;

    for (i, (item, ducats, platinum)) in planned.iter().enumerate() {
        match api::create_buy_order(client, config, &item.id, *platinum, QUANTITY).await {
            Ok(OrderOutcome::Created) => {
                created += 1;
                consecutive_failures = 0;
                println!(
                    "✅ {} ({ducats} ducats): buy {QUANTITY}x @ {platinum}{PLATINUM}",
                    item.name()
                );
            },
            // Already on the account at this price: that's the goal.
            Ok(OrderOutcome::AlreadyExists) => {
                already_existed += 1;
                consecutive_failures = 0;
                println!(
                    "☑️  {} ({ducats} ducats): order already exists",
                    item.name()
                );
            },
            Ok(OrderOutcome::ExceededOrderLimit) => {
                bail!(
                    "[{}]:   ⚠ Order limit reached. Get unlimited orders by subscribing to 42 Bytes on Patreon.",
                    timestamp(),
                );
            },
            Err(error) => {
                failed += 1;
                consecutive_failures += 1;
                eprintln!("[{}]:   ⚠ {}: {error:#}", timestamp(), item.slug);
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    bail!(
                        "aborting after {consecutive_failures} failed orders in a row \
                                 ({created} created, {already_existed} already existed, {failed} failed so far)"
                    );
                }
            },
        }
        log_progress(i + 1, planned.len(), "processed");
    }

    eprintln!(
        "[{}]: ✅ Done — {created} created, {already_existed} already existed, {failed} failed.",
        timestamp()
    );
    Ok(())
}
