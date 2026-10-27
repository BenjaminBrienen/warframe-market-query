use anyhow::{
    Result,
    anyhow,
};

use crate::{
    bans,
    cli::UserArguments,
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
    arguments: &UserArguments,
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
        arguments.ratio
    );

    let cursed_user_slug = ingame_name_to_user_slug_hack(&arguments.username);
    let user = match store.user(&cursed_user_slug).await {
        Ok(user) => user,
        Err(error) => {
            eprintln!("[{}]:   ⚠ {error}", timestamp());
            return Err(anyhow!("{error}"));
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
    if bans::get().contains(&arguments.username) {
        eprintln!("  ⚠  User {} is banned!", arguments.username);
    }
    let orders = match store
        .orders_by_user(&user.slug, config.cache_user_orders)
        .await
    {
        Ok(order) => order,
        Err(error) => {
            eprintln!("[{}]:   ⚠ {error}", timestamp());
            return Err(anyhow!("{error}"));
        },
    };
    let hits: Vec<_> = orders
        .iter()
        .filter_map(|order| order_to_hit(&items_by_id, order, arguments.ratio))
        .collect();
    if hits.is_empty() {
        eprintln!(
            "[{}]: Nothing to buy from user {} with the given criteria.",
            timestamp(),
            arguments.username
        );
    } else {
        print_user_hits(&arguments.username, &hits);
    }
    Ok(())
}
