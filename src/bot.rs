use std::sync::Arc;

use crate::error::Error;
use crate::Result;
use serenity::client::{Context, EventHandler};
use serenity::http::{CacheHttp, Http};
use serenity::model::id::{GuildId, UserId};
use serenity::model::interactions::Interaction;
use serenity::model::prelude::{Ready, User};
use serenity::prelude::{Mutex, TypeMapKey};
use serenity::{async_trait, Client};
use songbird::{Call, SerenityInit, Songbird};
use tracing::info;
use tracing::instrument;

use crate::commands;
use crate::config::Config;

pub struct OwnerKey;
impl TypeMapKey for OwnerKey {
    type Value = User;
}

#[derive(Debug)]
pub struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, rdy: Ready) {
        info!("{} is ready!", rdy.user.name)
    }

    #[instrument(level = "debug", skip_all)]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Ping(_) => todo!(),
            Interaction::ApplicationCommand(command) => {
                commands::handle_command(ctx, command).await
            }
            Interaction::MessageComponent(_) => todo!(),
        }
    }
}

#[instrument(level = "debug")]
pub async fn initialize(conf: Config) -> Result<Client> {
    let client = Client::builder(conf.token)
        .application_id(conf.app_id)
        .event_handler(Handler)
        .register_songbird()
        .await?;
    let http = client.cache_and_http.http();
    let owner = get_owner(&http).await?;
    {
        let mut data = client.data.write().await;
        data.insert::<OwnerKey>(owner)
    }

    //If TEST_GUILD is set
    if let Some(guild_id) = conf.test_guild {
        let guild = http.get_guild(guild_id).await?;
        commands::register_guild_commands(&http, guild).await?;
    }

    commands::register_global_commands(&http).await?;

    Ok(client)
}

#[instrument(level = "debug", skip(http))]
async fn get_owner(http: &Http) -> Result<User> {
    let app_info = http.get_current_application_info().await?;
    info!("Bot owner is {}", app_info.owner.name);
    Ok(app_info.owner)
}

#[instrument(level = "debug", skip(ctx))]
pub async fn join_user(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<Arc<Mutex<Call>>> {
    let guild = ctx
        .cache
        .guild(guild_id)
        .await
        .expect("Expected guild in cache");
    let channel_id = match guild
        .voice_states
        .get(&user_id)
        .and_then(|vs| vs.channel_id)
    {
        Some(id) => id,
        None => return Err(Error::NotInVoice(user_id).into()),
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird voice not initialized");

    let (call, result) = manager.join(guild_id, channel_id).await;
    result?;
    Ok(call)
}

pub async fn disconnect(ctx: &Context, guild_id: GuildId) -> Result<()> {
    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird initialized at startup")
        .clone();

    match manager.get(guild_id) {
        Some(_call) => manager.remove(guild_id).await?,
        None => return Err(Error::NoCall.into()),
    };

    Ok(())
}

pub async fn get_manager(ctx: &Context) -> Result<Arc<Songbird>> {
    match songbird::get(ctx).await {
        Some(s) => Ok(s),
        None => return Err(Error::MissingDriver.into()),
    }
}

pub async fn get_call(ctx: &Context, guild_id: GuildId) -> Result<Arc<Mutex<Call>>> {
    let manager = get_manager(ctx).await?;
    match manager.get(guild_id) {
        Some(c) => Ok(c),
        None => return Err(Error::NoCall.into()),
    }
}
