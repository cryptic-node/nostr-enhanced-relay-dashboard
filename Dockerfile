FROM rust:1.87-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
COPY public ./public
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd -m app
WORKDIR /app
COPY --from=builder /app/target/release/nostr-enhanced-relay-dashboard /usr/local/bin/nerd
COPY public ./public
USER app
EXPOSE 8080
CMD ["nerd"]
