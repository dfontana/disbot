set -e

env=${1:-dev}

docker-compose build "disbot-$env"
docker save -o "disbot-$env.tar" "disbot-$env:latest"
ssh $USER@raspberrypi 'mkdir -p ~/deploy'
scp "disbot-$env.tar" $USER@raspberrypi:~/deploy/
scp docker-compose.yaml $USER@raspberrypi:~/deploy/
scp "$env.env" $USER@raspberrypi:~/deploy/
rm "disbot-$env.tar"
ssh $USER@raspberrypi 'eval `ssh-agent` && cd ~/deploy && docker load -i '"disbot-$env.tar"' && docker-compose up -d '"disbot-$env"' && docker image prune -fa'
