use serenity::framework::standard::macros::group;

mod wake;
mod wol;

use wake::*;

#[group]
#[description = "Game Server Management tools, for turning on, off, and switching servers"]
#[summary = "Sheebs Givith Games"]
#[prefix = "server"]
#[commands(wake)]
pub struct Server;
