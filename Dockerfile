FROM rust:1.49-alpine as builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/local/src/aws-lambda-rie-gateway

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo 'fn main() {}' > src/bin/dummy.rs && cargo build --release --locked && rm -r src/bin

COPY src ./src/
RUN cargo build --release --locked --frozen

FROM alpine:latest
LABEL org.opencontainers.image.source https://github.com/eagletmt/aws-lambda-rie-gateway

ENV BIND 0.0.0.0:8080

EXPOSE 8080

COPY --from=builder /usr/local/src/aws-lambda-rie-gateway/target/release/aws-lambda-rie-gateway /usr/local/bin/aws-lambda-rie-gateway
ENTRYPOINT ["aws-lambda-rie-gateway"]
