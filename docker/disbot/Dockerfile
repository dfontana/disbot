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
FROM alpine:latest
COPY --from=builder /home/rust/src/cache_build/target/armv7-unknown-linux-musleabihf/release/disbot /app
COPY --from=builder /home/rust/src/cache_build/prod.env /prod.env
ENTRYPOINT ["/app"]
