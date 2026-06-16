mod analysis;
mod api;
mod cache;
mod client;
mod config;
mod data;
mod models;

use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};

use analysis::RelicValue;
use client::RateLimitedClient;
use config::Config;
use data::DataStore;
use models::DropSource;

// CLI definition

#[derive(Parser)]
#[command(
    name = "wfmq",
    about = "warframe.market query tool — always filters for in-game sellers."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Refresh the local static-data cache (items, locations, missions, dropsources).
    /// Run this the first time and after major game updates.
    Update(UpdateArgs),

    /// Search sell/buy orders by price, quantity, and item tags.
    Search(SearchArgs),

    /// Find sellers who have enough ducat-efficient items to justify one trade slot.
    Ducats(DucatsArgs),

    /// Show the best mission locations/rotations for farming ducats.
    /// Requires the cache to be populated first (`wfmq update`).
    Locations(LocationsArgs),

    /// Connect to the warframe.market WebSocket and print matching orders live.
    Listen(ListenArgs),
}

#[derive(Args)]
struct ListenArgs {
    /// Minimum ducats-per-platinum ratio a live order must meet to be reported.
    #[arg(long)]
    ratio: f32,
}

#[derive(Args)]
struct UpdateArgs {
    /// Re-fetch dropsource files even when they are already cached.
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct SearchArgs {
    #[arg(long = "type", default_value = "sell")]
    order_type: String,

    /// Maximum platinum price (inclusive).
    #[arg(long)]
    platinum: Option<u32>,

    /// Minimum quantity (inclusive).
    #[arg(long)]
    quantity: Option<u32>,

    /// Comma-separated item tags that must ALL match.  E.g. --tags prime,relic
    #[arg(long)]
    tags: Option<String>,
}

// ducats

#[derive(Args)]
struct DucatsArgs {
    /// Minimum ducats-per-platinum ratio.  E.g. --ratio 15 → ≥15 ducats/platinum.
    #[arg(long)]
    ratio: f32,

    /// Minimum total item-units a seller must have available across all
    /// qualifying listings.  Prevents wasting a daily trade slot.
    #[arg(long, default_value = "6")]
    quantity: u32,
}

// locations

#[derive(Args)]
struct LocationsArgs {
    /// Number of results to display.
    #[arg(long, default_value = "25")]
    limit: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load();
    let client = RateLimitedClient::new();

    match cli.command {
        Commands::Update(args)    => run_update(args, &client, &config).await,
        Commands::Search(args)    => run_search(args, &client, &config).await,
        Commands::Ducats(args)    => run_ducats(args, &client, &config).await,
        Commands::Locations(args) => run_locations(args, &client, &config).await,
        Commands::Listen(args)    => run_listen(args, &client, &config).await,
    }
}

// update

async fn run_update(args: UpdateArgs, client: &RateLimitedClient, _config: &Config) -> Result<()> {
    // Quick single-call endpoints
    eprintln!("📦 Fetching items...");
    let items = api::get_items(client).await?;
    cache::write(&cache::items_path(), &items)?;

    eprintln!("🌍 Fetching locations...");
    let locations = api::get_locations(client).await?;
    cache::write(&cache::locations_path(), &locations)?;

    eprintln!("🎯 Fetching missions...");
    let missions = api::get_missions(client).await?;
    cache::write(&cache::missions_path(), &missions)?;

    // Dropsource targets
    let prime_parts: Vec<_> = items.iter().filter(|i| i.ducats.unwrap_or(0) > 0).collect();
    let relics: Vec<_>      = items.iter().filter(|i| i.tags.contains(&"relic".to_string())).collect();

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
        targets.iter().copied().collect()
    } else {
        targets
            .iter()
            .copied()
            .filter(|item| !cache::dropsources_path(&item.slug).exists())
            .collect()
    };

    if to_fetch.is_empty() {
        eprintln!("✅ All dropsource files already cached (use --force to refresh).");
    } else {
        let eta = to_fetch.len() as f32 * 0.334;
        eprintln!(
            "⬇️  Fetching dropsources for {} items (~{:.0}s at rate limit)...",
            to_fetch.len(),
            eta
        );

        for (i, item) in to_fetch.iter().enumerate() {
            if (i + 1) % 50 == 0 || i + 1 == to_fetch.len() {
                eprintln!("  [{}/{}]", i + 1, to_fetch.len());
            }
            match api::get_dropsources(client, &item.slug).await {
                Ok(sources) => {
                    let _ = cache::write(&cache::dropsources_path(&item.slug), &sources);
                }
                Err(e) => {
                    eprintln!("  ⚠ {} — {e}", item.slug);
                }
            }
        }
    }

    // Compute & cache relic values
    eprintln!("🪙 Computing expected ducat values per relic...");

