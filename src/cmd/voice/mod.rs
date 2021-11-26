use serenity::framework::standard::macros::group;

mod connect_util;
mod list;
mod play;
mod reorder;
mod skip;
mod stop;

use list::*;
use play::*;
use reorder::*;
use skip::*;
use stop::*;

#[group]
#[description = "Stream sound to the channel"]
#[summary = "Sheebs Givith Loud Noises"]
#[prefixes("p", "play")]
#[only_in(guilds)]
#[default_command(play)]
#[commands(skip, stop, list, reorder)]
pub struct Voice;
