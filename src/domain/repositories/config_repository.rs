pub trait ConfigRepository: Send + Sync {
    fn set_notification_channel(&self, guild_id: u64, channel_id: u64);
    fn get_notification_channel(&self, guild_id: u64) -> Option<u64>;
}