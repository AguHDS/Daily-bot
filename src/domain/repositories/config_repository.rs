use async_trait::async_trait;

#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn set_notification_channel(&self, guild_id: u64, channel_id: u64);
    async fn get_notification_channel(&self, guild_id: u64) -> Option<u64>;
}
