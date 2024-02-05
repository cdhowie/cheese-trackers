FROM docker.io/rust:1.75.0-alpine3.19 AS serverbuilder

RUN apk add --no-cache musl-dev openssl-dev

WORKDIR /app
COPY server/ /app
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release && \
    cp target/release/mw-async-tracker-server .


FROM docker.io/node:20-bullseye AS frontendbuilder

WORKDIR /app
COPY frontend/ /app
RUN npm ci && npm run build

FROM docker.io/alpine:3.19

RUN apk add --no-cache ca-certificates libssl3 libgcc

WORKDIR /app
COPY --from=serverbuilder /app/mw-async-tracker-server /app/
COPY --from=frontendbuilder /app/dist /app/dist
USER nobody
ENTRYPOINT [ "./mw-async-tracker-server" ]
