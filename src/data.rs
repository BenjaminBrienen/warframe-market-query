//! Cache-first data access layer.
//!
//! Every method checks the local cache first.  Only on a cache miss (or a
//! stale orders file) does it hit the API, burning a rate-limit slot.
//!
//! Static game data (items, locations, missions, dropsources) has no TTL —
//! it only changes when the game is updated, so `wfmq update` is the
//! explicit refresh mechanism.
//!
//! Orders are dynamic and expire after `config.orders_cache_minutes`.

use anyhow::Result;

use crate::{
    api,
    cache,
    client::RateLimitedClient,
    config::Config,
    models::{
        DropSource,
        ItemShort,
        Location,
        Mission,
        Order,
        OrderWithUser,
        User,
    },
};

pub struct DataStore<'state> {
    client: &'state RateLimitedClient,
    config: &'state Config,
}

impl<'state> DataStore<'state> {
    pub const fn new(
        client: &'state RateLimitedClient,
        config: &'state Config,
    ) -> Self {
        Self { client, config }
    }

    // Static data

    pub async fn items(&self) -> Result<Vec<ItemShort>> {
        let path = cache::items_path();
        if let Some(cached) = cache::read(&path) {
            return Ok(cached);
        }
        let data = api::get_items(self.client).await?;
        drop(cache::write(&path, &data));
        Ok(data)
    }

    pub async fn locations(&self) -> Result<Vec<Location>> {
        let path = cache::locations_path();
        if let Some(cached) = cache::read(&path) {
            return Ok(cached);
        }
        let data = api::get_locations(self.client).await?;
        drop(cache::write(&path, &data));
        Ok(data)
    }

    pub async fn missions(&self) -> Result<Vec<Mission>> {
        let path = cache::missions_path();
        if let Some(cached) = cache::read(&path) {
            return Ok(cached);
        }
        let data = api::get_missions(self.client).await?;
        drop(cache::write(&path, &data));
        Ok(data)
    }

    /// Loads dropsource data for one item from cache only.
    /// Returns `None` if no cache file exists (caller should prompt `update`).
    #[expect(clippy::unused_self, reason = "intentional API")]
    pub fn dropsources_cached(
        &self,
        slug: &str,
    ) -> Option<Vec<DropSource>> {
        cache::read(&cache::dropsources_path(slug))
    }

    // Dynamic data (TTL-based)

    pub async fn orders_by_item(
        &self,
        slug: &str,
    ) -> Result<Vec<OrderWithUser>> {
        let path = cache::orders_item_path(slug);
        if cache::is_fresh(&path, self.config.orders_cache_minutes)
            && let Some(cached) = cache::read(&path)
        {
            return Ok(cached);
        }
        let data = api::get_orders_by_item(self.client, slug).await?;
        drop(cache::write(&path, &data));
        Ok(data)
    }

    pub async fn orders_by_user(
        &self,
        slug: &str,
    ) -> Result<Vec<Order>> {
        let path = cache::orders_user_path(slug);
        if cache::is_fresh(&path, self.config.orders_cache_minutes)
            && let Some(cached) = cache::read(&path)
        {
            return Ok(cached);
        }
        let data = api::get_orders_by_user(self.client, slug).await?;
        drop(cache::write(&path, &data));
        Ok(data)
    }

    pub async fn user(
        &self,
        slug: &str,
    ) -> Result<User> {
        let path = cache::user_path(slug);
        if cache::is_fresh(&path, self.config.orders_cache_minutes)
            && let Some(cached) = cache::read(&path)
        {
            return Ok(cached);
        }
        let data = api::get_user(self.client, slug).await?;
        drop(cache::write(&path, &data));
        Ok(data)
    }
}
