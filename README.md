# DisBot

A Discord Bot, that I'm not sure what it'll do yet - but I wanted to have something that:

- Powered by Rust
- Deployed via Systemctl 
- To a locally running Raspberry Pi

The rest I don't really care about at the moment :shrug-dog:

## Building (Optional Before Deploying)

For ArmV7 (eg Raspberry Pi) or `x86_64` Linux (eg Linux Server). 

- __Github Actions__
  - Builds on the remote server whenever you push a tag. You can then download this from releases. See `.github/workflows`
  ```
  ./bin/build_on_git {commit-sha} {message}
  ```
  - Note: Running `./bin/deploy` will NOT build this for you, it'll just download the latest release from CI
- __Cross__
  - Only works on non-`aarch64` machines (Eg NOT M1 Macs). [`cross`](https://github.com/rust-embedded/cross), simply put: 
  ```
  cargo install cross
  cross build --release --target armv7-unknown-linux-gnueabihf
  ```
  - Note: Running `./bin/deploy` WILL do this for you
- __Native Toolchains__ (Eg from an aarch64 machine)
  - Uses [`messense/homebrew-macos-cross-toolchains`](https://github.com/messense/homebrew-macos-cross-toolchains)
  ```
  brew tap messense/macos-cross-toolchains
  brew install armv7-unknown-linux-gnueabihf x86_64-unknown-linux-gnu
  ./bin/build_for_arm
  ./bin/build_for_x86_64
  ```
- __Native__ (Eg Machine matches host)
  - When native hosts matches deploy host, can just run `cargo build --release`

## Deploying
 
1. Define a `prod.toml` file inside the root of this repo:

```toml
# prod.toml
api_key = "<Your Bot's Token Here>"
app_id = <Your Bot's Application Id Here>
emote_name = "<your-emote || shrug_dog>"
emote_users = ["User1", "User2", "User3"]
server_mac = "<game-server-mac>"
server_ip = "<game-server-ip>"
server_docker_port = <docker-tcp-port-on-game-server>
server_user = "<game-server-user>"
log_level = "INFO"
voice_channel_timeout_seconds = 600

# You can repeat this for dev.toml as well
```

1. `ARCH={armv7-unknown-linux-gnueabihf|x86_64-unknown-linux-gnu} ./deploy.sh {dev|prod} {server.local|raspberrypi.local}`
  - You can use github to download a release made in CI with `BUILD_GITHUB=1`
  - Otherwise this will detect the correct way to build from your host system


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

### (First time admin UI setup)

If you want to use the admin UI the port exposed needs to be unblocked by the firewall. For Fedora this means the port is listed under `ports` in: `sudo firewall-cmd --list-all`. If not:

```
sudo firewall-cmd --permanent --add-port=3450/tcp
sudo firewall-cmd --reload
```

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

[Invite Link](https://discord.com/api/oauth2/authorize?client_id=764937518570536990&permissions=545430961264&scope=bot%20applications.commands)

## This Project Uses

- [Serenity](https://github.com/serenity-rs/serenity) for the Discord API
- [cross](https://github.com/rust-embedded/cross) for local compilation in docker
