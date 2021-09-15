use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
  let guild = msg.guild(&ctx.cache).await.unwrap();
  let guild_id = guild.id;

  let manager = songbird::get(ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

  match manager.get(guild_id) {
    None => {
      let _ = msg
        .channel_id
        .say(&ctx.http, "Not in a voice channel to play in")
        .await;
    }
    Some(handler_lock) => {
      let handler = handler_lock.lock().await;
      let queue = handler.queue();
      let _ = queue.skip();
      let reply = format!("Song skipped: {} in queue.", queue.len());
      let _res = msg.channel_id.say(&ctx.http, reply).await;
    }
  }
  Ok(())
}
