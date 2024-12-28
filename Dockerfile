FROM debian:bookworm-slim as base

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl gcc make build-essential && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*

FROM base as builder
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    . "$HOME/.cargo/env" && rustup install stable && rustup default stable

WORKDIR /usr/src/app
COPY . .

RUN . "$HOME/.cargo/env" && cargo build --release

FROM base
WORKDIR /usr/local/bin
COPY --from=builder /usr/src/app/target/release/customer_agent .
COPY --from=builder /usr/src/app/static /usr/local/bin/static

EXPOSE 3000

CMD ["customer_agent"]
