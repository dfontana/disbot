#!/bin/bash
set -e

env=${1:-dev}
host=${2:-kossserver.local}

cmd="
cd ~/deploy && docker load -i disbot-$env.tar && docker run --rm -d --name disbot-$env --restart always disbot-$env:latest && docker image prune -fa
"

docker build . --build-arg RUN_ENV=$env -f docker/disbot/disbot.dockerfile -t disbot-$env:latest
docker save -o disbot-$env.tar disbot-$env:latest
ssh $USER@$host 'mkdir -p ~/deploy'
scp "$env.env" $USER@$host:~/deploy/
scp disbot-$env.tar $USER@$host:~/deploy/
rm disbot-$env.tar
ssh $USER@$host "cd ~/deploy && docker load -i disbot-$env.tar && docker run --rm -d --name disbot-$env --restart always disbot-$env:latest && docker image prune -fa"
