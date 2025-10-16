use crate::features::server_specific::config::ServerConfig;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub struct MessageHandler {
    server_config: ServerConfig,
}

impl MessageHandler {
    pub fn new(server_config: ServerConfig) -> Self {
        Self { server_config }
    }

    /// Handles incoming messages and processes bot mentions
    pub async fn handle_message(&self, ctx: &Context, msg: &Message) {
        // Only process messages from our specific server
        if !self.is_from_target_server(msg) {
            return;
        }

        // Check if the bot was mentioned in this message
        if self.is_bot_mentioned(&ctx.cache.current_user(), msg) {
            self.handle_bot_mention(ctx, msg).await;
        }
    }

    /// Checks if the message is from our target server
    fn is_from_target_server(&self, msg: &Message) -> bool {
        msg.guild_id.map(|id| id.get()) == Some(self.server_config.server_id)
    }

    /// Checks if the bot was mentioned in the message
    fn is_bot_mentioned(&self, bot_user: &CurrentUser, msg: &Message) -> bool {
        // Check direct mentions
        if msg.mentions_user_id(bot_user.id) {
            return true;
        }

        // Check if message content contains bot mention
        if let Some(_guild_id) = msg.guild_id {
            let bot_mention = format!("<@{}>", bot_user.id);
            let bot_mention_nickname = format!("<@!{}>", bot_user.id);

            msg.content.contains(&bot_mention) || msg.content.contains(&bot_mention_nickname)
        } else {
            false
        }
    }

    /// Handles bot mention responses
    async fn handle_bot_mention(&self, ctx: &Context, msg: &Message) {
        log::info!(
            "Bot mentioned by user {} in channel {}",
            msg.author.name,
            msg.channel_id
        );

        // Select a random response
        let response = self.get_random_response();

        // Send the response
        if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
            log::error!("Error responding to mention: {}", why);
        } else {
            log::debug!("Successfully responded to bot mention");
        }
    }

    /// Gets a random response for bot mentions
    fn get_random_response(&self) -> &'static str {
        use rand::seq::SliceRandom;

        let responses = ["Cala boca filho da puta", "El Deivid: Un homosexual."];

        responses
            .choose(&mut rand::thread_rng())
            .unwrap_or(&"La inombrable: E")
    }
}
