FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
# Устанавливаем необходимые библиотеки
RUN apt-get update && apt-get install -y libssl3 libsqlite3-0 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/backend /app/backend
COPY --from=builder /app/db.sqlite /app/db.sqlite
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/diesel.toml /app/diesel.toml
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/Cargo.lock /app/Cargo.lock
EXPOSE 8080
CMD ["/app/backend"]