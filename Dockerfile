# syntax = docker/dockerfile:1.2
# Base image containing all binaries, deployed to gcr.io/mango-markets/mango-v4:latest
FROM rust:1.65 as base
# RUN cargo install cargo-chef --locked
RUN rustup component add rustfmt
RUN apt-get update && apt-get -y install clang cmake
WORKDIR /app

FROM base as plan
COPY . .
# Hack to prevent a ghost member lib/init
RUN sed -i 's|lib/\*|lib/checked_math|' Cargo.toml
# Hack to prevent local serum_dex manifests conflicting with cargo dependency
RUN rm -rf anchor/tests
# RUN cargo chef prepare --bin keeper --recipe-path recipe-keeper.json
# RUN cargo chef prepare --bin liquidator --recipe-path recipe-liquidator.json

FROM base as build
COPY --from=plan /app/recipe-*.json .
COPY . .
# RUN cargo chef cook --release --recipe-path recipe-keeper.json --bin keeper
# RUN cargo chef cook --release --recipe-path recipe-liquidator.json --bin liquidator
RUN cargo build --release --bin keeper --bin liquidator --bin settler

FROM debian:bullseye-slim as run
RUN apt-get update && apt-get -y install ca-certificates libc6
COPY --from=build /app/target/release/keeper /usr/local/bin/
COPY --from=build /app/target/release/liquidator /usr/local/bin/
COPY --from=build /app/target/release/settler /usr/local/bin/
RUN adduser --system --group --no-create-home mangouser
USER mangouser
