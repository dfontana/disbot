# DisBot

A Discord Bot, that I'm not sure what it'll do yet - but I wanted to have something that:

- Powered by Go
- Deployed via Docker
- To a locally running Raspberry Pi

The rest I don't really care about at the moment :shrug-dog:

## Deploying

1. Clone the project to your deploy location
1. Define a `prod.env` file inside the `docker/disbot/` folder:

```
#prod.env
API_KEY=<Your Bot's Token Here>
EMOTE_NAME=<your-emote || shrug_dog>
EMOTE_USERS=<csv of users || User1,User2,User3>
```

1. Launch with `docker-compose up --build`

## Invite Shruggin' Shiba to Your Server

[Invite Link](https://discord.com/api/oauth2/authorize?client_id=764937518570536990&permissions=342080&scope=bot)

## This Project Uses

- [Wire](https://github.com/google/wire) for compile time DI
- [DiscordGo](https://github.com/bwmarrin/discordgo) for the Discord API
