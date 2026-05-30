use anyhow::Result;

use crate::client::RateLimitedClient;
use crate::models::{ApiResponse, ItemShort, OrderWithUser};

/// Fetch the full tradable-item list from /v2/items.
pub async fn get_items(client: &RateLimitedClient) -> Result<Vec<ItemShort>> {
    let resp = client.get("/v2/items").await?;
    let body: ApiResponse<Vec<ItemShort>> = resp.json().await?;
    Ok(body.data)
}

/// Fetch all orders for one item (users active within the last 7 days).
pub async fn get_orders(
    client: &RateLimitedClient,
    slug: &str,
) -> Result<Vec<OrderWithUser>> {
    let path = format!("/v2/orders/item/{slug}");
    let resp = client.get(&path).await?;
    let body: ApiResponse<Vec<OrderWithUser>> = resp.json().await?;
    Ok(body.data)
}
