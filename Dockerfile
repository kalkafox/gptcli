FROM kalka:docker-cargo

RUN apk add --no-cache build-base pkgconfig openssl-dev

WORKDIR /build

COPY . .

RUN rustup target add x86_64-unknown-linux-musl

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine

RUN apk add --no-cache libgcc

COPY --from=0 /build/target/x86_64-unknown-linux-musl/release/gptcli /usr/local/bin/gptcli

ENTRYPOINT ["/usr/local/bin/gptcli"]
