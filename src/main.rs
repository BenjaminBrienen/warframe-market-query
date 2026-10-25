mod analysis;
mod api;
mod bans;
mod cache;
mod cli;
mod client;
mod commands;
mod config;
mod data;
mod hits;
mod models;
mod sound;
mod util;

use anyhow::Result;
use clap::Parser;
use cli::{
    Cli,
    Commands,
};
use client::RateLimitedClient;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load();
    let client = RateLimitedClient::new();

    match cli.command {
        Commands::Update(args) => commands::update::run(args, &client).await,
        Commands::Search(args) => commands::search::run(args, &client, &config).await,
        Commands::Ducats(args) => commands::ducats::run(args, &client, &config).await,
        Commands::Locations(args) => commands::locations::run(args, &client, &config).await,
        Commands::Listen(args) => commands::listen::run(args, &client, &config).await,
        Commands::UserDucats(args) => commands::user_ducats::run(args, &client, &config).await,
        Commands::BuyOrders(args) => commands::buy_orders::run(args, &client, &config).await,
        Commands::PlaySound(()) => {
            sound::play();
            Ok(())
        },
        Commands::Ban(args) => commands::ban::ban(args),
        Commands::Unban(args) => commands::ban::unban(args),
    }
}
