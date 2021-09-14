####################
# 0. Prep Builder 
####################
FROM messense/rust-musl-cross:armv7-musleabihf as builder
# Added these apt for cross compile, but it's currently broken
# https://github.com/serenity-rs/songbird/issues/80
RUN apt update
RUN apt install --no-install-recommends -y pkg-config libssl-dev autoconf libtool libopus-dev ffmpeg youtube-dl
RUN USER=root cargo new --bin cache_build
WORKDIR ./cache_build
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --target=armv7-unknown-linux-musleabihf --release
RUN rm src/*.rs

####################
# 1. Build
####################
ADD ./src ./src
ADD *.env ./
RUN rm target/armv7-unknown-linux-musleabihf/release/deps/disbot*
RUN cargo build ---target=armv7-unknown-linux-musleabihf --release


FROM alpine:3.12
COPY --from=builder /home/rust/src/cache_build/target/armv7-unknown-linux-musleabihf/release/disbot /app
ENTRYPOINT ["/app"]
