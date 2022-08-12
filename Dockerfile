# Base image containing all binaries, deployed to gcr.io/mango-markets/mango-v4:latest
FROM rust:1.60 as build
RUN apt-get update && apt-get -y install clang cmake

WORKDIR /app
COPY ./ .

# Hack to prevent a ghost member lib/init
RUN sed -i 's|lib/\*|lib/checked_math|' Cargo.toml

# Mount cache for downloaded and compiled dependencies
RUN --mount=type=cache,target=/usr/local/cargo,from=rust,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release --bins

# Copy bins out of cache
RUN --mount=type=cache,target=target mkdir .bin && cp target/release/keeper target/release/liquidator .bin/

FROM debian:buster-slim as run
RUN apt-get update && apt-get -y install ca-certificates libc6
COPY --from=build /app/.bin/* /usr/local/bin/
