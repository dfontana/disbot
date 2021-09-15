use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
};

#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
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
      return Ok(());
    }
    Some(handler_lock) => {
      let handler = handler_lock.lock().await;
      let queue = handler.queue();
      let _ = queue.stop();
    }
  }
  let _dc = manager.remove(guild_id).await;
  let _rep = msg.channel_id.say(&ctx.http, "Cya later.").await;

  Ok(())
}
