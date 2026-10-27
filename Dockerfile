# Broadsheet: a single static Rust binary (frontend embedded, SQLite compiled in) on an empty image.

# 1. The web app (Svelte + TipTap), built to static files.
FROM node:22-alpine AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

# 2. The server, with the built web app embedded.
FROM rust:1-alpine AS build
RUN apk add --no-cache build-base
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY --from=web /web/dist ./web/dist
ENV BROADSHEET_SKIP_WEB=1
RUN cargo build --release --locked
RUN mkdir /data

FROM scratch
COPY --from=build /src/target/release/broadsheet /broadsheet
COPY --from=build --chown=10001:10001 /data /data
USER 10001:10001
ENV DATABASE_PATH=/data/broadsheet.db \
    HOST=0.0.0.0 \
    PORT=8000
EXPOSE 8000
VOLUME ["/data"]
ENTRYPOINT ["/broadsheet"]
