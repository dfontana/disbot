set -e

env=${1:-dev}
host=${2:-raspberrypi.local}

if [[ $(uname -p) == 'arm' ]]; then
  # we must download from releases
  echo "On ARM machine, downloading release..."
  tmp=$(mktemp -d -t tmp)
  wrk=$(pwd)
  cd $tmp
  curl -s https://api.github.com/repos/dfontana/disbot/releases/latest \
    | jq -r '.assets[].browser_download_url | select(test("tar.gz$"))' \
    | xargs curl -LJO
  tar -xvzf *
  cd $wrk
  mkdir -p target/armv7-unknown-linux-gnueabihf/release
  mv $tmp/disbot target/armv7-unknown-linux-gnueabihf/release/disbot
  rm -r $tmp
else
  # we can cross compile on x86_64
  echo "Cross Compiling..."
  cross build --release --target armv7-unknown-linux-gnueabihf
fi
echo "Binary built."

ssh $USER@$host 'mkdir -p ~/deploy'
scp "$env.env" $USER@$host:~/deploy/
ssh -t $USER@$host 'sudo systemctl stop '"disbot-$env"
scp target/armv7-unknown-linux-gnueabihf/release/disbot $USER@$host:~/deploy/"disbot-$env"
ssh -t $USER@$host 'sudo systemctl restart '"disbot-$env"
