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

1. Launch with `cargo run dev`. Alternatively, if your raspberry pi is configured on the local network as expected, you can run `./deploy.sh`

### (First time Deploy Setup)

1. Create a systemd service file like so:

```
[Unit]
Description=Disbot Service File
After=network.target

[Service]
Type=simple
Restart=always
RestartSec=1
User=<user>
ExecStart=/home/<user>/deploy/disbot prod
WorkingDirectory=/home/<user>/deploy

[Install]
WantedBy=multi-user.target
```

1. `systemctl start disbot`
1. `systemctl enable disbot`
1. logs: `journalctl -u disbot -b -f` (`-b` is current boot filter)

### Gotchas

- Ensure the `SERVER_USER` has sudo-er privileged to run `shutdown` without a password. (Eg: `sudo visudo -> [user]\tALL=NOPASSWD:[pathToBin1],[pathtoBin2],...`)
- Equally, ensure the bot's host can run `ssh` without a password (eg setup it's SSH keys).

## Invite Shruggin' Shiba to Your Server

[Invite Link](https://discord.com/api/oauth2/authorize?client_id=764937518570536990&permissions=342080&scope=bot)

## This Project Uses

- [Serenity](https://github.com/serenity-rs/serenity) for the Discord API
- [rust-musl-cross](https://github.com/messense/rust-musl-cross) for local compilation in docker
