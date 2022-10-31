# syntax = docker/dockerfile:1.2
# Base image containing all binaries, deployed to gcr.io/mango-markets/mango-v4:latest
FROM rust:1.60 as base
RUN cargo install cargo-chef --locked
RUN rustup component add rustfmt
RUN apt-get update && apt-get -y install clang cmake
WORKDIR /app

FROM base as plan
COPY . .
# Hack to prevent a ghost member lib/init
RUN sed -i 's|lib/\*|lib/checked_math|' Cargo.toml
# Hack to prevent local serum_dex manifests conflicting with cargo dependency
RUN rm -rf anchor/tests
RUN cargo chef prepare --recipe-path recipe.json

FROM base as build
COPY --from=plan /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --bin keeper --bin liquidator
COPY . .

FROM debian:bullseye-slim as run
RUN apt-get update && apt-get -y install ca-certificates libc6
COPY --from=build /app/target/release/keeper /usr/local/bin/
COPY --from=build /apptarget/release/liquidator /usr/local/bin/
