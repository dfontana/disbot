set -e

env=${1:-dev}
host=${2:-raspberrypi}

# Build on remote
ssh $USER@$host 'mkdir -p ~/build' 
scp -r src/ $USER@$host:~/build
scp Cargo.* $USER@$host:~/build
ssh $USER@$host 'cd ~/build && '"/home/$USER/"'.cargo/bin/cargo build --release'

# Deploy
ssh $USER@$host 'mkdir -p ~/deploy'
scp "$env.env" $USER@$host:~/deploy/
ssh -t $USER@$host 'sudo systemctl stop '"disbot-$env"
ssh $USER@$host 'mv ~/build/target/release/disbot ~/deploy/'"disbot-$env"
ssh -t $USER@$host 'sudo systemctl restart '"disbot-$env"
