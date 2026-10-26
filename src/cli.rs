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
    Update(UpdateArguments),

    /// Search sell/buy orders by price, quantity, and item tags.
    Search(SearchArguments),

    /// Find sellers who have enough ducat-efficient items to justify one trade slot.
    Ducats(DucatsArguments),

    /// Show the best mission locations/rotations for farming ducats.
    /// Requires the cache to be populated first (`wfmq update`).
    Locations(LocationsArguments),

    /// Connect to the warframe.market WebSocket and print matching orders live.
    Listen(ListenArguments),

    /// Find out what is worth trading from the user for ducats.
    UserDucats(UserArguments),

    /// Create a buy order (qty 99) for every ducat item:
    /// 45 ducats → 2p, 100 ducats → 5p, more than 100 ducats → 6p.
    BuyOrders(BuyOrdersArguments),

    /// Test sound effect.
    PlaySound(()),

    /// Ban a user (do not want to trade with them).
    Ban(BanArguments),

    /// Unban a user.
    Unban(BanArguments),
}

#[derive(Args)]
pub struct BanArguments {
    /// Username of the user to ban.
    pub username: String,
}

#[derive(Args)]
pub struct UpdateArguments {
    /// Re-fetch dropsource files even when they are already cached.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ListenArguments {
    /// Minimum ducats-per-platinum ratio a live order must meet to be reported.
    #[arg(long)]
    pub ratio: f32,
}

#[derive(Args)]
pub struct SearchArguments {
    #[arg(long = "type", default_value = "sell")]
    pub r#type: String,

    /// Maximum platinum price (inclusive).
    #[arg(long)]
    pub platinum: Option<u32>,

    /// Minimum quantity (inclusive).
    #[arg(long)]
    pub quantity: Option<u32>,

    /// Comma-separated item tags that must ALL match. For example: `--tags prime,relic`.
    #[arg(long)]
    pub tags: Option<String>,

    #[clap(long, short, action)]
    pub include_offline: bool,
}

#[derive(Args)]
pub struct DucatsArguments {
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
pub struct UserArguments {
    /// Username (slug) of the user. Also the "profile".
    #[arg(long)]
    pub username: String,

    /// Minimum ducats-per-platinum ratio.  E.g. --ratio 15 → ≥15 ducats/platinum.
    #[arg(long)]
    pub ratio: f32,
}

#[derive(Args)]
pub struct LocationsArguments {
    /// Number of results to display.
    #[arg(long, default_value = "25")]
    pub limit: usize,
}

#[derive(Args)]
pub struct BuyOrdersArguments {
    /// Print the orders that would be created without sending anything.
    #[arg(long)]
    pub dry_run: bool,
}
