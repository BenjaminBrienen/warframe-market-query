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
use clap::Parser as _;
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

    match &cli.command {
        Commands::Update(arguments) => commands::update::run(arguments, &client).await,
        Commands::Search(arguments) => commands::search::run(arguments, &client, &config).await,
        Commands::Ducats(arguments) => commands::ducats::run(arguments, &client, &config).await,
        Commands::Locations(arguments) => {
            commands::locations::run(arguments, &client, &config).await
        },
        Commands::Listen(arguments) => commands::listen::run(arguments, &client, &config).await,
        Commands::UserDucats(arguments) => {
            commands::user_ducats::run(arguments, &client, &config).await
        },
        Commands::BuyOrders(arguments) => {
            commands::buy_orders::run(arguments, &client, &config).await
        },
        Commands::PlaySound(()) => {
            sound::play();
            Ok(())
        },
        Commands::Ban(arguments) => commands::ban::ban(arguments),
        Commands::Unban(arguments) => commands::ban::unban(arguments),
    }
}
