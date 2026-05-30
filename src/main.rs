mod api;
mod client;
mod models;

use std::collections::HashMap;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use client::RateLimitedClient;
use models::ItemShort;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "wfm",
    about = "Search warframe.market orders. Always filters for in-game sellers."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search orders by price, quantity, and item tags
    Search(SearchArgs),
    /// Find sellers with enough ducat-efficient items to be worth a single trade
    Ducats(DucatsArgs),
}

#[derive(Args)]
struct SearchArgs {
    /// Order type to search: sell or buy
    #[arg(long = "type", default_value = "sell")]
    order_type: String,

    /// Maximum platinum price (inclusive)
    #[arg(long)]
    platinum: Option<u32>,

    /// Minimum quantity per order (inclusive)
    #[arg(long)]
    quantity: Option<u32>,

    /// Comma-separated item tags to filter by (all must match).
    /// Examples: --tags prime   --tags prime,relic
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Args)]
struct DucatsArgs {
    /// Minimum ducats-per-platinum ratio a listing must meet.
    /// E.g. --ratio 15 means the item must yield ≥15 ducats per platinum spent.
    #[arg(long)]
    ratio: f32,

    /// Minimum total quantity a seller must have available across all qualifying
    /// listings before they are shown. Lets you avoid burning a daily trade slot
    /// on a seller who only has one or two cheap items.
    #[arg(long, default_value = "6")]
    quantity: u32,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = RateLimitedClient::new();

    match cli.command {
        Commands::Search(args) => run_search(args, &client).await,
        Commands::Ducats(args) => run_ducats(args, &client).await,
    }
}

// ---------------------------------------------------------------------------
// search subcommand
// ---------------------------------------------------------------------------

async fn run_search(args: SearchArgs, client: &RateLimitedClient) -> Result<()> {
    let required_tags: Vec<String> = args
        .tags
        .as_deref()
        .map(|t| t.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default();

    eprintln!("📦 Fetching item list...");
    let all_items = api::get_items(client).await?;

    let items: Vec<_> = all_items
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

        let orders = match api::get_orders(client, &item.slug).await {
            Ok(o) => o,
            Err(e) => {
                eprintln!("  ⚠ skipping {} — {}", item.slug, e);
                continue;
            }
        };

        for order in &orders {
            if order.user.status != "ingame" { continue; }
            if order.order_type != args.order_type { continue; }
            if args.platinum.is_some_and(|max| order.platinum > max) { continue; }
            if args.quantity.is_some_and(|min| order.quantity < min) { continue; }

            hits += 1;
            println!(
                "[{}] {}p x{} | {} (rep:{}) | https://warframe.market/profile/{} | https://warframe.market/items/{}",
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

// ---------------------------------------------------------------------------
// ducats subcommand
// ---------------------------------------------------------------------------

/// One qualifying listing collected during the scan.
struct Hit {
    item: ItemShort,
    platinum: u32,
    quantity: u32,
    ducats: u32,
}

async fn run_ducats(args: DucatsArgs, client: &RateLimitedClient) -> Result<()> {
    eprintln!("📦 Fetching item list...");
    let all_items = api::get_items(client).await?;

    // Only prime parts have ducat value.
    let ducat_items: Vec<_> = all_items
        .into_iter()
        .filter(|item| item.ducats.unwrap_or(0) > 0)
        .collect();

    let eta = ducat_items.len() as f32 * 0.334;
    eprintln!(
        "🪙 Scanning {} ducat items at ratio ≥{} (~{:.0}s)...",
        ducat_items.len(),
        args.ratio,
        eta
    );

    // ingame_name -> list of qualifying listings from that seller.
    let mut by_seller: HashMap<String, Vec<Hit>> = HashMap::new();

    for (i, item) in ducat_items.iter().enumerate() {
        if (i + 1) % 50 == 0 || i + 1 == ducat_items.len() {
            eprintln!("  [{}/{}] scanned", i + 1, ducat_items.len());
        }

        let ducats = item.ducats.unwrap_or(0);

        let orders = match api::get_orders(client, &item.slug).await {
            Ok(o) => o,
            Err(e) => {
                eprintln!("  ⚠ skipping {} — {}", item.slug, e);
                continue;
            }
        };

        for order in orders {
            if order.order_type != "sell" { continue; }
            if order.user.status != "ingame" { continue; }
            if order.platinum == 0 { continue; }

            let ratio = ducats as f32 / order.platinum as f32;
            if ratio < args.ratio { continue; }

            by_seller
                .entry(order.user.ingame_name.clone())
                .or_default()
                .push(Hit {
                    item: item.clone(),
                    platinum: order.platinum,
                    quantity: order.quantity,
                    ducats,
                });
        }
    }

    // Only keep sellers whose combined available quantity meets the threshold.
    let mut qualified: Vec<(String, Vec<Hit>)> = by_seller
        .into_iter()
        .filter(|(_, hits)| hits.iter().map(|h| h.quantity).sum::<u32>() >= args.quantity)
        .collect();

    // Sort by total ducats-per-platinum value descending so the best deals come first.
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
        // Build the item portion of the message.
        let item_parts: Vec<String> = hits
            .iter()
            .map(|h| format!("{}x {} ({}p)", h.quantity, h.item.name(), h.platinum))
            .collect();

        let body = item_parts.join(", ");
        let msg = format!("/w {username} Hi! I'd like to buy: {body} (warframe.market)");

        // Warframe chat messages are capped at ~255 characters.
        if msg.len() > 255 {
            eprintln!(
                "⚠  Message for {username} is {} chars — Warframe may truncate it.",
                msg.len()
            );
        }

        // Print a summary header to stderr so stdout stays pipe-friendly.
        let total_qty: u32 = hits.iter().map(|h| h.quantity).sum();
        let total_ducats: u32 = hits.iter().map(|h| h.ducats * h.quantity).sum();
        let total_plat: u32 = hits.iter().map(|h| h.platinum * h.quantity).sum();
        eprintln!(
            "👤 {username} — {total_qty} items, {total_plat}p total, ~{total_ducats} ducats"
        );

        println!("{msg}");
    }

    Ok(())
}
