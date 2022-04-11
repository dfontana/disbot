## Setup the build dependencies; eg a dedicated user etc
FROM rust:latest as compiler
RUN apt-get update && apt-get install -y cmake make clang gcc curl
RUN update-ca-certificates
ENV USER=myuser
ENV UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

## Build the actual artifact, this is the costly step
FROM compiler as builder
ARG name
ARG target=x86_64-unknown-linux-gnu
RUN echo "Building for Target:" "$target"
RUN USER=myuser cargo new --bin cache_build
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o yt-dlp
WORKDIR /cache_build
COPY src/ ./src
COPY Cargo.* ./
RUN cargo build --target="$target" --release
RUN mv /cache_build/target/$target/release/$name ./bot

## Similarly, we'll setup the runtime dependencies as a cached layer
FROM debian:bullseye-slim as runtime_cache
RUN apt-get update && apt-get install -y ffmpeg python3
RUN ldd --version
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group
COPY --from=builder /cache_build/yt-dlp /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp

## Using the cached runtime image, we'll copy over the built artifact for final running
FROM runtime_cache 
ARG env
WORKDIR /app
COPY --from=builder /cache_build/bot . 
COPY $env.env .
USER myuser:myuser
ENV RUN_ENV="$env"
CMD /app/bot ${RUN_ENV}

