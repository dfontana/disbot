# DisBot

A Discord Bot, that I'm not sure what it'll do yet - but I wanted to have something that:

- Powered by Rust
- Deployed via Docker
- To a locally running Raspberry Pi

The rest I don't really care about at the moment :shrug-dog:

## Deploying

1. Clone the project to your deploy location
1. Define a `prod.env` file inside the the root of this repo:

```
#prod.env
API_KEY=<Your Bot's Token Here>
EMOTE_NAME=<your-emote || shrug_dog>
EMOTE_USERS=<csv of users || User1,User2,User3>
SERVER_MAC=<game-server-mac>
SERVER_IP=<game-server-ip>
SERVER_USER=<game-server-user>

#You can repeat this for dev.env as well
```

1. Launch with `docker-compose up --build` or with `cargo run dev`. Alternatively, if your raspberry pi is configured on the local network as expected, you can run `./deploy.sh`

## Invite Shruggin' Shiba to Your Server

[Invite Link](https://discord.com/api/oauth2/authorize?client_id=764937518570536990&permissions=342080&scope=bot)

## This Project Uses

- [Serenity](https://github.com/serenity-rs/serenity) for the Discord API
- [rust-musl-cross](https://github.com/messense/rust-musl-cross) for local compilation in docker
