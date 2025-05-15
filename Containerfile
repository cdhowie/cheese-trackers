ARG GIT_COMMIT



FROM docker.io/rust:1.87.0-alpine3.21 AS serverbuilder

RUN apk add --no-cache musl-dev openssl-dev

COPY server /app/server
COPY server-macros /app/server-macros
WORKDIR /app/server
RUN --mount=type=cache,target=/app/server/target \
    --mount=type=cache,target=/app/server-macros/target \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release && \
    cp target/release/cheese-trackers-server /app



FROM docker.io/node:20-bookworm AS frontendbuilder

WORKDIR /app
COPY frontend/ /app
RUN npm ci

ARG GIT_COMMIT
ENV VITE_GIT_COMMIT=$GIT_COMMIT

RUN test -n "$VITE_GIT_COMMIT" && npm run build



FROM docker.io/alpine:3.21

RUN apk add --no-cache ca-certificates libssl3 libgcc

WORKDIR /app
COPY --from=serverbuilder /app/cheese-trackers-server /app/
COPY --from=frontendbuilder /app/dist /app/dist
ARG GIT_COMMIT
ENV CT_GIT_COMMIT=$GIT_COMMIT
USER nobody
ENTRYPOINT [ "./cheese-trackers-server" ]
