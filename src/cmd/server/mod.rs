use serenity::framework::standard::macros::group;

mod start;
mod stop;
mod wol;

use start::*;
use stop::*;
pub use wol::configure;

#[group]
#[description = "Game Server Management tools, for turning on, off, and switching servers"]
#[summary = "Sheebs Givith Games"]
#[prefix = "server"]
#[commands(stop, start)]
pub struct Server;
