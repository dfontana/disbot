set -e

env=${1:-dev}
host=${2:-raspberrypi}

docker build . -f docker/disbot/Dockerfile -t disbot:latest
id=$(docker create disbot)
docker cp $id:/app disbot
docker rm -v $id
ssh $USER@$host 'mkdir -p ~/deploy'
scp "$env.env" $USER@$host:~/deploy/
ssh -t $USER@$host 'sudo systemctl stop '"disbot-$env"
scp disbot $USER@$host:~/deploy/"disbot-$env"
ssh -t $USER@$host 'sudo systemctl restart '"disbot-$env"
rm disbot
