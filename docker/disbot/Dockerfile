####################
# 0. Prep Builder 
####################
FROM messense/rust-musl-cross:armv7-musleabihf as builder
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

###################
# 2. Run
###################
# As of writing (3/28/21) the latest version of Alpine (3.13) has a problem where internet fails
# for the raspberrypi which breaks literally everything. See https://gitlab.alpinelinux.org/alpine/aports/-/issues/12091
FROM alpine:3.12
ARG RUN_ENV
RUN apk add --no-cache curl ca-certificates openssh
COPY --from=builder /home/rust/src/cache_build/target/armv7-unknown-linux-musleabihf/release/disbot /app
COPY --from=builder /home/rust/src/cache_build/$RUN_ENV.env /$RUN_ENV.env

ENTRYPOINT ["/app"]
