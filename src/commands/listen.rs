use std::collections::HashMap;

use anyhow::{
    Result,
    anyhow,
};
use futures_util::{
    SinkExt,
    StreamExt,
};
use tokio_tungstenite::tungstenite::{
    Message,
    client::IntoClientRequest,
};

use crate::{
    bans,
    cli::ListenArgs,
    client::RateLimitedClient,
    config::Config,
    data::DataStore,
    hits::items_with_ducats,
    models,
    sound,
    util::{
        PLATINUM,
        timestamp,
    },
};

const WS_URL: &str = "wss://ws.warframe.market/socket";

pub async fn run(
    args: ListenArgs,
    client: &RateLimitedClient,
    config: &Config,
) -> Result<()> {
    let store = DataStore::new(client, config);

    // Build item_id → (display name, ducats) for all prime parts.
    eprintln!("[{}]: 📦 Loading item data...", timestamp());
    let items_by_id = items_with_ducats(&store).await?;

    eprintln!(
        "[{}]: 🪙  {} prime items with ducat values loaded. Ratio filter: ≥{}",
        timestamp(),
        items_by_id.len(),
        args.ratio
    );

    // Build WebSocket handshake request with the mandatory `wfm` sub-protocol.
    let mut request = WS_URL.into_client_request()?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", "wfm".parse()?);

    eprintln!("[{}]: 🔌 Connecting to {WS_URL} ...", timestamp());
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await?;
    eprintln!("[{}]: ✅ Connected. Waiting for orders...\n", timestamp());

    // Subscribe to new-order events.
    let sub = serde_json::json!({
        "route": "@wfm|cmd/subscribe/newOrders",
        "payload": {
            "platform": "pc",
            "crossplay": true
        },
        "id": "wfmq-listen-1"
    });
    ws.send(Message::Text(sub.to_string().into())).await?;

    while let Some(raw) = ws.next().await {
        match listen_one(raw, &items_by_id, args.ratio) {
            Ok(true) => continue,
            Ok(false) | Err(_) => break,
        }
    }

    eprintln!("[{}]: 📴 Disconnected.", timestamp());
    Ok(())
}

/// Handles one WebSocket message. Returns `Ok(false)` when the connection should close.
fn listen_one(
    raw: Result<Message, tokio_tungstenite::tungstenite::Error>,
    items_with_ducats_by_id: &HashMap<String, (String, u32)>,
    filter_ratio: f32,
) -> Result<bool> {
    let raw = match raw {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[{}]: ⚠  WebSocket error: {e}", timestamp());
            return Err(anyhow!("WebSocket error: {e}"));
        },
    };

    // Server sends periodic pings; tungstenite auto-replies with pong.
    let text = match raw {
        Message::Text(text) => text,
        Message::Close(_) => {
            eprintln!("[{}]: 🔌 Server closed the connection.", timestamp());
            return Ok(false);
        },
        _ => return Ok(true),
    };

    let msg: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("[{}]: ⚠️  message text is not JSON: {text}", timestamp());
            return Ok(true);
        },
    };

    // Only handle new-order events.
    let route = msg.get("route").and_then(|r| r.as_str()).unwrap_or("");
    // ignorable messages
    if matches!(
        route,
        "@wfm|event/reports/online" | "@wfm|cmd/subscribe/newOrders:ok"
    ) {
        return Ok(true);
    }
    if !route.contains("newOrder") {
        eprintln!(
            "[{}]: ⚠️  received response from the wrong route: {route}",
            timestamp()
        );
        return Ok(true);
    }

    let Some(payload) = msg.get("payload") else {
        eprintln!("[{}]: ⚠️  response is missing payload: {msg}", timestamp());
        return Ok(true);
    };
    let order: models::OrderWithUser = match serde_json::from_value(payload.clone()) {
        Ok(o) => o,
        Err(_) => {
            eprintln!(
                "[{}]: ⚠️  failed to deserialize response: {payload:?}",
                timestamp()
            );
            return Ok(true);
        },
    };

    // --- Filters ---
    if order.r#type != "sell" {
        return Ok(true);
    }
    // Accept both spellings — docs show "in_game" but REST uses "ingame".
    let status = order.user.status.as_str();
    if status != "ingame" && status != "in_game" {
        return Ok(true);
    }

    if bans::get().contains(&order.user.ingame_name) {
        return Ok(true);
    }

    // early return if item is not a prime part with a ducat value
    let Some((item_name, ducats)) = items_with_ducats_by_id.get(&order.item_id) else {
        return Ok(true);
    };
    // avoid divide by zero, should not actually happen
    if order.platinum == 0 {
        eprintln!("[{}]: ⚠️  order somehow is for 0 platinum", timestamp());
        return Ok(true);
    }

    let order_ratio = *ducats as f32 / order.platinum as f32;
    if order_ratio < filter_ratio {
        // not worth it
        return Ok(true);
    }

    // --- Output ---
    let msg_out = format!(
        "/w {} Hi! I'd like to buy: 1x {} ({}{PLATINUM}) (warframe.market via wfmq)",
        order.user.ingame_name, item_name, order.platinum,
    );

    sound::play();

    println!(
        "🔔 {} | {}{PLATINUM} | {:.0} ducats/{PLATINUM} | x{} available | {}",
        item_name, order.platinum, order_ratio, order.quantity, order.user.ingame_name,
    );
    println!("[{}]:    {msg_out}\n", timestamp());
    Ok(true)
}
