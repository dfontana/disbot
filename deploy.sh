ssh $USER@raspberrypi 'mkdir -p ~/deploy'
scp -r docker $USER@raspberrypi:~/deploy/
scp docker-compose.yaml $USER@raspberrypi:~/deploy/
scp -r src $USER@raspberrypi:~/deploy/
ssh $USER@raspberrypi 'cd ~/deploy && docker-compose up -d --build'
