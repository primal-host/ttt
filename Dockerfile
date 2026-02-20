FROM infra-eridu:latest AS builder
WORKDIR /build
COPY Cargo.toml ./
COPY src/ ./src/
RUN cargo build --release

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/ttt /usr/local/bin/
COPY static/ /app/static/
WORKDIR /app
EXPOSE 3000
CMD ["ttt"]
