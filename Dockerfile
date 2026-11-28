FROM node:24-bookworm AS frontend
WORKDIR /web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build
FROM rust:1.99-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY migrations migrations
COPY --from=frontend /web/dist/index.html web/dist/index.html
COPY src src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/dbwong /usr/local/bin/
USER 65534:65534
EXPOSE 8080
ENTRYPOINT ["dbwong"]
