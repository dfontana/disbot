set -e

env=${1:-dev}

docker build . -f docker/disbot/Dockerfile -t disbot:latest
id=$(docker create disbot)
docker cp $id:/app disbot
docker rm -v $id
ssh $USER@raspberrypi 'mkdir -p ~/deploy'
scp "$env.env" $USER@raspberrypi:~/deploy/
ssh -t $USER@raspberrypi 'sudo systemctl stop disbot'
scp disbot $USER@raspberrypi:~/deploy/
ssh -t $USER@raspberrypi 'sudo systemctl restart disbot'
rm disbot
