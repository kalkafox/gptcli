FROM kalka/docker-cargo:main

RUN apk add --no-cache build-base pkgconfig openssl-dev

WORKDIR /build

COPY . .

RUN cargo build --release

FROM alpine

RUN apk add --no-cache libgcc

COPY --from=0 /build/target/x86_64-unknown-linux-musl/release/gptcli /usr/local/bin/gptcli

ENTRYPOINT ["/usr/local/bin/gptcli"]
