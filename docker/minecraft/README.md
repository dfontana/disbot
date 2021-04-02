# Dockerized Minecraft

To utilize the expected file tree is:

```
minecraft/
   |- modpacks/
        |- <server_pack_from_curse>.zip
   |- <specific_server>/
```

After which you can start the server as normal (`docker-compose up ..`) and the server will boot. This is heavily reliant on [docker-minecraft-server](https://github.com/itzg/docker-minecraft-server)
