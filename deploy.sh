docker-compose build disbot
docker save -o disbot.tar disbot:latest
ssh $USER@raspberrypi 'mkdir -p ~/deploy'
scp disbot.tar $USER@raspberrypi:~/deploy/
scp docker-compose.yaml $USER@raspberrypi:~/deploy/
scp prod.env $USER@raspberrypi:~/deploy/
rm disbot.tar
ssh $USER@raspberrypi 'cd ~/deploy && docker load -i disbot.tar && docker-compose up -d && docker image prune -fa'
