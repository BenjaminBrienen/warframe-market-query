use anyhow::Result;

use crate::{
    bans,
    cli::BanArgs,
};

pub fn ban(args: BanArgs) -> Result<()> {
    bans::add(&args.username)
}

pub fn unban(args: BanArgs) -> Result<()> {
    bans::remove(&args.username)
}
