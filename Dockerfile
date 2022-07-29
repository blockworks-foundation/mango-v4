FROM debian:buster-slim

COPY target/release/keeper /usr/local/bin
COPY target/release/liquidator /usr/local/bin

CMD ["keeper"]