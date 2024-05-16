ARG GIT_COMMIT

FROM docker.io/rust:1.78.0-alpine3.19 AS serverbuilder

RUN apk add --no-cache musl-dev openssl-dev

ARG GIT_COMMIT
ENV GIT_COMMIT=$GIT_COMMIT

WORKDIR /app
COPY server/ /app
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    test -n "$GIT_COMMIT" && \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release && \
    cp target/release/cheese-trackers-server .


FROM docker.io/node:20-bullseye AS frontendbuilder

WORKDIR /app
COPY frontend/ /app
RUN npm ci

ARG GIT_COMMIT
ENV VITE_GIT_COMMIT=$GIT_COMMIT

RUN test -n "$VITE_GIT_COMMIT" && npm run build

FROM docker.io/alpine:3.19

RUN apk add --no-cache ca-certificates libssl3 libgcc

WORKDIR /app
COPY --from=serverbuilder /app/cheese-trackers-server /app/
COPY --from=frontendbuilder /app/dist /app/dist
USER nobody
ENTRYPOINT [ "./cheese-trackers-server" ]
