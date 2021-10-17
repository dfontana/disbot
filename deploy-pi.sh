#!/bin/bash
set -e

env=${1:-dev}
host=${2:-raspberrypi.local}

cross build --release --target armv7-unknown-linux-gnueabihf
ssh $USER@$host 'mkdir -p ~/deploy'
scp "$env.env" $USER@$host:~/deploy/
ssh -t $USER@$host 'sudo systemctl stop '"disbot-$env"
scp target/armv7-unknown-linux-gnueabihf/release/disbot $USER@$host:~/deploy/"disbot-$env"
ssh -t $USER@$host 'sudo systemctl restart '"disbot-$env"
