# syntax = docker/dockerfile:1.2
# Base image containing all binaries, deployed to ghcr.io/blockworks-foundation/mango-v4:latest
FROM lukemathwalker/cargo-chef:latest-rust-1.70-slim-bullseye as base
RUN apt-get update && apt-get -y install clang cmake perl libfindbin-libs-perl
WORKDIR /app

FROM base as plan
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base as build
COPY --from=plan /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bins

FROM debian:bullseye-slim as run
RUN apt-get update && apt-get -y install ca-certificates libc6

COPY --from=build /app/target/release/keeper /usr/local/bin/
COPY --from=build /app/target/release/liquidator /usr/local/bin/
COPY --from=build /app/target/release/settler /usr/local/bin/

COPY --from=build /app/target/release/service-mango-* /usr/local/bin/
COPY --from=build /app/bin/service-mango-pnl/conf/template-config.toml ./pnl-config.toml
COPY --from=build /app/bin/service-mango-fills/conf/template-config.toml ./fills-config.toml
COPY --from=build /app/bin/service-mango-orderbook/conf/template-config.toml ./orderbook-config.toml
COPY --from=build /app/bin/service-mango-health/conf/template-config.toml ./health-config.toml

COPY --from=build /app/bin/service-mango-pnl/conf/template-config.toml ./pnl-config.toml
COPY --from=build /app/bin/service-mango-fills/conf//template-config.toml ./fills-config.toml
COPY --from=build /app/bin/service-mango-orderbook/conf/template-config.toml ./orderbook-config.toml

RUN adduser --system --group --no-create-home mangouser
USER mangouser
