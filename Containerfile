ARG GIT_COMMIT

FROM docker.io/rust:1.75.0-alpine3.19 AS serverbuilder

ARG GIT_COMMIT
ENV GIT_COMMIT=$GIT_COMMIT

RUN test -n "$GIT_COMMIT" && apk add --no-cache musl-dev openssl-dev

WORKDIR /app
COPY server/ /app
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release && \
    cp target/release/cheese-trackers-server .


FROM docker.io/node:20-bullseye AS frontendbuilder

ARG GIT_COMMIT
ENV GIT_COMMIT=$GIT_COMMIT

WORKDIR /app
COPY frontend/ /app
RUN test -n "$GIT_COMMIT" && npm ci && npm run build && rm -fr node_modules ~/.npm

FROM docker.io/alpine:3.19

RUN apk add --no-cache ca-certificates libssl3 libgcc

WORKDIR /app
COPY --from=serverbuilder /app/cheese-trackers-server /app/
COPY --from=frontendbuilder /app/dist /app/dist
USER nobody
ENTRYPOINT [ "./cheese-trackers-server" ]
