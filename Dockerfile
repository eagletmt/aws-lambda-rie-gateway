FROM public.ecr.aws/docker/library/rust:1.74-alpine as builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/local/src/aws-lambda-rie-gateway

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo 'fn main() {}' > src/bin/dummy.rs && cargo build --release --locked && rm -r src/bin

COPY src ./src/
RUN cargo build --release --locked --frozen

FROM public.ecr.aws/docker/library/alpine:latest

ENV BIND 0.0.0.0:8080

EXPOSE 8080

COPY --from=builder /usr/local/src/aws-lambda-rie-gateway/target/release/aws-lambda-rie-gateway /usr/local/bin/aws-lambda-rie-gateway
ENTRYPOINT ["aws-lambda-rie-gateway"]
