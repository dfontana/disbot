use std::collections::HashSet;

use serenity::{
  client::Context,
  framework::standard::{
    help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
  },
  model::{channel::Message, id::UserId},
};

#[help]
#[individual_command_tip = "Pass the name of a command to learn more"]
#[command_not_found_text = "Could not find: `{}`"]
#[strikethrough_commands_tip_in_guild = ""]
#[lacking_permissions = "Hide"]
#[lacking_role = "Hide"]
#[wrong_channel = "Hide"]
async fn help(
  context: &Context,
  msg: &Message,
  args: Args,
  help_options: &'static HelpOptions,
  groups: &[&'static CommandGroup],
  owners: HashSet<UserId>,
) -> CommandResult {
  let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
  Ok(())
}
