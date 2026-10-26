//! Raw API calls — no caching, no business logic.
//! All requests go through the rate-limited client.

use anyhow::{
    Result,
    bail,
};
use serde::Serialize;

use crate::{
    client::RateLimitedClient,
    config::Config,
    models::*,
};

pub async fn get_items(client: &RateLimitedClient) -> Result<Vec<ItemShort>> {
    Ok(client
        .get("/v2/items")
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}

pub async fn get_locations(client: &RateLimitedClient) -> Result<Vec<Location>> {
    Ok(client
        .get("/v2/locations")
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}

pub async fn get_missions(client: &RateLimitedClient) -> Result<Vec<Mission>> {
    Ok(client
        .get("/v2/missions")
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}

/// Dropsources live on v1, which uses a different response envelope.
pub async fn get_dropsources(
    client: &RateLimitedClient,
    slug: &str,
) -> Result<Vec<DropSource>> {
    let response = client
        .get(&format!("/v1/items/{slug}/dropsources"))
        .await?
        .json::<V1DropSourcesResponse>()
        .await?;
    Ok(response.payload.dropsources)
}

pub async fn get_orders_by_item(
    client: &RateLimitedClient,
    slug: &str,
) -> Result<Vec<OrderWithUser>> {
    Ok(client
        .get(&format!("/v2/orders/item/{slug}"))
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}

pub async fn get_orders_by_user(
    client: &RateLimitedClient,
    slug: &str,
) -> Result<Vec<Order>> {
    Ok(client
        .get(&format!("/v2/orders/user/{slug}"))
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}

pub async fn get_user(
    client: &RateLimitedClient,
    slug: &str,
) -> Result<User> {
    Ok(client
        .get(&format!("/v2/user/{slug}"))
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrder<'a> {
    item_id: &'a str,
    #[serde(rename = "type")]
    r#type: &'a str,
    platinum: u32,
    quantity: u32,
    visible: bool,
}

/// What happened when we tried to create an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderOutcome {
    Created,
    /// warframe.market refused it because we already have an order for this
    /// item at this price, which is what we wanted anyway.
    AlreadyExists,
    /// User has hit the open order limit.
    ExceededOrderLimit,
}

/// Error code returned when an identical order is already on the account.
const SAME_PRICE_ERROR: &str = "exceededOrderLimitSamePrice";
const ORDER_LIMIT_ERROR: &str = "exceededOrderLimit";

/// Creates a visible buy order for `item_id`. Requires a token in
/// `auth.txt` (as `JWT:"<token>"`).
pub async fn create_buy_order(
    client: &RateLimitedClient,
    config: &Config,
    item_id: &str,
    platinum: u32,
    quantity: u32,
) -> Result<OrderOutcome> {
    let Some(token) = config.auth_token() else {
        bail!("no auth token: put JWT:\"<token>\" in auth.txt");
    };

    let body = NewOrder {
        item_id,
        r#type: "buy",
        platinum,
        quantity,
        visible: true,
    };
    let response = client.post_json("/v2/order", &token, &body).await?;

    let status = response.status();
    if status.is_success() {
        return Ok(OrderOutcome::Created);
    }
    let text = response.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::FORBIDDEN && text.contains(SAME_PRICE_ERROR) {
        return Ok(OrderOutcome::AlreadyExists);
    }
    if status == reqwest::StatusCode::FORBIDDEN && text.contains(ORDER_LIMIT_ERROR) {
        return Ok(OrderOutcome::ExceededOrderLimit);
    }
    bail!("creating order failed: {status}: {text}");
}
