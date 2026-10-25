use clap::{
    Args,
    Parser,
    Subcommand,
};

#[derive(Parser)]
#[command(
    name = "wfmq",
    about = "warframe.market query tool — always filters for in-game sellers."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

    /// Find out what is worth trading from the user for ducats.
    UserDucats(UserArgs),

    /// Create a buy order (qty 99) for every ducat item:
    /// 45 ducats → 2p, 100 ducats → 5p, more than 100 ducats → 6p.
    BuyOrders(BuyOrdersArgs),

    /// Test sound effect.
    PlaySound(()),

    /// Ban a user (do not want to trade with them).
    Ban(BanArgs),

    /// Unban a user.
    Unban(BanArgs),
}

#[derive(Args)]
pub struct BanArgs {
    /// Username of the user to ban.
    pub username: String,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Re-fetch dropsource files even when they are already cached.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ListenArgs {
    /// Minimum ducats-per-platinum ratio a live order must meet to be reported.
    #[arg(long)]
    pub ratio: f32,
}

#[derive(Args)]
pub struct SearchArgs {
    #[arg(long = "type", default_value = "sell")]
    pub r#type: String,

    /// Maximum platinum price (inclusive).
    #[arg(long)]
    pub platinum: Option<u32>,

    /// Minimum quantity (inclusive).
    #[arg(long)]
    pub quantity: Option<u32>,

    /// Comma-separated item tags that must ALL match.  E.g. --tags prime,relic
    #[arg(long)]
    pub tags: Option<String>,

    #[clap(long, short, action)]
    pub include_offline: bool,
}

#[derive(Args)]
pub struct DucatsArgs {
    /// Minimum ducats-per-platinum ratio.  E.g. --ratio 15 → ≥15 ducats/platinum.
    #[arg(long)]
    pub ratio: f32,

    /// Minimum total item-units a seller must have available across all
    /// qualifying listings.  Prevents wasting a daily trade slot.
    #[arg(long, default_value = "6")]
    pub quantity: u32,

    #[clap(long, short, action)]
    pub include_offline: bool,
}

#[derive(Args)]
pub struct UserArgs {
    /// Username (slug) of the user. Also the "profile".
    #[arg(long)]
    pub username: String,

    /// Minimum ducats-per-platinum ratio.  E.g. --ratio 15 → ≥15 ducats/platinum.
    #[arg(long)]
    pub ratio: f32,
}

#[derive(Args)]
pub struct LocationsArgs {
    /// Number of results to display.
    #[arg(long, default_value = "25")]
    pub limit: usize,
}

#[derive(Args)]
pub struct BuyOrdersArgs {
    /// Print the orders that would be created without sending anything.
    #[arg(long)]
    pub dry_run: bool,
}
