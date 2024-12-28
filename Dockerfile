FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM ubuntu

WORKDIR /usr/local/bin
COPY --from=builder /usr/src/app/target/release/customer_agent .
COPY --from=builder /usr/src/app/static /usr/local/bin/static

ENV REDIS_URL=redis://redis:6379

EXPOSE 3000

CMD ["customer_agent"]
