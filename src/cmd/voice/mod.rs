use serenity::framework::standard::macros::group;

mod play;
mod skip;
mod stop;

use play::*;
use skip::*;
use stop::*;

#[group]
#[description = "Stream sound to the channel"]
#[summary = "Sheebs Givith Loud Noises"]
#[prefix = "p"]
#[only_in(guilds)]
#[default_command(play)]
#[commands(skip, stop)]
pub struct Voice;
