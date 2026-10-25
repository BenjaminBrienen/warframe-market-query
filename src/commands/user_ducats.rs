use anyhow::{
    Result,
    anyhow,
};

use crate::{
    bans,
    cli::UserArgs,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    hits::{
        items_with_ducats,
        order_to_hit,
        print_user_hits,
    },
    util::{
        ingame_name_to_user_slug_hack,
        timestamp,
        username_quoted,
    },
};

pub async fn run(
    args: UserArgs,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    // Build item_id → (display name, ducats) for all prime parts.
    eprintln!("[{}]: 📦 Loading item data...", timestamp());
    let items_by_id = items_with_ducats(&store).await?;

    eprintln!(
        "[{}]: 🪙  {} prime items with ducat values loaded. Ratio filter: ≥{}",
        timestamp(),
        items_by_id.len(),
        args.ratio
    );

    let cursed_user_slug = ingame_name_to_user_slug_hack(&args.username);
    let user = match store.user(&cursed_user_slug).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[{}]:   ⚠ {e}", timestamp());
            return Err(anyhow!("{e}"));
        },
    };
    if user.status != "ingame" {
        eprintln!(
            "[{}]: User {username} is not ingame anymore.",
            timestamp(),
            username = username_quoted(&user.ingame_name)
        );
        return Ok(());
    }
    if bans::get().contains(&args.username) {
        eprintln!("  ⚠  User {} is banned!", args.username);
    }
    let orders = match store.orders_by_user(&user.slug).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[{}]:   ⚠ {e}", timestamp());
            return Err(anyhow!("{e}"));
        },
    };
    let hits: Vec<_> = orders
        .iter()
        .filter_map(|order| order_to_hit(&items_by_id, order, args.ratio))
        .collect();
    if hits.is_empty() {
        eprintln!(
            "[{}]: Nothing to buy from user {} with the given criteria.",
            timestamp(),
            args.username
        );
    } else {
        print_user_hits(&args.username, &hits);
    }
    Ok(())
}
