FROM infra-eridu:latest AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build WASM package
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-pack
RUN wasm-pack build --target web --release --out-dir static/pkg -- --no-default-features --features wasm

# Build server binary
COPY static/ ./static/
RUN cargo build --release

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/ttt /usr/local/bin/
COPY --from=builder /build/static/ /app/static/
WORKDIR /app
EXPOSE 3000
CMD ["ttt"]