    let mut dropsources_by_slug: HashMap<String, Vec<DropSource>> = HashMap::new();
    for part in &prime_parts {
        if let Some(sources) = cache::read::<Vec<DropSource>>(&cache::dropsources_path(&part.slug)) {
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

// search

async fn run_search(args: SearchArgs, client: &RateLimitedClient, config: &Config) -> Result<()> {
    let store = DataStore::new(client, config);

    let required_tags: Vec<String> = args
        .tags
        .as_deref()
        .map(|t| t.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default();

    eprintln!("📦 Loading item list...");
    let items: Vec<_> = store
        .items()
        .await?
        .into_iter()
        .filter(|item| {
            required_tags.is_empty()
                || required_tags.iter().all(|tag| item.tags.contains(tag))
        })
        .collect();

    let eta = items.len() as f32 * 0.334;
    eprintln!("🔍 Scanning {} items (~{:.0}s)...", items.len(), eta);

    let mut hits = 0usize;
    for (i, item) in items.iter().enumerate() {
        if (i + 1) % 50 == 0 || i + 1 == items.len() {
            eprintln!("  [{}/{}] scanned", i + 1, items.len());
        }

        let orders = match store.orders(&item.slug).await {
            Ok(o) => o,
            Err(e) => { eprintln!("  ⚠ {}: {e}", item.slug); continue; }
        };

        for order in &orders {
            if order.user.status != "ingame"           { continue; }
            if order.order_type != args.order_type     { continue; }
            if args.platinum.is_some_and(|max| order.platinum > max) { continue; }
            if args.quantity.is_some_and(|min| order.quantity < min)  { continue; }

            hits += 1;
            println!(
                "[{}] {}:platinum: x{} | {} (rep:{}) | https://warframe.market/profile/{} | https://warframe.market/items/{}",
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

    eprintln!("✅ Done — {hits} matching order(s).");
    Ok(())
}

// ducats

struct Hit {
    item_name: String,
    platinum:  u32,
    quantity:  u32,
    ducats:    u32,
}

async fn run_ducats(args: DucatsArgs, client: &RateLimitedClient, config: &Config) -> Result<()> {
    let store = DataStore::new(client, config);

    eprintln!("📦 Loading item list...");
    let all_items = store.items().await?;

    let ducat_items: Vec<_> = all_items
        .into_iter()
        .filter(|i| i.ducats.unwrap_or(0) > 0)
        .collect();

    let eta = ducat_items.len() as f32 * 0.334;
    eprintln!(
        "🪙 Scanning {} ducat items at ratio ≥{} (~{:.0}s)...",
        ducat_items.len(), args.ratio, eta
    );

    let mut by_seller: HashMap<String, Vec<Hit>> = HashMap::new();

    for (i, item) in ducat_items.iter().enumerate() {
        if (i + 1) % 50 == 0 || i + 1 == ducat_items.len() {
            eprintln!("  [{}/{}] scanned", i + 1, ducat_items.len());
        }

        let ducats = item.ducats.unwrap_or(0);
        let orders = match store.orders(&item.slug).await {
            Ok(o) => o,
            Err(e) => { eprintln!("  ⚠ {}: {e}", item.slug); continue; }
        };

        for order in orders {
            if order.order_type != "sell"       { continue; }
            if order.user.status != "ingame"    { continue; }
            if order.platinum == 0              { continue; }
            if (ducats as f32 / order.platinum as f32) < args.ratio { continue; }
            by_seller.entry(order.user.ingame_name.clone()).or_default().push(Hit {
                item_name: item.name().to_owned(),
                platinum:  order.platinum,
                quantity:  order.quantity,
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
        score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal)
    });

    eprintln!("✅ {} seller(s) qualify.\n", qualified.len());

    for (username, hits) in &qualified {
        let total_qty:    u32 = hits.iter().map(|h| h.quantity).sum();
        let total_plat:   u32 = hits.iter().map(|h| h.platinum * h.quantity).sum();
        let total_ducats: u32 = hits.iter().map(|h| h.ducats * h.quantity).sum();
        eprintln!(
            "👤 {username} — {total_qty} items, {total_plat}:platinum: total, ~{total_ducats} ducats"
        );

        let item_parts: Vec<String> = hits
            .iter()
            .map(|h| format!("{}x {} ({}:platinum:)", h.quantity, h.item_name, h.platinum))
            .collect();

        let msg = format!(
            "/w {username} Hi! I'd like to buy: {} (warframe.market via wfmq)",
            item_parts.join(", ")
        );

        if msg.len() > 255 {
            eprintln!(
                "  ⚠  Message is {} chars — Warframe may truncate it.",
                msg.len()
            );
        }

        println!("{msg}");
    }

    Ok(())
}

// locations

async fn run_locations(args: LocationsArgs, client: &RateLimitedClient, config: &Config) -> Result<()> {
    let store = DataStore::new(client, config);

    // These three are quick and auto-fetch if needed.
    eprintln!("📦 Loading game data...");
    let items     = store.items().await?;
    let locs_list = store.locations().await?;
    let mis_list  = store.missions().await?;

    // Relic values must have been built by `update`.
    let relic_values: HashMap<String, RelicValue> = match cache::read(&cache::ducats_per_relic_path()) {
        Some(v) => v,
        None => bail!(
            "No relic value cache found. Please run `wfmq update` first to populate drop-source data."
        ),
    };

    let relics: Vec<_> = items.iter().filter(|i| i.tags.contains(&"relic".to_string())).collect();
    eprintln!("🗺️  Loading dropsources for {} relics...", relics.len());

    let mut relic_dropsources: HashMap<String, Vec<DropSource>> = HashMap::new();
    let mut missing = 0usize;

    for relic in &relics {
        match store.dropsources_cached(&relic.slug) {
            Some(sources) => { relic_dropsources.insert(relic.slug.clone(), sources); }
            None          => { missing += 1; }
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
    eprintln!("🏆 Showing top {} of {} location/rotation combinations (intact relics):\n", args.limit.min(total), total);

    for score in resolved.iter().take(args.limit) {
        println!("{}", analysis::format_score(score, &locations_map, &missions_map));
    }

    Ok(())
}

// listen

async fn run_listen(args: ListenArgs, client: &RateLimitedClient, config: &Config) -> Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

    let store = DataStore::new(client, config);

    // Build item_id → (display name, ducats) for all prime parts.
    eprintln!("📦 Loading item data...");
    let items = store.items().await?;
    let items_by_id: HashMap<String, (String, u32)> = items
        .iter()
        .filter_map(|i| {
            let d = i.ducats.filter(|&d| d > 0)?;
            Some((i.id.clone(), (i.name().to_owned(), d)))
        })
        .collect();

    eprintln!(
        "🪙 {} prime items with ducat values loaded. Ratio filter: ≥{}",
        items_by_id.len(),
        args.ratio
    );

    // Build WebSocket handshake request with the mandatory `wfm` sub-protocol.
    let mut request = "wss://ws.warframe.market/socket".into_client_request()?;
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        "wfm".parse()?,
    );

    eprintln!("🔌 Connecting to wss://ws.warframe.market/socket ...");
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await?;
    eprintln!("✅ Connected. Waiting for orders...\n");

    // Subscribe to new-order events.
    let sub = serde_json::json!({
        "route": "@wfm|cmd/subscribe/newOrders",
        "payload": {
            "platform": "pc",
            "crossplay": true
        },
        "id": "wfmq-listen-1"
    });
    ws.send(Message::Text(sub.to_string())).await?;

    while let Some(raw) = ws.next().await {
        let raw = match raw {
            Ok(m) => m,
            Err(e) => {
                eprintln!("⚠  WebSocket error: {e}");
                break;
            }
        };

        // Server sends periodic pings; tungstenite auto-replies with pong.
        let text = match raw {
            Message::Text(t)  => t,
            Message::Close(_) => { eprintln!("🔌 Server closed the connection."); break; }
            _                 => continue,
        };

        let msg: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Only handle new-order events.
        let route = msg.get("route").and_then(|r| r.as_str()).unwrap_or("");
        if !route.contains("newOrder") { continue; }

        let Some(payload) = msg.get("payload") else { continue };
        let order: models::OrderWithUser = match serde_json::from_value(payload.clone()) {
            Ok(o) => o,
            Err(_) => continue,
        };

        // --- Filters ---
        if order.order_type != "sell" { continue; }
        // Accept both spellings — docs show "in_game" but REST uses "ingame".
        let status = order.user.status.as_str();
        if status != "ingame" && status != "in_game" { continue; }

        let item_id = match &order.item_id {
            Some(id) => id,
            None => continue,
        };

        let Some((item_name, ducats)) = items_by_id.get(item_id) else { continue };
        if order.platinum == 0 { continue; }

        let ratio = *ducats as f32 / order.platinum as f32;
        if ratio < args.ratio { continue; }

        // --- Output ---
        let msg_out = format!(
            "/w {} Hi! I'd like to buy: 1x {} ({}:platinum:) (warframe.market via wfmq)",
            order.user.ingame_name, item_name, order.platinum,
        );

        println!(
            "🔔 {} | {}:platinum: | {:.0} d/:platinum: | x{} available | {}",
            item_name,
            order.platinum,
            ratio,
            order.quantity,
            order.user.ingame_name,
        );
        println!("   {msg_out}\n");
    }

    eprintln!("📴 Disconnected.");
    Ok(())
}