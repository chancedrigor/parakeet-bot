/*!
 * Implements the `/skip` command.
 *
 * The bot will skip the current track and start playing the next one
 * in the queue (if there is one).
 */

use log::instrument;

use crate::{log, Context, Result};

/// Skips the current audio track.
#[instrument]
#[poise::command(slash_command, guild_only, guild_cooldown = 2)]
pub async fn skip(ctx: Context<'_>) -> Result<()> {
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("expected songbird initialized");
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| log::eyre!("Not in a guild."))?;
    let call = manager
        .get(guild_id)
        .ok_or_else(|| log::eyre!("I'm not in a voice channel."))?
        .clone();

    let call_lock = call.lock().await;
    let queue = call_lock.queue();
    match queue.current() {
        None => Err(log::eyre!("Nothing to skip.")),
        Some(curr_track) => {
            let track_name = curr_track
                .metadata()
                .title
                .clone()
                .unwrap_or_else(|| "???".into());
            ctx.say(format!("Skipped `{track_name}`.")).await?;
            queue.skip()?;
            Ok(())
        }
    }
}
