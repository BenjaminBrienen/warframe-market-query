use anyhow::Result;

use crate::{
    bans,
    cli::BanArguments,
};

pub fn ban(arguments: &BanArguments) -> Result<()> {
    bans::add(&arguments.username)
}

pub fn unban(arguments: &BanArguments) -> Result<()> {
    bans::remove(&arguments.username)
}
