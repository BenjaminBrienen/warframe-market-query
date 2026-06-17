//! Raw API calls — no caching, no business logic.
//! All requests go through the rate-limited client.

use anyhow::Result;

use crate::{
    client::RateLimitedClient,
    models::{
        ApiResponse, DropSource, ItemShort, Location, Mission, OrderWithUser, V1DropSourcesResponse,
    },
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
pub async fn get_dropsources(client: &RateLimitedClient, slug: &str) -> Result<Vec<DropSource>> {
    let resp = client
        .get(&format!("/v1/items/{slug}/dropsources"))
        .await?
        .json::<V1DropSourcesResponse>()
        .await?;
    Ok(resp.payload.dropsources)
}

pub async fn get_orders(client: &RateLimitedClient, slug: &str) -> Result<Vec<OrderWithUser>> {
    Ok(client
        .get(&format!("/v2/orders/item/{slug}"))
        .await?
        .json::<ApiResponse<_>>()
        .await?
        .data)
}
