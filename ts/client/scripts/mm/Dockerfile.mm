FROM debian:bullseye as builder

ARG NODE_VERSION=16.17.0
ARG YARN_VERSION=1.22.19

RUN apt-get update; apt install -y curl
RUN curl https://get.volta.sh | bash
ENV VOLTA_HOME /root/.volta
ENV PATH /root/.volta/bin:$PATH
RUN volta install node@${NODE_VERSION} yarn@${YARN_VERSION}

#######################################################################

RUN mkdir /app
WORKDIR /app

COPY . .

RUN yarn install && yarn run build

FROM debian:bullseye as run

LABEL fly_launch_runtime="nodejs"

COPY --from=builder /root/.volta /root/.volta
COPY --from=builder /app /app

WORKDIR /app
ENV NODE_ENV production
ENV PATH /root/.volta/bin:$PATH

RUN adduser --system --group --no-create-home mangouser
USER mangouser

CMD [ "node", "dist/cjs/src/scripts/mm/market-maker.js" ]