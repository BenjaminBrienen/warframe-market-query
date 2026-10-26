use std::collections::{
    HashMap,
    HashSet,
};

use anyhow::Result;

use crate::{
    data::DataStore,
    models,
    util::{
        PLATINUM,
        positive_ducats,
        timestamp,
        username_quoted,
    },
};

pub struct Hit {
    pub item_name: String,
    pub platinum: u32,
    pub quantity: u32,
    pub ducats: u32,
}

/// `item_id → (display name, ducats)` for every item that is worth ducats.
pub async fn items_with_ducats(store: &DataStore<'_>) -> Result<HashMap<String, (String, u32)>> {
    let items = store.items().await?;
    let items_by_id = items
        .iter()
        .filter_map(|item| {
            let ducats = positive_ducats(item.ducats)?;
            Some((item.id.clone(), (item.name().to_owned(), ducats)))
        })
        .collect();
    Ok(items_by_id)
}

pub fn order_to_hit(
    items_by_id: &HashMap<String, (String, u32)>,
    order: &models::Order,
    ratio: f32,
) -> Option<Hit> {
    if order.r#type != "sell" {
        return None;
    }
    if order.platinum == 0 {
        return None;
    }
    let (item_name, ducats) = items_by_id.get(&order.item_id)?;
    if (*ducats as f32 / order.platinum as f32) < ratio {
        return None;
    }
    Some(Hit {
        item_name: item_name.clone(),
        platinum: order.platinum.cast_unsigned(),
        quantity: order.quantity.cast_unsigned(),
        ducats: *ducats,
    })
}

pub fn print_user_hits(
    username: &str,
    hits: &[Hit],
) {
    let total_qty: u32 = hits.iter().map(|hit| hit.quantity).sum();
    let total_plat: u32 = hits.iter().map(|hit| hit.platinum * hit.quantity).sum();
    let total_ducats: u32 = hits.iter().map(|hit| hit.ducats * hit.quantity).sum();
    let total_ratio = total_ducats as f32 / total_plat as f32;
    eprintln!(
        "👤 {username} — {total_qty} items, {total_plat}{PLATINUM} total, ~{total_ducats} ducats, {total_ratio} ducats/{PLATINUM}",
        username = username_quoted(username),
    );

    let item_parts: Vec<String> = hits
        .iter()
        .map(|hit| {
            format!(
                "{}x {} ({}{PLATINUM})",
                hit.quantity, hit.item_name, hit.platinum
            )
        })
        .collect();

    let message = format!(
        "/w {} Hi! I'd like to buy: {} (warframe.market via wfmq)",
        username_quoted(username),
        item_parts.join(", ")
    );

    if message.len() > 0x00ff {
        eprintln!(
            "  ⚠  Message is {} chars — Warframe may truncate it.",
            message.len()
        );
    }
    println!("[{}]: {message}", timestamp());
}

pub fn without_bans(
    qualified: &[(String, Vec<Hit>)],
    bans: HashSet<String>,
) -> impl Iterator<Item = &(String, Vec<Hit>)> + '_ {
    qualified.iter().filter(move |item| !bans.contains(&item.0))
}
