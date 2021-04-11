use serenity::framework::standard::macros::group;

mod list;
mod start;
mod stop;
mod wol;

use list::*;
use start::*;
use stop::*;
pub use wol::configure;

#[group]
#[description = "Game Server Management tools, for turning on, off, and switching servers"]
#[summary = "Sheebs Givith Games"]
#[prefix = "server"]
#[commands(list, stop, start)]
pub struct Server;
