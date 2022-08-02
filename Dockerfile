FROM rust:1.60 as build

# ENV NODE_VERSION   16.13.1
# ENV SOLANA_VERSION 1.9.13
# ENV ANCHOR_VERSION 0.24.2
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
RUN ls .bin
FROM debian:buster-slim as run
COPY --from=build /app/.bin/* /usr/local/bin/

