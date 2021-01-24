use crate::emoji::EmojiLookup;
use rand::Rng;
use serenity::{
  client::Context,
  framework::standard::{macros::command, Args, CommandResult},
  model::channel::Message,
  utils::MessageBuilder,
};

#[command]
#[description = "Roll a die, optionally between the given bounds"]
#[usage = "[lower] [upper]"]
#[example = "will roll 1-100"]
#[example = "20 will roll 1-20"]
#[example = "100 100 will roll 100-100"]
#[min_args(0)]
#[max_args(2)]
async fn roll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
  let (lower, upper) = validate(args)?;
  let roll = rand::thread_rng().gen_range(lower..upper + 1);
  let guild_id = match msg.guild_id {
    Some(id) => id,
    None => return Ok(()),
  };
  let emoji = EmojiLookup::inst().get(guild_id, &ctx.cache).await?;
  let response = MessageBuilder::new()
    .push("rolls ")
    .push(" ")
    .mention(&emoji)
    .push(" ")
    .push_bold(roll)
    .push(" ")
    .mention(&emoji)
    .push(" ")
    .push_mono(format!("({} - {})", lower, upper))
    .build();
  msg.reply_mention(&ctx.http, response).await?;
  Ok(())
}

fn validate(mut args: Args) -> Result<(u32, u32), String> {
  match args.remaining() {
    0 => Ok((1, 100)),
    1 => match args.single::<u32>() {
      Ok(upper) => Ok((1, upper)),
      Err(_) => Err("Could not parse lower bound".to_string()),
    },
    2 => {
      let lower = match args.single::<u32>() {
        Ok(lower) => lower,
        Err(_) => return Err("Could not parse lower bound".to_string()),
      };
      let upper = match args.single::<u32>() {
        Ok(upper) => upper,
        Err(_) => return Err("Could not parse upper bound".to_string()),
      };
      Ok((lower, upper))
    }
    _ => Err("Too many arugments provided".to_string()),
  }
}
