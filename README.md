# DisBot

A Discord Bot, that I'm not sure what it'll do yet - but I wanted to have something that:

- Powered by Rust
- Deployed via Systemctl 
- To a locally running Raspberry Pi

The rest I don't really care about at the moment :shrug-dog:

## Building

For ArmV7 - eg Raspberry Pi. Note: the deploy script will do this for you. 3 Ways:

- __Github Actions__
  - Builds on the remote server whenever you push a tag. You can then download this from releases. See `.github/workflows`
  ```
  ./bin/build_on_git {commit-sha} {message}
  ```
- __Cross__
  - Only works on non-`aarch64` machines. [`cross`](https://github.com/rust-embedded/cross), simply put: 
  ```
  cargo install cross
  cross build --release --target armv7-unknown-linux-gnueabihf
  ```
- __Native Toolchains__
  - Uses [`messense/homebrew-macos-cross-toolchains`](https://github.com/messense/homebrew-macos-cross-toolchains)
  ```
  brew tap messense/macos-cross-toolchains
  brew install armv7-unknown-linux-gnueabihf
  ./bin/build
  ```

## Deploying
 
1. Define a `prod.env` file inside the root of this repo:

```
#prod.env
API_KEY=<Your Bot's Token Here>
APP_ID=<Your Bot's Application Id Here>
EMOTE_NAME=<your-emote || shrug_dog>
EMOTE_USERS=<csv of users || User1,User2,User3>
SERVER_MAC=<game-server-mac>
SERVER_IP=<game-server-ip>
SERVER_DOCKER_PORT=<docker-tcp-port-on-game-server>
SERVER_USER=<game-server-user>
LOG_LEVEL=INFO

#You can repeat this for dev.env as well
```

1. `./deploy.sh {dev|prod} {raspberrypi.local}`
  - You can use github to build the binary by setting `BUILD_GITHUB=1`. This assumes you've already cut the release

### (First time Deploy Setup on Remote Host)

Install required dependencies for the songbird functionality to work:

```
apt install libopus-dev ffmpeg
sudo curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp
sudo chmod a+rx /usr/local/bin/yt-dlp
```

(If you need to update `yt-dlp` use the `-U` flag)

1. Create a systemd service file like so (you might repeat for dev):

```
[Unit]
Description=Disbot Service File
After=network.target

[Service]
Type=simple
Restart=always
RestartSec=1
User=<user>
ExecStart=/home/<user>/deploy/disbot-prod prod
WorkingDirectory=/home/<user>/deploy

[Install]
WantedBy=multi-user.target
```

1. `systemctl start disbot`
1. `systemctl enable disbot`
1. logs: `journalctl -u disbot -b -f` (`-b` is current boot filter)

### Docker interactions

For docker interactions to work over the local network you'll need to edit the systemd service to enable TCP access over the local network:

```
sudo vim /etc/systemd/system/snap.docker.dockerd.service

...
ExecStart=/usr/bin/snap run docker.dockerd -H tcp://0.0.0.0:2375 -H unix:///var/run/docker.sock
```

And then reload the daemon:

```
sudo systemctl daemon-reload
sudo systemctl restart snap.docker.dockerd.service
```

Validate: `curl http://localhost:2375/v1.40/containers/json`. This will need to be repeated each time the snap is updated. Ideally this isn't a problem if the daemon can be configured from outside the snap, however it's unclear if that's plausible at this point in time.

### Gotchas

- Ensure the `SERVER_USER` has sudo-er privileged to run `shutdown` without a password. (Eg: `sudo visudo -> [user]\tALL=NOPASSWD:[pathToBin1],[pathtoBin2],...`)
- Equally, ensure the bot's host can run `ssh` without a password (eg setup it's SSH keys).

## Invite Shruggin' Shiba to Your Server

[Invite Link](https://discord.com/api/oauth2/authorize?client_id=764937518570536990&permissions=342080&scope=bot)

## This Project Uses

- [Serenity](https://github.com/serenity-rs/serenity) for the Discord API
- [cross](https://github.com/rust-embedded/cross) for local compilation in docker
