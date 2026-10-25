# Broadsheet: a single static Rust binary (frontend embedded, SQLite compiled in) on an empty image.
FROM rust:1-alpine AS build
RUN apk add --no-cache build-base
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY frontend ./frontend
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
