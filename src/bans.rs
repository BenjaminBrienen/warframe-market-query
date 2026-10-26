use std::{
    collections::HashSet,
    fmt::Write as _,
    fs,
    path::PathBuf,
};

use anyhow::Result;

pub fn bans_path() -> PathBuf {
    PathBuf::from("data/bans.txt")
}

pub fn get() -> HashSet<String> {
    let Ok(text) = fs::read_to_string(bans_path()) else {
        return HashSet::default();
    };
    text.lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn add(value: &str) -> Result<()> {
    let mut banned = get();
    banned.insert(value.to_owned());
    let lines = banned.iter().fold(String::new(), |mut accumulator, next| {
        writeln!(accumulator, "{next}").unwrap();
        accumulator
    });
    fs::write(bans_path(), lines)?;
    Ok(())
}

pub fn remove(value: &str) -> Result<()> {
    let mut banned = get();
    banned.remove(value);
    let lines = banned.iter().fold(String::new(), |mut accumulator, next| {
        writeln!(accumulator, "{next}").unwrap();
        accumulator
    });
    fs::write(bans_path(), lines)?;
    Ok(())
}
